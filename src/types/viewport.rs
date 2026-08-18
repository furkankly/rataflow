//! Viewport types for panning and coordinate transformation.
//!
//! The viewport represents the visible area of the graph. It supports:
//! - Panning (translating the view)
//! - Optional zooming (scaling the view)
//! - Coordinate transformations between world and canvas space
//!
//! # Coordinate Spaces
//!
//! The viewport transforms between two coordinate spaces:
//! - **World**: Logical coordinates where nodes and edges exist
//! - **Canvas**: After viewport transform (pan/zoom applied), relative to canvas origin
//!
//! The canvas offset (for terminal buffer positioning) is handled separately
//! by [`RenderContext`](crate::state::RenderContext).

use super::geometry::{CoordinateExtent, Dimensions, Position, Rect};

/// The visible area and coordinate transformation of the flow graph.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Viewport {
    /// Horizontal pan offset (positive = content moves right).
    #[cfg_attr(feature = "serde", serde(default))]
    pub x: f64,
    /// Vertical pan offset (positive = content moves down).
    #[cfg_attr(feature = "serde", serde(default))]
    pub y: f64,
    /// Zoom level (1.0 = no zoom, >1 = zoomed in, <1 = zoomed out).
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::f64_one"))]
    pub zoom: f64,
}

impl Viewport {
    /// Creates a new viewport with the given offset and zoom.
    pub const fn new(x: f64, y: f64, zoom: f64) -> Self {
        Self { x, y, zoom }
    }

    /// Creates a viewport at the origin with 1× zoom (identity).
    pub const fn default_view() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }

    /// Returns the pan offset as a position.
    pub fn offset(&self) -> Position {
        Position::new(self.x, self.y)
    }

    /// Sets the pan offset.
    pub fn set_offset(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }

    /// Pans by the given offset delta.
    ///
    /// Positive `dx` moves content right on screen (reveals the left side of the
    /// graph). Matches xyflow's `panBy` convention: the delta is added directly
    /// to the viewport offset, not interpreted as a camera direction.
    pub fn pan_by(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }

    // ========================================================================
    // World <-> Canvas coordinate transformations
    // ========================================================================

    /// Transforms a position from world space to canvas space.
    ///
    /// Applies zoom and pan: `canvas = world * zoom + pan`
    pub fn world_to_canvas(&self, world_pos: Position) -> Position {
        Position::new(
            world_pos.x * self.zoom + self.x,
            world_pos.y * self.zoom + self.y,
        )
    }

    /// Transforms a position from canvas space to world space.
    ///
    /// Inverse of [`Self::world_to_canvas`]: `world = (canvas - pan) / zoom`
    pub fn canvas_to_world(&self, canvas_pos: Position) -> Position {
        Position::new(
            (canvas_pos.x - self.x) / self.zoom,
            (canvas_pos.y - self.y) / self.zoom,
        )
    }

    /// Transforms a rectangle from world space to canvas space.
    pub fn world_to_canvas_rect(&self, rect: Rect) -> Rect {
        let position = self.world_to_canvas(rect.position);
        let dims = Dimensions::new(
            rect.dimensions.width * self.zoom,
            rect.dimensions.height * self.zoom,
        );
        Rect::new(position, dims)
    }

    /// Clamps the viewport within the given extent.
    pub fn clamp(&mut self, extent: &CoordinateExtent) {
        self.x = self.x.clamp(extent.min.x, extent.max.x);
        self.y = self.y.clamp(extent.min.y, extent.max.y);
    }

    /// Sets the zoom level, clamped to valid range.
    pub fn set_zoom(&mut self, zoom: f64, min: f64, max: f64) {
        self.zoom = zoom.clamp(min, max);
    }

    /// Zooms by a factor around a canvas position.
    ///
    /// The point under the cursor stays fixed while the zoom changes.
    /// Use `RenderContext::terminal_to_canvas` to convert terminal coordinates
    /// to canvas coordinates first.
    pub fn zoom_around(&mut self, factor: f64, canvas_pos: Position, min: f64, max: f64) {
        let old_zoom = self.zoom;
        let new_zoom = (self.zoom * factor).clamp(min, max);

        if (new_zoom - old_zoom).abs() < f64::EPSILON {
            return;
        }

        // Get world point under cursor at current zoom
        let world_point = self.canvas_to_world(canvas_pos);

        // Apply new zoom
        self.zoom = new_zoom;

        // Adjust pan so the world point stays at the same canvas position
        // canvas = world * zoom + pan
        // So: pan = canvas - world * zoom
        self.x = canvas_pos.x - world_point.x * self.zoom;
        self.y = canvas_pos.y - world_point.y * self.zoom;
    }

    /// Centers the viewport on a point in world space.
    pub fn center_on(&mut self, world_point: Position, canvas_size: Dimensions) {
        self.x = canvas_size.width / 2.0 - world_point.x * self.zoom;
        self.y = canvas_size.height / 2.0 - world_point.y * self.zoom;
    }

    /// Fits the viewport to show all the given bounds (in world space).
    pub fn fit_bounds(
        &mut self,
        bounds: Rect,
        canvas_size: Dimensions,
        padding: f64,
        min_zoom: f64,
        max_zoom: f64,
    ) -> bool {
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return false;
        }

        let available_width = canvas_size.width - padding * 2.0;
        let available_height = canvas_size.height - padding * 2.0;

        if available_width <= 0.0 || available_height <= 0.0 {
            return false;
        }

        let zoom_x = available_width / bounds.width();
        let zoom_y = available_height / bounds.height();
        let zoom = zoom_x.min(zoom_y).clamp(min_zoom, max_zoom);

        self.zoom = zoom;

        // Center the bounds in the viewport
        let center = bounds.center();
        self.center_on(center, canvas_size);
        true
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::default_view()
    }
}

