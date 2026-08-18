//! Render context for coordinate transformations during rendering.
//!
//! The render context combines layout information (canvas area) with viewport state
//! to provide coordinate transformations between world, canvas, and terminal spaces.
//!
//! # Coordinate Spaces
//!
//! ```text
//! world → (viewport: pan/zoom) → canvas → (offset) → terminal
//! ```
//!
//! - **World**: Logical coordinates where nodes and edges exist
//! - **Canvas**: After viewport transform (pan/zoom), relative to canvas origin (0,0)
//! - **Terminal**: Actual buffer positions in the terminal (always i32 — points as
//!   `(i32, i32)`, rects as `(i32, i32, i32, i32)` edges)

use ratatui::layout::Rect;

use crate::types::{Dimensions, Position, Rect as WorldRect, Viewport};

/// Context for rendering operations, providing coordinate transformations.
///
/// This struct is updated during each render with the current canvas area,
/// and provides helpers to transform between coordinate spaces.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct RenderContext {
    /// The canvas area within the terminal buffer.
    pub canvas_area: Rect,
}

impl RenderContext {
    /// Creates a new render context with the given canvas area (used in tests).
    #[allow(dead_code)]
    pub const fn new(canvas_area: Rect) -> Self {
        Self { canvas_area }
    }

    /// Updates the canvas area (called during render).
    pub fn set_canvas_area(&mut self, area: Rect) {
        self.canvas_area = area;
    }

    /// Returns the canvas dimensions.
    pub fn canvas_size(&self) -> Dimensions {
        Dimensions::new(
            self.canvas_area.width as f64,
            self.canvas_area.height as f64,
        )
    }

    // ========================================================================
    // Canvas <-> Terminal transformations (just offset)
    // ========================================================================

    /// Transforms canvas coordinates to terminal buffer coordinates.
    ///
    /// Uses floor() to ensure consistent discrete grid mapping. This is important
    /// for centering handles on odd-height nodes: a handle at y=3.5 should map to
    /// y=3 (the visual center of cells 2,3,4), not y=4 (which round() would give).
    pub fn canvas_to_terminal(&self, canvas_pos: Position) -> (i32, i32) {
        (
            canvas_pos.x.floor() as i32 + self.canvas_area.x as i32,
            canvas_pos.y.floor() as i32 + self.canvas_area.y as i32,
        )
    }

    /// Transforms terminal buffer coordinates to canvas coordinates.
    ///
    /// Adds 0.5 to map to the **center** of the terminal cell, compensating for
    /// `canvas_to_terminal`'s `floor()`. Without this, hit testing drifts
    /// downward/rightward at low zoom because `floor()` shifts rendered positions
    /// toward the cell's top-left while the inverse path assumes the top-left
    /// is the exact position. See INTERNALS.md § "Cell-Center Compensation".
    pub fn terminal_to_canvas(&self, column: u16, row: u16) -> Position {
        Position::new(
            (column as i32 - self.canvas_area.x as i32) as f64 + 0.5,
            (row as i32 - self.canvas_area.y as i32) as f64 + 0.5,
        )
    }

    // ========================================================================
    // World <-> Terminal transformations (composite, requires viewport)
    // ========================================================================

    /// Transforms world coordinates to terminal buffer coordinates.
    ///
    /// This applies the viewport transform (pan/zoom) then adds the canvas offset.
    pub fn world_to_terminal(&self, viewport: &Viewport, world_pos: Position) -> (i32, i32) {
        let canvas_pos = viewport.world_to_canvas(world_pos);
        self.canvas_to_terminal(canvas_pos)
    }

    /// Transforms world coordinates to terminal buffer coordinates, unrounded.
    ///
    /// The same mapping as [`world_to_terminal`](Self::world_to_terminal) minus the
    /// final `floor()`, which sub-cell renderers need: which of a braille cell's
    /// 2x4 dots a stroke lands on is decided entirely by the fraction `floor()`
    /// discards.
    pub fn world_to_terminal_f64(&self, viewport: &Viewport, world_pos: Position) -> (f64, f64) {
        let canvas_pos = viewport.world_to_canvas(world_pos);
        (
            canvas_pos.x + self.canvas_area.x as f64,
            canvas_pos.y + self.canvas_area.y as f64,
        )
    }

