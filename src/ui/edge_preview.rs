//! Edge preview rendering and style configuration.

use ratatui::buffer::Buffer;
use ratatui::style::Color;

use crate::content::EdgeContent;
use crate::state::RenderContext;
use crate::theme::Palette;
use crate::types::{ComputedHandle, Position, Viewport};

use super::edge_render::{EdgeStroke, EdgeStyle, render_path};

/// Visual configuration for edge preview rendering.
///
/// Configures how edge previews are drawn during connection creation. The preview
/// inherits its path shape from the edge type (`E::default()`); how that path is
/// rasterized and colored is configured here.
///
/// The three colors map to validation states:
/// - **Valid color** — hovering over a compatible target handle
/// - **Invalid color** — hovering over an incompatible target handle
/// - **No-target color** — not hovering over any handle
///
/// # Examples
///
/// ```no_run
/// use ratatui::style::Color;
/// use rataflow::EdgePreviewStyle;
///
/// let style = EdgePreviewStyle::default()
///     .with_valid_color(Color::Cyan)
///     .with_no_target_color(Color::Gray);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EdgePreviewStyle {
    valid_color: Option<Color>,
    invalid_color: Option<Color>,
    no_target_color: Option<Color>,
    stroke: Option<EdgeStroke>,
}

impl EdgePreviewStyle {
    /// Sets the color shown when hovering over a valid target handle.
    pub fn with_valid_color(mut self, color: Color) -> Self {
        self.valid_color = Some(color);
        self
    }

    /// Sets the color shown when hovering over an invalid target handle.
    pub fn with_invalid_color(mut self, color: Color) -> Self {
        self.invalid_color = Some(color);
        self
    }

    /// Sets the color shown when not hovering over any handle.
    pub fn with_no_target_color(mut self, color: Color) -> Self {
        self.no_target_color = Some(color);
        self
    }

    /// Sets how the preview line is rasterized.
    ///
    /// Defaults to the [`EdgeStroke`] default (box-drawing characters). Match it to
    /// your edge type when that strokes unusually, so the preview looks like the
    /// edge it becomes:
    ///
    /// ```
    /// # use rataflow::{EdgePreviewStyle, EdgeStroke};
    /// let style = EdgePreviewStyle::default().with_stroke(EdgeStroke::Braille);
    /// ```
    ///
    /// Only the stroke is taken; the color always comes from the validation state,
    /// so there is nothing here that could override it.
    pub fn with_stroke(mut self, stroke: EdgeStroke) -> Self {
        self.stroke = Some(stroke);
        self
    }

    /// Returns the appropriate color for the given target validity state.
    pub(crate) fn color_for(&self, is_valid_target: Option<bool>, palette: &Palette) -> Color {
        match is_valid_target {
            Some(true) => self.valid_color.unwrap_or(palette.success),
            Some(false) => self.invalid_color.unwrap_or(palette.error),
            None => self.no_target_color.unwrap_or(palette.accent),
        }
    }
}

/// Renders the edge preview line from source handle to target position.
///
/// When a target handle is resolved, uses its actual position for path routing
/// and its render offset for endpoint alignment. When no target handle is
/// resolved (free-space dragging), assumes the opposite direction.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_drag_edge_preview<E: EdgeContent>(
    source_handle: &ComputedHandle,
    source_bounds: crate::types::Rect,
    cursor_world: Position,
    target_handle: Option<&ComputedHandle>,
    target_bounds: Option<crate::types::Rect>,
    is_valid_target: Option<bool>,
    preview_style: &EdgePreviewStyle,
    palette: &Palette,
    render_ctx: &RenderContext,
    viewport: &Viewport,
    buf: &mut Buffer,
) {
    let target_position = target_handle
        .map(|h| h.position)
        .unwrap_or_else(|| source_handle.position.opposite());

    let preview_edge = E::default();
    let path = preview_edge.compute_path(&crate::content::EdgePathContext {
        from: source_handle.absolute_position,
        to: cursor_world,
        source_position: source_handle.position,
        target_position,
        source_bounds,
        target_bounds,
    });

    // From the path, for the same reason the rendered edge takes them from there:
    // the preview edge may leave from a side its handle does not sit on.
    let source_offset = path.source_position.edge_endpoint_render_offset();
    let target_offset = if target_handle.is_some() {
        path.target_position.edge_endpoint_render_offset()
    } else {
        // Free-space drag: the endpoint is the cursor, not a node border.
        (0, 0)
    };

    let color = preview_style.color_for(is_valid_target, palette);

    // The preview owns its appearance: rasterization from `preview_style`, color
    // from the validation state. An `EdgeStroke` carries no color, so the two
    // cannot conflict.
    let mut style = EdgeStyle::without_markers();
    if let Some(stroke) = preview_style.stroke {
        style = style.with_stroke(stroke);
    }
    let resolved_style = style.resolved_style(
        ratatui::style::Style::default().fg(color),
        ratatui::style::Style::default().fg(palette.text),
    );

    render_path(
        &path,
        &resolved_style,
        None,
        viewport,
        render_ctx,
        source_offset,
        target_offset,
        None,
        buf,
    );
}
