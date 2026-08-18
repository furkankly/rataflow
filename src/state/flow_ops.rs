//! Object-safe trait exposing non-generic [`Flow`] methods for `dyn` dispatch.
//!
//! [`FlowOps`] enables type-erased access to `Flow<N, E>` — useful when you
//! need `&mut dyn FlowOps` or `Box<dyn FlowOps>` to call viewport, selection, and
//! graph mutation methods without knowing the concrete `N`/`E` content types.
//!
//! All existing inherent methods on `Flow` remain unchanged. The blanket impl
//! delegates each trait method to the corresponding inherent method. Users who call
//! `Flow` methods directly never need to import this trait.
//!
//! ```no_run
//! # use rataflow::{Flow, FlowOps};
//! fn zoom_and_fit(ops: &mut dyn FlowOps) {
//!     ops.zoom_to(1.0);
//!     ops.request_fit_view();
//! }
//!
//! // Works with any Flow<N, E>:
//! # let mut flow: Flow = Flow::new();
//! zoom_and_fit(&mut flow);
//! ```

use std::collections::HashMap;
use std::time::Duration;

use ratatui::layout::Rect;

use crate::actions::{ControlsAction, EventResponse, FlowAction};
use crate::content::{EdgeContent, NodeContent};
use crate::input::{KeyEvent, MouseEvent};
use crate::state::Flow;
use crate::state::edge_preview::EdgePreview;
use crate::state::selection::Direction;
use crate::types::{Connection, Dimensions, FitViewOptions, HandleType, Position, Reconnectable};
use crate::ui::HandleStyle;

/// Object-safe trait exposing non-generic [`Flow`] methods for `dyn` dispatch.
///
/// `Flow<N, E>` is generic over content types, which prevents trait objects
/// (`Box<dyn ...>`) without knowing `N` and `E`. This trait extracts the ~70 methods
/// whose signatures don't mention `N` or `E` — event handling, viewport, selection,
/// graph mutation, and animation — into an object-safe interface.
///
/// A blanket impl covers all `Flow<N, E>`, so any `&mut Flow<N, E>` can
/// be used as `&mut dyn FlowOps`. You don't need to import this trait for direct
/// `Flow` usage — inherent methods take priority over trait methods.
///
/// Methods that need the generic types (`add_node`, `add_edge`, `node_content_mut`,
/// etc.) remain inherent on `Flow` and are not part of this trait.
///
/// # `impl Into<Position>` vs `Position`
///
/// Three inherent methods accept `impl Into<Position>` (`set_node_position`,
/// `move_node`, `center_on`). The trait versions take `Position` directly for
/// object safety. Both coexist without ambiguity — inherent methods take priority
/// for direct calls; trait methods are used through `dyn FlowOps`.
pub trait FlowOps {
    // ========== Event Handling ==========

    /// Handles a keyboard event with default bindings.
    fn handle_key_event(&mut self, event: KeyEvent) -> EventResponse;

    /// Handles a mouse event with default behavior.
    fn handle_mouse_event(&mut self, event: MouseEvent) -> EventResponse;

    /// Applies a semantic action.
    fn apply(&mut self, action: FlowAction) -> EventResponse;

    /// Applies a controls action (zoom, fit, lock).
    fn apply_controls_action(&mut self, action: ControlsAction) -> EventResponse;

    /// Handles a keyboard event with default controls bindings.
    fn handle_controls_key_event(&mut self, event: KeyEvent) -> EventResponse;

    /// Handles a browser wheel event by zooming, around the given cell (ratzilla /
    /// WebAssembly only). See [`Flow::handle_wheel`](crate::Flow::handle_wheel).
    #[cfg(feature = "ratzilla")]
    fn handle_wheel(
        &mut self,
        delta_y: f64,
        delta_mode: u32,
        column: u16,
        row: u16,
    ) -> EventResponse;

    // ========== Viewport ==========

    /// Pans the viewport by the given offset delta.
    ///
    /// Positive `dx` moves content right (reveals left), positive `dy` moves
    /// content down (reveals top). See [`Viewport::pan_by`](crate::Viewport::pan_by)
    /// for details.
    fn pan(&mut self, dx: f64, dy: f64);

    /// Zooms in by the default step factor.
    fn zoom_in(&mut self);

    /// Zooms out by the default step factor.
    fn zoom_out(&mut self);

