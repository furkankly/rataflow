//! Canvas rendering for flow graphs.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use crate::content::{
    EdgeContent, EdgePathContext, EdgeRenderContext, NodeContent, NodeRenderContext,
};
use crate::state::{DragState, Flow};
use crate::theme::Theme;
use crate::types::InternalNode;

use super::edge_preview::render_drag_edge_preview;
use super::handle_render::{HandleStyle, render_handle};

impl<N: NodeContent, E: EdgeContent> Widget for &mut Flow<N, E> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Resolve any deferred hierarchy from drag events
        self.resolve_drag_hierarchy_if_pending();
        // Recompute z-order if dirty (selection, z-index, or node changes)
        self.ensure_z_order();

        // Render block if present
        let canvas_area = if let Some(ref block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        // Update render context for coordinate transforms
        self.render_context.set_canvas_area(canvas_area);

        // Apply deferred fit-view (re-fits until canvas size stabilizes)
        self.apply_pending_fit_view();

        if canvas_area.width < 2 || canvas_area.height < 2 {
            return;
        }

        let render_ctx = &self.render_context;
        let viewport = &self.viewport;

        let visible_area = render_ctx.visible_world_area(viewport);

        // Render edges to separate buffer (allows edge merging at intersections)
        let mut edge_buf = Buffer::empty(canvas_area);

        render_edges(self, &visible_area, &mut edge_buf);

        render_edge_preview::<N, E>(self, &mut edge_buf);

        // Composite edge buffer onto main buffer
        // Only copy non-empty cells (edges overwrite background, not vice versa)
        composite_cells(&edge_buf, canvas_area, buf, canvas_area, false);

        render_nodes(self, &visible_area, canvas_area, buf);

        render_selection_box(self, buf);
    }
}

/// Draws the drag selection box on top of everything.
///
/// Theme-coloured with no style knob: a marquee is transient feedback, not part of
/// the graph's appearance.
fn render_selection_box<N: NodeContent, E: EdgeContent>(flow: &Flow<N, E>, buf: &mut Buffer) {
    let DragState::SelectingBox { anchor, current } = flow.drag_state else {
        return;
    };

    let (left, top) = flow.render_context.world_to_terminal(
        &flow.viewport,
        crate::Position::new(anchor.x.min(current.x), anchor.y.min(current.y)),
    );
    let (right, bottom) = flow.render_context.world_to_terminal(
        &flow.viewport,
        crate::Position::new(anchor.x.max(current.x), anchor.y.max(current.y)),
    );

    let style = Style::default().fg(flow.theme.palette().accent);
    let mut plot = |x: i32, y: i32, ch: char| {
        if flow.render_context.is_in_canvas(x, y) {
            buf[(x as u16, y as u16)].set_char(ch).set_style(style);
        }
    };

    // Dashed box-drawing, deliberately unlike every `BackgroundVariant`: `Dots`
    // draws the same middle dot a dotted outline would, and `Lines`/`Cross` draw
    // the solid forms of these. These read as a marquee against all three.
    for x in left..=right {
        plot(x, top, '┄');
        plot(x, bottom, '┄');
    }
    for y in top..=bottom {
        plot(left, y, '┊');
        plot(right, y, '┊');
    }
    // Corners last: the loops above cross there, and a dashed run reading through
    // the turn looks like a broken box rather than a closed one.
    plot(left, top, '╭');
    plot(right, top, '╮');
    plot(left, bottom, '╰');
    plot(right, bottom, '╯');
}