/// Options for viewport fitting operations.
///
/// The `min_zoom` and `max_zoom` fields are optional overrides. If not set,
/// the operation will use the `Flow`'s `min_zoom`/`max_zoom` values.
#[derive(Debug, Clone, Default)]
pub struct FitViewOptions {
    /// Padding around the content (default: 0.0).
    pub padding: f64,
    /// Whether to include hidden nodes (default: false).
    pub include_hidden: bool,
    /// Minimum zoom level override (uses flow's min_zoom if None).
    pub min_zoom: Option<f64>,
    /// Maximum zoom level override (uses flow's max_zoom if None).
    pub max_zoom: Option<f64>,
    /// Specific node IDs to fit (if empty, fits all).
    pub nodes: Vec<String>,
}

impl FitViewOptions {
    /// Sets the padding.
    pub fn with_padding(mut self, padding: f64) -> Self {
        self.padding = padding;
        self
    }

    /// Sets whether to include hidden nodes.
    pub fn with_include_hidden(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }

    /// Sets the minimum zoom level override.
    pub fn with_min_zoom(mut self, min: f64) -> Self {
        self.min_zoom = Some(min);
        self
    }

    /// Sets the maximum zoom level override.
    pub fn with_max_zoom(mut self, max: f64) -> Self {
        self.max_zoom = Some(max);
        self
    }

    /// Sets both zoom level overrides.
    pub fn with_zoom_range(mut self, min: f64, max: f64) -> Self {
        self.min_zoom = Some(min);
        self.max_zoom = Some(max);
        self
    }

    /// Sets specific nodes to fit.
    pub fn with_nodes(mut self, nodes: Vec<String>) -> Self {
        self.nodes = nodes;
        self
    }
}

/// Pan mode for scroll-based panning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PanMode {
    /// Free panning in all directions.
    #[default]
    Free,
    /// Horizontal panning only.
    Horizontal,
    /// Vertical panning only.
    Vertical,
}

impl PanMode {
    /// Applies the pan mode constraints to a delta.
    pub fn constrain(&self, dx: f64, dy: f64) -> (f64, f64) {
        match self {
            PanMode::Free => (dx, dy),
            PanMode::Horizontal => (dx, 0.0),
            PanMode::Vertical => (0.0, dy),
        }
    }
}