    /// Transforms terminal buffer coordinates to world coordinates.
    ///
    /// This subtracts the canvas offset then applies the inverse viewport transform.
    pub fn terminal_to_world(&self, viewport: &Viewport, column: u16, row: u16) -> Position {
        let canvas_pos = self.terminal_to_canvas(column, row);
        viewport.canvas_to_world(canvas_pos)
    }

    /// Transforms a rectangle from world coordinates to terminal coordinates.
    ///
    /// Returns `(left, top, right, bottom)` as i32 edges — consistent with
    /// [`world_to_terminal`] which returns `(i32, i32)` for points.
    ///
    /// Both corners go through `world_to_terminal`, so these edges agree with every
    /// other coordinate derived from the same world point: an edge terminating at a
    /// node's border, an abutting node's left edge. Dimensions are derived from the
    /// differences (`right - left`, `bottom - top`) rather than transformed
    /// separately, because `floor(pos) + floor(dim) ≠ floor(pos + dim)` and a
    /// separately snapped dimension puts the right edge a cell away from where the
    /// rest of the pipeline thinks it is.
    ///
    /// Tradeoff, inherent to corner snapping and not a defect to fix here: the
    /// derived extent varies by one cell with the subcell pan offset. A 1.5-cell-wide
    /// rect spans `[0, 1)` at pan 0.0, `[0, 2)` at 0.6, `[1, 2)` at 1.0. Snapping the
    /// dimension instead would give a stable size and inconsistent edges; we take the
    /// consistent edges.
    pub fn world_to_terminal_rect(
        &self,
        viewport: &Viewport,
        rect: WorldRect,
    ) -> (i32, i32, i32, i32) {
        let (left, top) = self.world_to_terminal(viewport, rect.position);
        let (right, bottom) =
            self.world_to_terminal(viewport, Position::new(rect.right(), rect.bottom()));
        (left, top, right, bottom)
    }

    // ========================================================================
    // Bounds checking
    // ========================================================================

    /// Checks if terminal coordinates are within the canvas bounds.
    pub fn is_in_canvas(&self, x: i32, y: i32) -> bool {
        x >= self.canvas_area.x as i32
            && x < (self.canvas_area.x + self.canvas_area.width) as i32
            && y >= self.canvas_area.y as i32
            && y < (self.canvas_area.y + self.canvas_area.height) as i32
    }