/// Renders all visible edges into the edge buffer.
///
/// Skips hidden edges and any edge currently being reconnected (replaced by preview).
/// Each edge is resolved through its source/target handles, path-computed in world
/// coordinates, visibility-culled, then rendered via [`EdgeContent::render`].
fn render_edges<N: NodeContent, E: EdgeContent>(
    flow: &Flow<N, E>,
    visible_area: &crate::Rect,
    edge_buf: &mut Buffer,
) {
    let render_ctx = &flow.render_context;
    let viewport = &flow.viewport;

    // Edge being reconnected should be hidden during drag (preview replaces it)
    let reconnecting_edge_id =
        if let DragState::ReconnectingEdge { ref edge_id, .. } = flow.drag_state {
            Some(edge_id.as_str())
        } else {
            None
        };

    for edge in &flow.edges {
        if edge.hidden {
            continue;
        }

        // Skip the edge being reconnected (preview replaces it)
        if reconnecting_edge_id == Some(edge.id.as_str()) {
            continue;
        }

        if let Some((source_handle, target_handle)) = flow.resolve_edge_handles(edge) {
            // Compute the path in world coordinates
            let path = edge.content.compute_path(&EdgePathContext {
                from: source_handle.absolute_position,
                to: target_handle.absolute_position,
                source_position: source_handle.position,
                target_position: target_handle.position,
                source_bounds: flow.node_bounds(&edge.source).unwrap_or_default(),
                target_bounds: flow.node_bounds(&edge.target),
            });

            // Endpoint offsets come from the path, not from the handles: an edge is
            // free to leave from a side other than the one its handle sits on, and
            // the marker has to sit on the side the line actually uses.
            let source_offset = path.source_position.edge_endpoint_render_offset();
            let target_offset = path.target_position.edge_endpoint_render_offset();

            // Cull edges whose full path bounds are off-screen
            if !path.bounds().intersects(visible_area) {
                continue;
            }

            let animation_phase = if edge.animated {
                Some((flow.animation_elapsed_ms / flow.animation_speed_ms.max(1)) as usize)
            } else {
                None
            };

            // Create edge render context with world-coordinate path
            let ctx = EdgeRenderContext {
                id: &edge.id,
                selected: edge.selected,
                label: edge.label.as_deref(),
                path,
                theme: flow.theme,
                viewport,
                render_ctx,
                source_endpoint_offset: source_offset,
                target_endpoint_offset: target_offset,
                animation_phase,
            };
            edge.content.render(&ctx, edge_buf);
        }
    }
}

/// Renders the edge preview (connection creation, reconnection, or programmatic).
///
/// Only renders when `to_world` is `Some` — `start_edge_preview()` enters connection
/// mode with `to_world = None` (no visible line until a target is set).
fn render_edge_preview<N: NodeContent, E: EdgeContent>(flow: &Flow<N, E>, edge_buf: &mut Buffer) {
    if let Some(ref ep) = flow.edge_preview
        && let Some(to_world) = ep.to_world
        && let Some(source_node) = flow.internal_node(&ep.from_node_id)
        && let Some(handle) = source_node
            .handle_bounds
            .find(ep.from_handle_id.as_deref(), ep.from_handle_type)
    {
        let target_handle = ep.to_node_id.as_ref().and_then(|to_node_id| {
            let target_node = flow.internal_node(to_node_id)?;
            target_node
                .handle_bounds
                .find(ep.to_handle_id.as_deref(), ep.from_handle_type.opposite())
        });
        let target_bounds = ep.to_node_id.as_ref().and_then(|id| flow.node_bounds(id));
        let preview_style = flow.preview_style.unwrap_or_default();
        let palette = flow.theme.palette();
        render_drag_edge_preview::<E>(
            handle,
            source_node.bounds(),
            to_world,
            target_handle,
            target_bounds,
            ep.is_valid,
            &preview_style,
            &palette,
            &flow.render_context,
            &flow.viewport,
            edge_buf,
        );
    }
}