    /// Zooms to a specific level.
    fn zoom_to(&mut self, zoom: f64);

    /// Zooms around a specific canvas position.
    fn zoom_around(&mut self, factor: f64, canvas_pos: Position);

    /// Resets the zoom to 1.0.
    fn reset_zoom(&mut self);

    /// Centers the viewport on selected nodes.
    fn center_on_selected(&mut self);

    /// Centers the viewport on a world position.
    fn center_on(&mut self, world_pos: Position);

    /// Requests a deferred fit-view with default options.
    fn request_fit_view(&mut self);

    /// Requests a deferred fit-view with the given options.
    fn request_fit_view_with_options(&mut self, options: FitViewOptions);

    /// Pans the minimum amount needed to make a node fully visible.
    fn ensure_node_visible(&mut self, node_id: &str);

    /// Returns the canvas size from the last render.
    fn canvas_size(&self) -> Dimensions;

    // ========== Selection ==========

    /// Clears all selection.
    fn clear_selection(&mut self);

    /// Selects every node.
    fn select_all_nodes(&mut self);

    /// Selects every edge.
    fn select_all_edges(&mut self);

    /// Returns true if any node is selected.
    fn has_selected_nodes(&self) -> bool;

    /// Returns true if any edge is selected.
    fn has_selected_edges(&self) -> bool;

    /// Selects a node by ID, clearing other selection.
    fn select_node(&mut self, id: &str);

    /// Toggles a node's selection without clearing others.
    fn toggle_node_selection(&mut self, id: &str);

    /// Returns the ID of the first selected node.
    fn first_selected_node_id(&self) -> Option<String>;

    /// Selects the next node.
    fn select_next_node(&mut self);

    /// Selects the previous node.
    fn select_prev_node(&mut self);

    /// Selects the nearest node in the given spatial direction.
    fn select_node_in_direction(&mut self, direction: Direction);

    /// Selects an edge by ID, clearing other selection.
    fn select_edge(&mut self, id: &str);

    /// Toggles an edge's selection without clearing others.
    fn toggle_edge_selection(&mut self, id: &str);

    /// Returns the ID of the first selected edge.
    fn first_selected_edge_id(&self) -> Option<String>;

    /// Selects the next edge.
    fn select_next_edge(&mut self);

    /// Selects the previous edge.
    fn select_prev_edge(&mut self);

    // ========== Graph Mutation (non-generic) ==========

    /// Sets the position of a node.
    fn set_node_position(&mut self, id: &str, position: Position);

    /// Moves a node by a relative delta.
    fn move_node(&mut self, id: &str, delta: Position);

    /// Sets the dimensions of a node.
    fn set_node_dimensions(&mut self, id: &str, width: f64, height: f64);

    /// Sets the z-index of a node.
    fn set_node_z_index(&mut self, id: &str, z_index: i32);

    /// Sets the hidden state of a node.
    fn set_node_hidden(&mut self, id: &str, hidden: bool);

    /// Sets whether a node can be selected.
    fn set_node_selectable(&mut self, id: &str, selectable: bool);

    /// Sets whether a node can be deleted.
    fn set_node_deletable(&mut self, id: &str, deletable: bool);

    /// Sets whether a node can be dragged.
    fn set_node_draggable(&mut self, id: &str, draggable: bool);

    /// Sets whether a node's handles can participate in connections.
    fn set_node_connectable(&mut self, id: &str, connectable: bool);

    /// Sets whether a node blocks content behind it.
    fn set_node_opaque(&mut self, id: &str, opaque: bool);

    /// Sets the handle style for all handles on a node.
    fn set_handle_styles(&mut self, id: &str, style: Option<HandleStyle>);

    /// Sets the handle style for a single handle on a node.
    fn set_handle_style(&mut self, node_id: &str, handle_id: &str, style: Option<HandleStyle>);

    /// Sets the handle disabled style for all handles on a node.
    fn set_handle_disabled_styles(&mut self, id: &str, style: Option<HandleStyle>);

    /// Sets the handle disabled style for a single handle on a node.
    fn set_handle_disabled_style(
        &mut self,
        node_id: &str,
        handle_id: &str,
        style: Option<HandleStyle>,
    );

    /// Sets the hidden state for all handles on a node.
    fn set_handles_hidden(&mut self, id: &str, hidden: bool);