/// What the viewport does when keyboard navigation changes the selection.
///
/// The `Select*` actions (arrow / Tab spatial and sequential nav) apply this
/// *after* moving the selection — so the selection change and the
/// `SelectionChanged` event fire regardless of the policy; only the camera
/// response differs. Point-and-click selection and the explicit
/// `CenterOnSelected` action are unaffected.
///
/// Set it with [`Flow::with_selection_reveal`](crate::Flow::with_selection_reveal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SelectionReveal {
    /// Leave the viewport untouched — the consumer drives the camera itself
    /// (e.g. an eased glide of its own). Use this to avoid the built-in reveal
    /// fighting a custom camera.
    None,
    /// Pan the minimum amount to bring the newly-selected node fully on-screen
    /// (with a 1-cell margin), doing nothing if it is already visible. Default.
    #[default]
    EnsureVisible,
    /// Center the viewport on the newly-selected node.
    Center,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_to_canvas_no_transform() {
        let vp = Viewport::default();
        let canvas = vp.world_to_canvas(Position::new(10.0, 20.0));

        assert_eq!(canvas.x, 10.0);
        assert_eq!(canvas.y, 20.0);
    }

    #[test]
    fn test_world_to_canvas_with_pan() {
        let vp = Viewport::new(10.0, 20.0, 1.0);
        let canvas = vp.world_to_canvas(Position::new(5.0, 5.0));

        // canvas = world * zoom + pan = 5 * 1 + 10 = 15
        assert_eq!(canvas.x, 15.0);
        assert_eq!(canvas.y, 25.0);
    }

    #[test]
    fn test_world_to_canvas_with_zoom() {
        let vp = Viewport::new(0.0, 0.0, 2.0);
        let canvas = vp.world_to_canvas(Position::new(10.0, 20.0));

        // canvas = world * zoom = 10 * 2 = 20
        assert_eq!(canvas.x, 20.0);
        assert_eq!(canvas.y, 40.0);
    }

    #[test]
    fn test_canvas_to_world_no_transform() {
        let vp = Viewport::default();
        let world = vp.canvas_to_world(Position::new(10.0, 20.0));

        assert_eq!(world.x, 10.0);
        assert_eq!(world.y, 20.0);
    }

    #[test]
    fn test_canvas_to_world_with_pan() {
        let vp = Viewport::new(10.0, 20.0, 1.0);
        let world = vp.canvas_to_world(Position::new(30.0, 40.0));

        // world = (canvas - pan) / zoom = (30 - 10) / 1 = 20
        assert_eq!(world.x, 20.0);
        assert_eq!(world.y, 20.0);
    }

    #[test]
    fn test_canvas_to_world_with_zoom() {
        let vp = Viewport::new(0.0, 0.0, 2.0);
        let world = vp.canvas_to_world(Position::new(20.0, 40.0));

        // world = canvas / zoom = 20 / 2 = 10
        assert_eq!(world.x, 10.0);
        assert_eq!(world.y, 20.0);
    }

    #[test]
    fn test_world_canvas_roundtrip() {
        let vp = Viewport::new(5.0, 10.0, 1.5);
        let original = Position::new(100.0, 200.0);

        let canvas = vp.world_to_canvas(original);
        let back = vp.canvas_to_world(canvas);

        assert!((back.x - original.x).abs() < f64::EPSILON);
        assert!((back.y - original.y).abs() < f64::EPSILON);
    }

    #[test]
    fn test_center_on() {
        let mut vp = Viewport::default();
        let canvas_size = Dimensions::new(100.0, 80.0);
        let point = Position::new(50.0, 40.0);

        vp.center_on(point, canvas_size);

        // The point should now be at the center of the canvas
        let canvas_pos = vp.world_to_canvas(point);
        assert_eq!(canvas_pos.x, 50.0); // canvas width / 2
        assert_eq!(canvas_pos.y, 40.0); // canvas height / 2
    }

    #[test]
    fn test_zoom_around() {
        let mut vp = Viewport::new(0.0, 0.0, 1.0);
        let canvas_pos = Position::new(50.0, 50.0);

        // Get world point at this canvas position before zoom
        let world_before = vp.canvas_to_world(canvas_pos);

        // Zoom in 2x around this point
        vp.zoom_around(2.0, canvas_pos, 0.1, 10.0);

        // The world point should still map to the same canvas position
        let canvas_after = vp.world_to_canvas(world_before);
        assert!((canvas_after.x - canvas_pos.x).abs() < f64::EPSILON);
        assert!((canvas_after.y - canvas_pos.y).abs() < f64::EPSILON);
    }
}