/// Renders all visible nodes and their handles in z-order.
///
/// Each node's handles render right after its body, so a front node's body
/// naturally occludes a behind node's handles.
fn render_nodes<N: NodeContent, E: EdgeContent>(
    flow: &Flow<N, E>,
    visible_area: &crate::Rect,
    canvas_area: Rect,
    buf: &mut Buffer,
) {
    let render_ctx = &flow.render_context;
    let viewport = &flow.viewport;

    let dragging_node_id = match &flow.drag_state {
        DragState::MovingNode { node_id, .. } => Some(node_id.as_str()),
        _ => None,
    };

    // Same clock as edge marching ants — see NodeRenderContext::animation_phase.
    let animation_phase = (flow.animation_elapsed_ms / flow.animation_speed_ms.max(1)) as usize;

    for &idx in flow.z_ordered_indices() {
        let node = &flow.nodes[idx];
        if node.node.hidden || !node.bounds().intersects(visible_area) {
            continue;
        }
        let dragging = dragging_node_id == Some(node.node.id.as_str());
        render_node(
            node,
            render_ctx,
            viewport,
            canvas_area,
            dragging,
            flow.theme,
            animation_phase,
            buf,
        );
        render_handles(node, render_ctx, viewport, canvas_area, flow.theme, buf);
        if node.node.resizable {
            render_resize_grip(node, render_ctx, viewport, flow.theme, buf);
        }
    }
}

/// Marks a resizable node's bottom-right corner, where the resize drag begins.
///
/// Drawn for every resizable node rather than only the selected one, because the
/// grip is the affordance — hiding it until selection would make the gesture
/// undiscoverable.
fn render_resize_grip<N: NodeContent>(
    node: &InternalNode<N>,
    render_ctx: &crate::state::RenderContext,
    viewport: &crate::types::Viewport,
    theme: Theme,
    buf: &mut Buffer,
) {
    let bounds = crate::Rect::new(node.position_absolute, node.dimensions());
    let (_, _, right, bottom) = render_ctx.world_to_terminal_rect(viewport, bounds);
    let (x, y) = (right - 1, bottom - 1);
    if render_ctx.is_in_canvas(x, y) {
        buf[(x as u16, y as u16)]
            .set_char('◢')
            .set_style(Style::default().fg(theme.palette().accent));
    }
}

/// Renders a single node to the buffer.
///
/// Each node is rendered into a per-node scratch buffer at full dimensions
/// (local coordinates from `(0, 0)`), then only the visible portion is
/// composited onto the main buffer. This ensures [`NodeContent::render`]
/// always receives the node's complete area — correct partial rendering
/// and sidestepping ratatui's u16 coordinate space (nodes extending off
/// the left/top edge have negative terminal positions).
#[allow(clippy::too_many_arguments)]
fn render_node<N: NodeContent>(
    node: &InternalNode<N>,
    render_ctx: &crate::state::RenderContext,
    viewport: &crate::types::Viewport,
    canvas_area: Rect,
    dragging: bool,
    theme: Theme,
    animation_phase: usize,
    buf: &mut Buffer,
) {
    let dimensions = node.dimensions();
    let node_world_rect = crate::Rect::new(node.position_absolute, dimensions);
    let (left, top, right, bottom) = render_ctx.world_to_terminal_rect(viewport, node_world_rect);

    // Clip to canvas bounds
    let ca_right = canvas_area.x as i32 + canvas_area.width as i32;
    let ca_bottom = canvas_area.y as i32 + canvas_area.height as i32;
    let vis_left = left.max(canvas_area.x as i32);
    let vis_top = top.max(canvas_area.y as i32);
    let vis_right = right.min(ca_right);
    let vis_bottom = bottom.min(ca_bottom);

    if vis_left < vis_right && vis_top < vis_bottom {
        let visible_area = Rect::new(
            vis_left as u16,
            vis_top as u16,
            (vis_right - vis_left) as u16,
            (vis_bottom - vis_top) as u16,
        );

        let full_w = (right - left) as u16;
        let full_h = (bottom - top) as u16;
        let local_area = Rect::new(0, 0, full_w, full_h);

        // Scratch buffer at local (0,0) with full node dimensions
        let mut scratch = Buffer::empty(local_area);

        let ctx = NodeRenderContext {
            id: &node.node.id,
            area: local_area,
            selected: node.node.selected,
            dragging,
            position_absolute: node.position_absolute,
            theme,
            animation_phase,
        };
        node.node.content.render(&ctx, &mut scratch);

        // Offset from the full terminal rect's top-left to the visible rect's top-left
        let src_x = (vis_left - left) as u16;
        let src_y = (vis_top - top) as u16;
        let src_area = Rect::new(src_x, src_y, visible_area.width, visible_area.height);

        // Composite node cells onto the main buffer. When opaque, all cells are
        // written (blocking content behind). When transparent, only cells the
        // NodeContent actually touched are written.
        composite_cells(&scratch, src_area, buf, visible_area, node.node.opaque);
    }
}