    /// Sets the hidden state for a single handle on a node.
    fn set_handle_hidden(&mut self, node_id: &str, handle_id: &str, hidden: bool);

    /// Sets the hidden state of an edge.
    fn set_edge_hidden(&mut self, id: &str, hidden: bool);

    /// Sets the label of an edge.
    fn set_edge_label(&mut self, id: &str, label: Option<String>);

    /// Sets whether an edge can be selected.
    fn set_edge_selectable(&mut self, id: &str, selectable: bool);

    /// Sets whether an edge can be deleted.
    fn set_edge_deletable(&mut self, id: &str, deletable: bool);

    /// Sets whether an edge is animated.
    fn set_edge_animated(&mut self, id: &str, animated: bool);

    /// Sets whether an edge can be reconnected.
    fn set_edge_reconnectable(&mut self, id: &str, reconnectable: Reconnectable);

    /// Sets node positions from a map.
    fn set_node_positions(&mut self, positions: HashMap<String, Position>);

    /// Clears all nodes and edges.
    fn clear(&mut self);

    /// Starts an edge preview from a source handle.
    fn start_edge_preview(
        &mut self,
        from_node_id: &str,
        from_handle_id: Option<&str>,
        from_handle_type: HandleType,
    ) -> bool;

    /// Points the edge preview at a specific handle on a target node.
    fn preview_to_handle(&mut self, to_node_id: &str, to_handle_id: Option<&str>) -> bool;

    /// Points the edge preview at a target node.
    fn preview_to_node(&mut self, to_node_id: &str) -> bool;

    /// Cycles the to-handle of the edge preview.
    fn cycle_to_handle(&mut self, forward: bool) -> bool;

    /// Cycles the from-handle of the edge preview.
    fn cycle_from_handle(&mut self, forward: bool) -> bool;

    /// Completes the edge preview and returns a normalized Connection if valid.
    fn complete_edge_preview(&mut self) -> Option<Connection>;

    /// Returns the edge preview state, or `None` if no preview is active.
    fn edge_preview(&self) -> Option<&EdgePreview>;

    /// Clears the edge preview.
    fn clear_edge_preview(&mut self);

    // ========== Animation & Auto-Pan ==========

    /// Advances the animation clock.
    fn tick_animation(&mut self, elapsed: Duration);

    /// Advances auto-pan state by the given elapsed time.
    fn tick_auto_pan(&mut self, elapsed: Duration) -> EventResponse;

    // ========== State Queries ==========

    /// Returns the canvas area from the last render.
    fn canvas_area(&self) -> Rect;

    /// Returns true if a drag operation is in progress.
    fn is_dragging(&self) -> bool;

    /// Toggles the interaction lock.
    fn toggle_lock(&mut self);

    /// Clears the connection validator.
    fn clear_connection_validator(&mut self);
}

impl<N: NodeContent, E: EdgeContent> FlowOps for Flow<N, E> {
    fn handle_key_event(&mut self, event: KeyEvent) -> EventResponse {
        self.handle_key_event(event)
    }

    fn handle_mouse_event(&mut self, event: MouseEvent) -> EventResponse {
        self.handle_mouse_event(event)
    }

    fn apply(&mut self, action: FlowAction) -> EventResponse {
        self.apply(action)
    }

    fn apply_controls_action(&mut self, action: ControlsAction) -> EventResponse {
        self.apply_controls_action(action)
    }

    fn handle_controls_key_event(&mut self, event: KeyEvent) -> EventResponse {
        self.handle_controls_key_event(event)
    }

    #[cfg(feature = "ratzilla")]
    fn handle_wheel(
        &mut self,
        delta_y: f64,
        delta_mode: u32,
        column: u16,
        row: u16,
    ) -> EventResponse {
        self.handle_wheel(delta_y, delta_mode, column, row)
    }

    fn pan(&mut self, dx: f64, dy: f64) {
        self.pan(dx, dy)
    }

    fn zoom_in(&mut self) {
        self.zoom_in()
    }

    fn zoom_out(&mut self) {
        self.zoom_out()
    }

    fn zoom_to(&mut self, zoom: f64) {
        self.zoom_to(zoom)
    }

    fn zoom_around(&mut self, factor: f64, canvas_pos: Position) {
        self.zoom_around(factor, canvas_pos)
    }

