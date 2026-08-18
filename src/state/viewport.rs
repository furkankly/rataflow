//! Viewport operations for Flow.

use super::Flow;
use crate::content::{EdgeContent, NodeContent};
use crate::types::{Dimensions, FitViewOptions, Position};

/// Default zoom factor for zoom_in/zoom_out.
const ZOOM_STEP: f64 = 1.2;

/// Default minimum zoom level.
pub(crate) const DEFAULT_MIN_ZOOM: f64 = 0.5;

/// Default maximum zoom level.
pub(crate) const DEFAULT_MAX_ZOOM: f64 = 2.0;

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    /// Pans the viewport by the given offset delta.
    ///
    /// Positive `dx` moves content right (reveals left), positive `dy` moves
    /// content down (reveals top). See [`Viewport::pan_by`](crate::Viewport::pan_by)
    /// for details.
    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.viewport.pan_by(dx, dy);
    }

    /// Zooms in by the default step factor.
    ///
    /// Zooms around the center of the canvas. Uses the flow's zoom limits.
    pub fn zoom_in(&mut self) {
        let center = self.canvas_center();
        self.viewport
            .zoom_around(ZOOM_STEP, center, self.min_zoom, self.max_zoom);
    }

    /// Zooms out by the default step factor.
    ///
    /// Zooms around the center of the canvas. Uses the flow's zoom limits.
    pub fn zoom_out(&mut self) {
        let center = self.canvas_center();
        self.viewport
            .zoom_around(1.0 / ZOOM_STEP, center, self.min_zoom, self.max_zoom);
    }

    /// Zooms to a specific level.
    ///
    /// Zooms around the center of the canvas. Uses the flow's zoom limits.
    pub fn zoom_to(&mut self, zoom: f64) {
        let center = self.canvas_center();
        let factor = zoom / self.viewport.zoom;
        self.viewport
            .zoom_around(factor, center, self.min_zoom, self.max_zoom);
    }

    /// Zooms around a specific canvas position.
    ///
    /// The point under the cursor stays fixed while the zoom changes.
    /// Uses the flow's zoom limits.
    pub fn zoom_around(&mut self, factor: f64, canvas_pos: Position) {
        self.viewport
            .zoom_around(factor, canvas_pos, self.min_zoom, self.max_zoom);
    }

    /// Resets the zoom to 1.0 (no zoom).
    pub fn reset_zoom(&mut self) {
        self.zoom_to(1.0);
    }

    /// Centers the viewport on the selected node(s).
    ///
    /// With a single selected node, centers on that node.
    /// With multiple selected nodes, centers on the bounding box of all selected nodes.
    ///
    /// **Important:** This method uses the canvas size from the last render.
    /// You must render at least once before calling this method.
    ///
    /// Does nothing if no node is selected.
    pub fn center_on_selected(&mut self) {
        let bounds = self
            .nodes
            .iter()
            .filter(|n| n.node.selected)
            .map(|n| n.bounds())
            .reduce(|a, b| a.union(&b));

        if let Some(bounds) = bounds {
            let center = bounds.center();
            self.viewport
                .center_on(center, self.render_context.canvas_size());
        }
    }

    /// Centers the viewport on a world position.
    ///
    /// **Important:** This method uses the canvas size from the last render.
    /// You must render at least once before calling this method.
    pub fn center_on(&mut self, world_pos: impl Into<Position>) {
        self.viewport
            .center_on(world_pos.into(), self.render_context.canvas_size());
    }

    /// Fits the viewport with the given options immediately.
    ///
    /// This is the internal implementation used by `apply_pending_fit_view()`.
    /// Returns `true` if the viewport was adjusted.
    pub(crate) fn fit_view_with_options(&mut self, options: FitViewOptions) -> bool {
        if self.nodes.is_empty() {
            return false;
        }

        // Filter nodes based on options
        let bounds = self
            .nodes
            .iter()
            .filter(|n| {
                // Filter by hidden status
                if !options.include_hidden && n.node.hidden {
                    return false;
                }
                // Filter by node IDs if specified
                if !options.nodes.is_empty() && !options.nodes.iter().any(|s| s == n.id()) {
                    return false;
                }
                true
            })
            .map(|n| n.bounds())
            .reduce(|a, b| a.union(&b));

        if let Some(bounds) = bounds {
            let min_zoom = options.min_zoom.unwrap_or(self.min_zoom);
            let max_zoom = options.max_zoom.unwrap_or(self.max_zoom);
            self.viewport.fit_bounds(
                bounds,
                self.render_context.canvas_size(),
                options.padding,
                min_zoom,
                max_zoom,
            )
        } else {
            false
        }
    }

    /// Requests a deferred fit-view with default options.
    ///
    /// The fit is applied during the next render,
    /// after the canvas size is known. If the canvas size changes between frames
    /// (e.g., terminal resize on startup), the fit is re-applied automatically
    /// until the size stabilizes.
    ///
    /// This replaces the render-then-fit pattern — call this before your event loop
    /// instead of rendering once and calling `fit_view_default()`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rataflow::{Edge, Flow, Node, StepEdge};
    /// # fn main() -> Result<(), rataflow::Error> {
    /// # let nodes = vec![Node::from_text("a", (0.0, 0.0), "A")];
    /// # let edges: Vec<Edge<StepEdge>> = vec![];
    /// let mut flow = Flow::with_graph(nodes, edges)?;
    /// flow.request_fit_view();
    ///
    /// // Applied during the next render:
    /// //   loop {
    /// //       terminal.draw(|frame| frame.render_widget(&mut flow, area))?;
    /// //       // ... handle events ...
    /// //   }
    /// # Ok(())
    /// # }
    /// ```
    pub fn request_fit_view(&mut self) {
        self.pending_fit = Some(FitViewOptions::default());
        self.pending_fit_canvas = None;
    }

    /// Requests a deferred fit-view with the given options.
    ///
    /// See [`request_fit_view`](Self::request_fit_view) for details.
    pub fn request_fit_view_with_options(&mut self, options: FitViewOptions) {
        self.pending_fit = Some(options);
        self.pending_fit_canvas = None;
    }

    /// Applies the pending fit-view request.
    ///
    /// Called during `Flow::render()` after `set_canvas_area()`. Re-fits
    /// whenever the canvas size changes (e.g., terminal resize on startup), then
    /// clears once the size stabilizes between consecutive renders.
    pub(crate) fn apply_pending_fit_view(&mut self) {
        if let Some(ref options) = self.pending_fit {
            let cs = self.render_context.canvas_size();
            let current = (cs.width as u16, cs.height as u16);
            if self.pending_fit_canvas != Some(current) {
                self.fit_view_with_options(options.clone());
                self.pending_fit_canvas = Some(current);
            } else {
                self.pending_fit = None;
                self.pending_fit_canvas = None;
            }
        }
    }

    /// Returns the center of the canvas in canvas coordinates.
    fn canvas_center(&self) -> Position {
        let size = self.render_context.canvas_size();
        Position::new(size.width / 2.0, size.height / 2.0)
    }

    /// Returns the canvas size from the last render.
    pub fn canvas_size(&self) -> Dimensions {
        self.render_context.canvas_size()
    }

    /// Transforms a world-coordinate point to terminal coordinates.
    ///
    /// **Escape hatch** for app-drawn overlays — typically called right after
    /// rendering the flow in the same draw pass. Identical to
    /// [`EdgeRenderContext::world_to_terminal`](crate::EdgeRenderContext::world_to_terminal);
    /// exposed on `Flow` because app code has no render context. Coordinates may
    /// lie outside the canvas, so guard writes with [`is_in_bounds`](Self::is_in_bounds).
    pub fn world_to_terminal(&self, pos: Position) -> (i32, i32) {
        self.render_context.world_to_terminal(&self.viewport, pos)
    }

    /// Returns a node's terminal-space rectangle.
    ///
    /// **Escape hatch** for app-drawn overlays anchored to nodes (badges,
    /// tooltips, activity indicators) — typically called right after rendering
    /// the flow in the same draw pass. Returns `(left, top, right, bottom)`
    /// as i32 edges, unclipped — anchor math needs the true rect even when
    /// the node extends off-canvas. Returns `None` if no node has the given id.
    ///
    /// # Example
    ///
    /// ```
    /// # use ratatui::buffer::Buffer;
    /// # use ratatui::layout::Rect;
    /// # use rataflow::Flow;
    /// # let flow: Flow = Flow::new();
    /// # let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    /// // Draw a badge below a node, clipped per cell
    /// if let Some((left, _, _, bottom)) = flow.node_terminal_rect("a") {
    ///     for (i, ch) in "⚒ Bash".chars().enumerate() {
    ///         let (x, y) = (left + i as i32, bottom);
    ///         if flow.is_in_bounds(x, y) {
    ///             buf[(x as u16, y as u16)].set_char(ch);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn node_terminal_rect(&self, id: &str) -> Option<(i32, i32, i32, i32)> {
        let node = self.internal_node(id)?;
        Some(
            self.render_context
                .world_to_terminal_rect(&self.viewport, node.bounds()),
        )
    }

    /// Checks if terminal coordinates are within the drawable canvas area.
    ///
    /// **Escape hatch** companion to [`world_to_terminal`](Self::world_to_terminal)
    /// and [`node_terminal_rect`](Self::node_terminal_rect); identical to
    /// [`EdgeRenderContext::is_in_bounds`](crate::EdgeRenderContext::is_in_bounds).
    /// Always `false` before the first render (zero-sized canvas), so guarded
    /// overlay writes are safe no-ops.
    pub fn is_in_bounds(&self, x: i32, y: i32) -> bool {
        self.render_context.is_in_canvas(x, y)
    }

    /// Ensures the first selected node is visible, if any.
    pub(crate) fn ensure_selected_node_visible(&mut self) {
        if let Some(id) = self.first_selected_node_id() {
            self.ensure_node_visible(&id);
        }
    }

    /// Pans the minimum amount needed to make a node fully visible.
    ///
    /// Computes the node's bounding rect in canvas space and shifts the viewport
    /// just enough to bring any off-screen portion into view (with a 1-cell margin).
    /// Does nothing if the node is already fully visible or if the canvas size
    /// is unknown (no render yet).
    ///
    /// Unlike [`center_on_selected`](Self::center_on_selected), this only pans
    /// when necessary and by the minimum amount — no jarring jumps when the node
    /// is already visible.
    pub fn ensure_node_visible(&mut self, node_id: &str) {
        let canvas_size = self.render_context.canvas_size();
        if canvas_size.width == 0.0 || canvas_size.height == 0.0 {
            return;
        }

        let node_bounds = match self.internal_node(node_id) {
            Some(n) => n.bounds(),
            None => return,
        };

        let canvas_rect = self.viewport.world_to_canvas_rect(node_bounds);
        let margin = 1.0;

        let mut dx = 0.0;
        let mut dy = 0.0;

        if canvas_rect.position.x < margin {
            dx = margin - canvas_rect.position.x;
        } else if canvas_rect.right() > canvas_size.width - margin {
            dx = (canvas_size.width - margin) - canvas_rect.right();
        }

        if canvas_rect.position.y < margin {
            dy = margin - canvas_rect.position.y;
        } else if canvas_rect.bottom() > canvas_size.height - margin {
            dy = (canvas_size.height - margin) - canvas_rect.bottom();
        }

        if dx != 0.0 || dy != 0.0 {
            self.viewport.x += dx;
            self.viewport.y += dy;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Flow;
    use crate::types::{Node, Position};
    use crate::ui::TextContent;
    use ratatui::layout::Rect;

    fn make_flow() -> Flow {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(0.0, 0.0),
                (10.0, 5.0),
                TextContent::from("A"),
            ),
            Node::new(
                "b",
                Position::new(100.0, 50.0),
                (10.0, 5.0),
                TextContent::from("B"),
            ),
        ];
        let mut flow = Flow::with_graph(nodes, vec![]).unwrap();
        flow.render_context.canvas_area = Rect::new(0, 0, 80, 24);
        flow
    }

    #[test]
    fn selection_reveal_policy_gates_the_keyboard_reveal() {
        use crate::{FlowAction, SelectionReveal};

        // `None`: the selection still moves, but the viewport is left untouched
        // (the consumer drives its own camera).
        let mut flow = make_flow();
        flow.select_node("a");
        flow.selection_reveal = SelectionReveal::None;
        let before = (flow.viewport.x, flow.viewport.y);
        flow.apply(FlowAction::SelectNext);
        assert_eq!(
            flow.first_selected_node_id().as_deref(),
            Some("b"),
            "selection still changes under None"
        );
        assert_eq!(
            (flow.viewport.x, flow.viewport.y),
            before,
            "None must not move the viewport"
        );

        // `EnsureVisible` (default): pans to reveal the off-screen node "b".
        let mut flow = make_flow();
        flow.select_node("a");
        let before = (flow.viewport.x, flow.viewport.y);
        flow.apply(FlowAction::SelectNext);
        assert_ne!(
            (flow.viewport.x, flow.viewport.y),
            before,
            "EnsureVisible reveals the off-screen node"
        );

        // `Center`: centers the newly-selected node on the canvas.
        let mut flow = make_flow();
        flow.select_node("a");
        flow.selection_reveal = SelectionReveal::Center;
        flow.apply(FlowAction::SelectNext);
        let rect = flow
            .viewport
            .world_to_canvas_rect(flow.internal_node("b").unwrap().bounds());
        let cx = (rect.position.x + rect.right()) / 2.0;
        let cy = (rect.position.y + rect.bottom()) / 2.0;
        assert!(
            (cx - 40.0).abs() < 2.0 && (cy - 12.0).abs() < 2.0,
            "Center puts the node at the canvas center (~40,12), got ({cx},{cy})"
        );
    }

    #[test]
    fn visible_node_no_pan() {
        let mut flow = make_flow();
        // Pan so node "a" is comfortably inside the canvas (not at the margin edge)
        flow.viewport.x = 5.0;
        flow.viewport.y = 5.0;
        let vp_before = (flow.viewport.x, flow.viewport.y);
        flow.ensure_node_visible("a");
        assert_eq!((flow.viewport.x, flow.viewport.y), vp_before);
    }

    #[test]
    fn offscreen_right_pans_minimum() {
        let mut flow = make_flow();
        // Node "b" at world (100, 50) is off-screen on an 80x24 canvas
        flow.ensure_node_visible("b");

        // Node's right edge (110 * zoom=1 + pan_x) should now be at canvas_width - margin
        let canvas_rect = flow
            .viewport
            .world_to_canvas_rect(flow.internal_node("b").unwrap().bounds());
        assert!(canvas_rect.right() <= 79.0 + f64::EPSILON);
        assert!(canvas_rect.position.x >= 1.0 - f64::EPSILON);
    }

    #[test]
    fn offscreen_left_pans_minimum() {
        let mut flow = make_flow();
        // Pan far right so node "a" at (0,0) is off the left edge
        flow.viewport.x = -50.0;
        flow.ensure_node_visible("a");

        let canvas_rect = flow
            .viewport
            .world_to_canvas_rect(flow.internal_node("a").unwrap().bounds());
        assert!(canvas_rect.position.x >= 1.0 - f64::EPSILON);
    }

    #[test]
    fn pans_only_needed_axis() {
        let mut flow = make_flow();
        // Node "a" is at world (0,0) with h=5. Place it vertically centered but off-screen left.
        flow.viewport.x = -200.0;
        flow.viewport.y = 5.0; // comfortably inside vertically
        let vp_before_y = flow.viewport.y;
        flow.ensure_node_visible("a");

        // X should have changed, Y should stay
        assert!(flow.viewport.x > -200.0);
        assert_eq!(flow.viewport.y, vp_before_y);
    }

    #[test]
    fn world_to_terminal_applies_viewport_and_canvas_offset() {
        let mut flow = make_flow();
        flow.render_context.set_canvas_area(Rect::new(5, 3, 80, 24));
        flow.viewport.x = 10.0;
        flow.viewport.y = 5.0;
        flow.viewport.zoom = 2.0;
        // world (4, 2) → canvas (4*2+10, 2*2+5) = (18, 9) → terminal (23, 12)
        assert_eq!(flow.world_to_terminal(Position::new(4.0, 2.0)), (23, 12));
    }

    #[test]
    fn node_terminal_rect_matches_node_geometry() {
        let mut flow = make_flow();
        flow.render_context.set_canvas_area(Rect::new(0, 0, 80, 24));
        // Node "a" at world (0,0) with dims (10,5), identity viewport
        assert_eq!(flow.node_terminal_rect("a"), Some((0, 0, 10, 5)));
        assert_eq!(flow.node_terminal_rect("missing"), None);
    }

    #[test]
    fn node_terminal_rect_is_unclipped_offscreen() {
        let mut flow = make_flow();
        flow.render_context.set_canvas_area(Rect::new(0, 0, 80, 24));
        flow.viewport.x = -20.0; // pan node "a" off the left edge
        let (left, _, right, _) = flow.node_terminal_rect("a").unwrap();
        // Unclipped: negative terminal coordinates preserved, width intact
        assert!(left < 0);
        assert_eq!(right - left, 10);
    }

    #[test]
    fn is_in_bounds_respects_canvas_area() {
        let mut flow = make_flow();
        flow.render_context.set_canvas_area(Rect::new(5, 3, 10, 5));
        assert!(flow.is_in_bounds(5, 3));
        assert!(flow.is_in_bounds(14, 7));
        assert!(!flow.is_in_bounds(15, 7));
        assert!(!flow.is_in_bounds(4, 3));
        assert!(!flow.is_in_bounds(-1, 4));
    }

    #[test]
    fn is_in_bounds_false_before_first_render() {
        // Fresh flow, no render yet (make_flow pre-seeds a canvas area)
        let flow: Flow = Flow::new();
        // Zero-sized canvas: nothing is in bounds, guarded writes are no-ops
        assert!(!flow.is_in_bounds(0, 0));
    }
}