/// Renders handles for a node.
///
/// Style fallback chain:
/// 1. Non-connectable node: `handle.disabled_style` → `HandleStyle::disabled()` → theme muted color
/// 2. Connectable node: `handle.style` → `HandleStyle::default()` → theme accent color
fn render_handles<N>(
    node: &InternalNode<N>,
    render_ctx: &crate::state::RenderContext,
    viewport: &crate::types::Viewport,
    canvas_area: Rect,
    theme: Theme,
    buf: &mut Buffer,
) {
    let palette = theme.palette();
    let accent = Style::default().fg(palette.accent);
    let muted = Style::default().fg(palette.muted);
    let default_style = HandleStyle::default();
    let disabled_default = HandleStyle::disabled();

    // Render all source handles
    for handle in &node.handle_bounds.source {
        if handle.hidden {
            continue;
        }
        let (x, y) = render_ctx.world_to_terminal(viewport, handle.absolute_position);
        let (off_x, off_y) = handle.handle_render_offset();
        let resolved_style = if !node.node.connectable {
            handle
                .disabled_style
                .unwrap_or(disabled_default)
                .resolved_style(muted)
        } else {
            handle.style.unwrap_or(default_style).resolved_style(accent)
        };
        render_handle(
            x + off_x,
            y + off_y,
            canvas_area,
            &resolved_style,
            handle.position,
            buf,
        );
    }

    // Render all target handles
    for handle in &node.handle_bounds.target {
        if handle.hidden {
            continue;
        }
        let (x, y) = render_ctx.world_to_terminal(viewport, handle.absolute_position);
        let (off_x, off_y) = handle.handle_render_offset();
        let resolved_style = if !node.node.connectable {
            handle
                .disabled_style
                .unwrap_or(disabled_default)
                .resolved_style(muted)
        } else {
            handle.style.unwrap_or(default_style).resolved_style(accent)
        };
        render_handle(
            x + off_x,
            y + off_y,
            canvas_area,
            &resolved_style,
            handle.position,
            buf,
        );
    }
}