    /// Returns the visible area in world coordinates.
    ///
    /// Used for early culling of edges and nodes before coordinate transformation.
    /// Instead of transforming every element to terminal coordinates and then clipping,
    /// check if an element's world bounds intersect this visible area.
    pub fn visible_world_area(&self, viewport: &Viewport) -> WorldRect {
        // Top-left world coordinate visible at canvas position (0, 0)
        let top_left = viewport.canvas_to_world(Position::new(0.0, 0.0));
        let dims = Dimensions::new(
            self.canvas_area.width as f64 / viewport.zoom,
            self.canvas_area.height as f64 / viewport.zoom,
        );
        WorldRect::new(top_left, dims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(x: u16, y: u16, w: u16, h: u16) -> RenderContext {
        RenderContext::new(Rect::new(x, y, w, h))
    }

    #[test]
    fn canvas_to_terminal_floors_fractional() {
        let rc = ctx(5, 3, 80, 24);
        // 3.9 floors to 3, not rounds to 4 — critical for handle centering on
        // odd-height nodes (a handle at y=3.5 should map to y=3, the visual center)
        assert_eq!(rc.canvas_to_terminal(Position::new(3.9, 3.5)), (8, 6));
    }

    #[test]
    fn canvas_to_terminal_negative_result() {
        let rc = ctx(5, 3, 80, 24);
        // Off-screen elements produce negative terminal positions — the reason
        // terminal-space uses i32, not u16
        assert_eq!(rc.canvas_to_terminal(Position::new(-10.0, -5.0)), (-5, -2));
    }

    #[test]
    fn terminal_to_canvas_cell_center_compensation() {
        let rc = ctx(5, 3, 80, 24);
        let pos = rc.terminal_to_canvas(15, 10);
        // +0.5 maps to cell center, compensating for canvas_to_terminal's floor().
        // Without this, hit testing drifts downward/rightward at low zoom.
        assert!((pos.x - 10.5).abs() < f64::EPSILON);
        assert!((pos.y - 7.5).abs() < f64::EPSILON);
    }

    #[test]
    fn floor_and_cell_center_roundtrip() {
        let rc = ctx(0, 0, 80, 24);
        // Any fractional position floors to the cell, then +0.5 → cell center.
        // The roundtrip error should be at most 0.5 (symmetric, no directional bias).
        let original = Position::new(7.3, 4.8);
        let (tx, ty) = rc.canvas_to_terminal(original);
        let back = rc.terminal_to_canvas(tx as u16, ty as u16);
        assert!((back.x - 7.5).abs() < f64::EPSILON);
        assert!((back.y - 4.5).abs() < f64::EPSILON);
    }

    #[test]
    fn world_terminal_roundtrip_with_zoom_and_pan() {
        let rc = ctx(5, 3, 80, 24);
        let vp = Viewport::new(10.0, 5.0, 1.5);
        let world_pos = Position::new(20.0, 15.0);
        let (tx, ty) = rc.world_to_terminal(&vp, world_pos);
        assert!(tx >= 0 && ty >= 0);
        let back = rc.terminal_to_world(&vp, tx as u16, ty as u16);
        // Error bounded by 1.0/zoom (cell granularity scaled to world)
        assert!((back.x - world_pos.x).abs() < 1.0 / vp.zoom);
        assert!((back.y - world_pos.y).abs() < 1.0 / vp.zoom);
    }

    #[test]
    fn roundtrip_error_bounded_across_zoom_levels() {
        let rc = ctx(0, 0, 80, 24);
        // Error should be bounded by 0.5/zoom at every zoom level
        for &zoom in &[0.25, 0.5, 1.0, 1.5, 2.0, 4.0, 8.0] {
            let vp = Viewport::new(3.7, -2.1, zoom);
            let world_pos = Position::new(13.37, 7.77);
            let (tx, ty) = rc.world_to_terminal(&vp, world_pos);
            if tx < 0 || ty < 0 {
                continue; // off-screen, can't round-trip through u16
            }
            let back = rc.terminal_to_world(&vp, tx as u16, ty as u16);
            let err_x = (back.x - world_pos.x).abs();
            let err_y = (back.y - world_pos.y).abs();
            assert!(
                err_x <= 0.5 / zoom + f64::EPSILON,
                "zoom={zoom}: x error {err_x} > 0.5/{zoom}"
            );
            assert!(
                err_y <= 0.5 / zoom + f64::EPSILON,
                "zoom={zoom}: y error {err_y} > 0.5/{zoom}"
            );
        }
    }

    #[test]
    fn floor_boundary_same_width_nodes() {
        // Two nodes with same world width at different x positions.
        // At fractional zoom, floor(right) - floor(left) can differ by 1 cell
        // depending on where the fractional part lands. This test documents the behavior.
        let rc = ctx(0, 0, 200, 50);
        let vp = Viewport::new(0.0, 0.0, 1.3);

        // Node A at x=0, width=10 → canvas [0, 13) → terminal [0, 13)
        let (la, _) = rc.world_to_terminal(&vp, Position::new(0.0, 0.0));
        let (ra, _) = rc.world_to_terminal(&vp, Position::new(10.0, 0.0));
        let wa = ra - la;

        // Node B at x=5, width=10 → canvas [6.5, 19.5) → terminal [6, 19)
        let (lb, _) = rc.world_to_terminal(&vp, Position::new(5.0, 0.0));
        let (rb, _) = rc.world_to_terminal(&vp, Position::new(15.0, 0.0));
        let wb = rb - lb;

        // Both should be within 1 cell of each other
        assert!((wa - wb).abs() <= 1, "width diff {wa} vs {wb} exceeds 1");
    }

    #[test]
    fn visible_world_area_shrinks_with_zoom() {
        let rc = ctx(0, 0, 80, 24);
        let vp = Viewport::new(0.0, 0.0, 2.0);
        let area = rc.visible_world_area(&vp);
        // At 2x zoom, visible world area is half the canvas dimensions
        assert!((area.width() - 40.0).abs() < f64::EPSILON);
        assert!((area.height() - 12.0).abs() < f64::EPSILON);
    }
}