    fn reset_zoom(&mut self) {
        self.reset_zoom()
    }

    fn center_on_selected(&mut self) {
        self.center_on_selected()
    }

    fn center_on(&mut self, world_pos: Position) {
        self.center_on(world_pos)
    }

    fn request_fit_view(&mut self) {
        self.request_fit_view()
    }

    fn request_fit_view_with_options(&mut self, options: FitViewOptions) {
        self.request_fit_view_with_options(options)
    }

    fn ensure_node_visible(&mut self, node_id: &str) {
        self.ensure_node_visible(node_id)
    }

    fn canvas_size(&self) -> Dimensions {
        self.canvas_size()
    }

    fn clear_selection(&mut self) {
        self.clear_selection()
    }

    fn select_all_nodes(&mut self) {
        self.select_all_nodes()
    }

    fn select_all_edges(&mut self) {
        self.select_all_edges()
    }

    fn has_selected_nodes(&self) -> bool {
        self.has_selected_nodes()
    }

    fn has_selected_edges(&self) -> bool {
        self.has_selected_edges()
    }

    fn select_node(&mut self, id: &str) {
        self.select_node(id)
    }

    fn toggle_node_selection(&mut self, id: &str) {
        self.toggle_node_selection(id)
    }

    fn first_selected_node_id(&self) -> Option<String> {
        self.first_selected_node_id()
    }

    fn select_next_node(&mut self) {
        self.select_next_node()
    }

    fn select_prev_node(&mut self) {
        self.select_prev_node()
    }

    fn select_node_in_direction(&mut self, direction: Direction) {
        self.select_node_in_direction(direction)
    }

    fn select_edge(&mut self, id: &str) {
        self.select_edge(id)
    }

    fn toggle_edge_selection(&mut self, id: &str) {
        self.toggle_edge_selection(id)
    }

    fn first_selected_edge_id(&self) -> Option<String> {
        self.first_selected_edge_id()
    }

    fn select_next_edge(&mut self) {
        self.select_next_edge()
    }

    fn select_prev_edge(&mut self) {
        self.select_prev_edge()
    }

    fn set_node_position(&mut self, id: &str, position: Position) {
        self.set_node_position(id, position)
    }

    fn move_node(&mut self, id: &str, delta: Position) {
        self.move_node(id, delta)
    }

    fn set_node_dimensions(&mut self, id: &str, width: f64, height: f64) {
        self.set_node_dimensions(id, width, height)
    }

    fn set_node_z_index(&mut self, id: &str, z_index: i32) {
        self.set_node_z_index(id, z_index)
    }

    fn set_node_hidden(&mut self, id: &str, hidden: bool) {
        self.set_node_hidden(id, hidden)
    }

    fn set_node_selectable(&mut self, id: &str, selectable: bool) {
        self.set_node_selectable(id, selectable)
    }

    fn set_node_deletable(&mut self, id: &str, deletable: bool) {
        self.set_node_deletable(id, deletable)
    }

    fn set_node_draggable(&mut self, id: &str, draggable: bool) {
        self.set_node_draggable(id, draggable)
    }

    fn set_node_connectable(&mut self, id: &str, connectable: bool) {
        self.set_node_connectable(id, connectable)
    }

    fn set_node_opaque(&mut self, id: &str, opaque: bool) {
        self.set_node_opaque(id, opaque)
    }

    fn set_handle_styles(&mut self, id: &str, style: Option<HandleStyle>) {
        self.set_handle_styles(id, style)
    }

    fn set_handle_style(&mut self, node_id: &str, handle_id: &str, style: Option<HandleStyle>) {
        self.set_handle_style(node_id, handle_id, style)
    }

    fn set_handle_disabled_styles(&mut self, id: &str, style: Option<HandleStyle>) {
        self.set_handle_disabled_styles(id, style)
    }

    fn set_handle_disabled_style(
        &mut self,
        node_id: &str,
        handle_id: &str,
        style: Option<HandleStyle>,
    ) {
        self.set_handle_disabled_style(node_id, handle_id, style)
    }

    fn set_handles_hidden(&mut self, id: &str, hidden: bool) {
        self.set_handles_hidden(id, hidden)
    }

    fn set_handle_hidden(&mut self, node_id: &str, handle_id: &str, hidden: bool) {
        self.set_handle_hidden(node_id, handle_id, hidden)
    }