/// Composites cells from `source` onto `dest`.
///
/// When `opaque` is false, cells at `Cell::default()` are skipped — content behind
/// (edges, background, lower z-index nodes) shows through. When `opaque` is true,
/// all cells are written, blocking whatever is behind.
///
/// When a source cell has no explicit background (`Color::Reset`), the destination's
/// background is preserved. This ensures cells that only set fg (edge characters,
/// transparent nodes) don't punch through the canvas background.
fn composite_cells(
    source: &Buffer,
    source_area: Rect,
    dest: &mut Buffer,
    dest_area: Rect,
    opaque: bool,
) {
    debug_assert_eq!(source_area.width, dest_area.width);
    debug_assert_eq!(source_area.height, dest_area.height);

    let default = ratatui::buffer::Cell::default();
    for dy in 0..source_area.height {
        for dx in 0..source_area.width {
            let cell = &source[(source_area.x + dx, source_area.y + dy)];
            if opaque || cell != &default {
                let dest_cell = &mut dest[(dest_area.x + dx, dest_area.y + dy)];
                let dest_bg = dest_cell.bg;
                *dest_cell = cell.clone();
                if cell.bg == Color::Reset {
                    dest_cell.bg = dest_bg;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buf(w: u16, h: u16) -> (Buffer, Rect) {
        let area = Rect::new(0, 0, w, h);
        (Buffer::empty(area), area)
    }

    #[test]
    fn untouched_cells_are_transparent() {
        let (source, area) = make_buf(3, 1);
        // source is all Cell::default() — nothing was rendered
        let (mut dest, dest_area) = make_buf(3, 1);
        dest[(0, 0)].set_char('X');
        dest[(1, 0)].set_char('Y');
        dest[(2, 0)].set_char('Z');

        composite_cells(&source, area, &mut dest, dest_area, false);

        // Destination is untouched — transparent source cells don't overwrite
        assert_eq!(dest[(0, 0)].symbol(), "X");
        assert_eq!(dest[(1, 0)].symbol(), "Y");
        assert_eq!(dest[(2, 0)].symbol(), "Z");
    }

    #[test]
    fn touched_cells_overwrite_destination() {
        let (mut source, area) = make_buf(3, 1);
        source[(1, 0)].set_char('A');
        let (mut dest, dest_area) = make_buf(3, 1);
        dest[(0, 0)].set_char('X');
        dest[(1, 0)].set_char('Y');
        dest[(2, 0)].set_char('Z');

        composite_cells(&source, area, &mut dest, dest_area, false);

        assert_eq!(dest[(0, 0)].symbol(), "X"); // untouched in source
        assert_eq!(dest[(1, 0)].symbol(), "A"); // overwritten
        assert_eq!(dest[(2, 0)].symbol(), "Z"); // untouched in source
    }

    #[test]
    fn color_reset_preserves_destination_background() {
        let (mut source, area) = make_buf(1, 1);
        // Edge character: sets fg but leaves bg as Color::Reset (default)
        source[(0, 0)].set_char('│');
        source[(0, 0)].set_style(ratatui::style::Style::default().fg(Color::White));
        assert_eq!(source[(0, 0)].bg, Color::Reset);

        let (mut dest, dest_area) = make_buf(1, 1);
        dest[(0, 0)].set_style(ratatui::style::Style::default().bg(Color::DarkGray));

        composite_cells(&source, area, &mut dest, dest_area, false);

        // Symbol and fg come from source
        assert_eq!(dest[(0, 0)].symbol(), "│");
        assert_eq!(dest[(0, 0)].fg, Color::White);
        // Background preserved from destination — the Color::Reset special case
        assert_eq!(dest[(0, 0)].bg, Color::DarkGray);
    }

    #[test]
    fn explicit_background_overwrites_destination() {
        let (mut source, area) = make_buf(1, 1);
        source[(0, 0)].set_char('█');
        source[(0, 0)].set_style(
            ratatui::style::Style::default()
                .fg(Color::White)
                .bg(Color::Blue),
        );

        let (mut dest, dest_area) = make_buf(1, 1);
        dest[(0, 0)].set_style(ratatui::style::Style::default().bg(Color::Red));

        composite_cells(&source, area, &mut dest, dest_area, false);

        // Explicit bg replaces destination bg (not Color::Reset, so no preservation)
        assert_eq!(dest[(0, 0)].bg, Color::Blue);
    }

    #[test]
    fn opaque_blocks_content_behind() {
        let (source, area) = make_buf(3, 1);
        // source is all Cell::default() — nothing was rendered
        let (mut dest, dest_area) = make_buf(3, 1);
        dest[(0, 0)].set_char('X');
        dest[(1, 0)].set_char('Y');
        dest[(2, 0)].set_char('Z');

        composite_cells(&source, area, &mut dest, dest_area, true);

        // Opaque: untouched cells still overwrite destination
        assert_eq!(dest[(0, 0)].symbol(), " ");
        assert_eq!(dest[(1, 0)].symbol(), " ");
        assert_eq!(dest[(2, 0)].symbol(), " ");
    }

    #[test]
    fn opaque_preserves_destination_background() {
        let (source, area) = make_buf(1, 1);
        // source is Cell::default() — untouched, bg is Color::Reset
        let (mut dest, dest_area) = make_buf(1, 1);
        dest[(0, 0)].set_char('X');
        dest[(0, 0)].set_style(ratatui::style::Style::default().bg(Color::DarkGray));

        composite_cells(&source, area, &mut dest, dest_area, true);

        // Opaque blocks the character but preserves destination bg
        assert_eq!(dest[(0, 0)].symbol(), " ");
        assert_eq!(dest[(0, 0)].bg, Color::DarkGray);
    }
}