    fn set_edge_hidden(&mut self, id: &str, hidden: bool) {
        self.set_edge_hidden(id, hidden)
    }

    fn set_edge_label(&mut self, id: &str, label: Option<String>) {
        self.set_edge_label(id, label)
    }

    fn set_edge_selectable(&mut self, id: &str, selectable: bool) {
        self.set_edge_selectable(id, selectable)
    }

    fn set_edge_deletable(&mut self, id: &str, deletable: bool) {
        self.set_edge_deletable(id, deletable)
    }

    fn set_edge_animated(&mut self, id: &str, animated: bool) {
        self.set_edge_animated(id, animated)
    }

    fn set_edge_reconnectable(&mut self, id: &str, reconnectable: Reconnectable) {
        self.set_edge_reconnectable(id, reconnectable)
    }

    fn set_node_positions(&mut self, positions: HashMap<String, Position>) {
        self.set_node_positions(positions)
    }

    fn clear(&mut self) {
        self.clear()
    }

    fn start_edge_preview(
        &mut self,
        from_node_id: &str,
        from_handle_id: Option<&str>,
        from_handle_type: HandleType,
    ) -> bool {
        self.start_edge_preview(from_node_id, from_handle_id, from_handle_type)
    }

    fn preview_to_handle(&mut self, to_node_id: &str, to_handle_id: Option<&str>) -> bool {
        self.preview_to_handle(to_node_id, to_handle_id)
    }

    fn preview_to_node(&mut self, to_node_id: &str) -> bool {
        self.preview_to_node(to_node_id)
    }

    fn cycle_to_handle(&mut self, forward: bool) -> bool {
        self.cycle_to_handle(forward)
    }

    fn cycle_from_handle(&mut self, forward: bool) -> bool {
        self.cycle_from_handle(forward)
    }

    fn complete_edge_preview(&mut self) -> Option<Connection> {
        self.complete_edge_preview()
    }

    fn edge_preview(&self) -> Option<&EdgePreview> {
        self.edge_preview()
    }

    fn clear_edge_preview(&mut self) {
        self.clear_edge_preview()
    }

    fn tick_animation(&mut self, elapsed: Duration) {
        self.tick_animation(elapsed)
    }

    fn tick_auto_pan(&mut self, elapsed: Duration) -> EventResponse {
        self.tick_auto_pan(elapsed)
    }

    fn canvas_area(&self) -> Rect {
        self.canvas_area()
    }

    fn is_dragging(&self) -> bool {
        self.is_dragging()
    }

    fn toggle_lock(&mut self) {
        self.toggle_lock()
    }

    fn clear_connection_validator(&mut self) {
        self.clear_connection_validator()
    }
}

#[cfg(test)]
mod tests {
    use super::FlowOps;
    use crate::state::Flow;
    use crate::types::{Edge, Node, Position};
    use crate::ui::{StepEdge, TextContent};

    fn abc() -> Flow<TextContent, StepEdge> {
        let nodes = vec![
            Node::new("a", (0.0, 0.0), (10.0, 5.0), TextContent::from("A")),
            Node::new("b", (20.0, 0.0), (10.0, 5.0), TextContent::from("B")),
        ];
        let edges: Vec<Edge<StepEdge>> = vec![Edge::new("e1", "a", "b")];
        Flow::with_graph(nodes, edges).unwrap()
    }

    /// Each blanket impl forwards to the inherent method of the same name, relying on
    /// inherent-method priority. If that ever stopped resolving, the call would recurse
    /// into itself — a stack overflow no compile check catches. Exercising them through
    /// `dyn FlowOps` is what proves the forwarding lands on the inherent method.
    #[test]
    fn dyn_forwarding_reaches_the_inherent_methods() {
        let mut flow = abc();
        let ops: &mut dyn FlowOps = &mut flow;

        ops.select_all_nodes();
        ops.select_all_edges();

        let mut positions = std::collections::HashMap::new();
        positions.insert("a".to_string(), Position::new(7.0, 8.0));
        ops.set_node_positions(positions);

        assert_eq!(flow.selected_nodes().count(), 2);
        assert_eq!(flow.selected_edges().count(), 1);
        assert_eq!(flow.node("a").unwrap().position, Position::new(7.0, 8.0));
    }
}
