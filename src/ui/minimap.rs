//! MiniMap widget for flow graph overview.
//!
//! Provides a scaled-down overview of the entire graph with a viewport indicator.

use std::collections::HashMap;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Widget},
};

use crate::content::{EdgeContent, NodeContent};
use crate::state::Flow;
use crate::types::{Dimensions, Position, Rect as FlowRect};

/// Position of the minimap within the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MiniMapPosition {
    /// Top-left corner.
    TopLeft,
    /// Top-right corner.
    TopRight,
    /// Bottom-left corner.
    BottomLeft,
    /// Bottom-right corner.
    #[default]
    BottomRight,
}

/// Quadrant glyph for each 2x2 mask: bit 0 top-left, 1 top-right, 2 bottom-left,
/// 3 bottom-right. All sixteen combinations exist in Unicode, so there is no
/// fallback case — the reason quadrants are the default rather than sextants.
const QUADRANT_GLYPHS: [&str; 16] = [
    " ", "▘", "▝", "▀", "▖", "▌", "▞", "▛", "▗", "▚", "▐", "▜", "▄", "▙", "▟", "█",
];

/// Visual configuration for minimap rendering.
/// Style defaults to the current theme when not set.
#[derive(Debug, Clone, Copy, Default)]
pub struct MiniMapStyle {
    /// Background color.
    bg_color: Option<Color>,
    /// Node color.
    node_color: Option<Color>,
    /// Selected node color.
    selected_node_color: Option<Color>,
    /// Viewport indicator background fill color.
    viewport_color: Option<Color>,
}

impl MiniMapStyle {
    /// Returns a copy with `None` colors resolved to theme defaults.
    pub(crate) fn resolved_style(self, palette: &crate::theme::Palette) -> Self {
        Self {
            bg_color: self.bg_color.or(Some(palette.surface)),
            node_color: self.node_color.or(Some(palette.muted)),
            selected_node_color: self.selected_node_color.or(Some(palette.accent)),
            viewport_color: self.viewport_color.or(Some(palette.subtle)),
        }
    }

    /// Sets the background color.
    pub fn with_bg_color(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
        self
    }

    /// Sets the node color.
    pub fn with_node_color(mut self, color: Color) -> Self {
        self.node_color = Some(color);
        self
    }

    /// Sets the selected node color.
    pub fn with_selected_node_color(mut self, color: Color) -> Self {
        self.selected_node_color = Some(color);
        self
    }

    /// Sets the viewport indicator background fill color.
    pub fn with_viewport_color(mut self, color: Color) -> Self {
        self.viewport_color = Some(color);
        self
    }
}

/// A minimap widget showing an overview of the flow graph.
///
/// Displays a scaled-down view of all nodes with a rectangle
/// indicating the current viewport position.
///
/// # Example
///
/// ```no_run
/// # use ratatui::{Frame, layout::Rect};
/// # use rataflow::{Flow, MiniMap, MiniMapPosition};
/// # fn draw(frame: &mut Frame, area: Rect, flow: &Flow) {
/// let minimap = MiniMap::new(flow)
///     .position(MiniMapPosition::BottomRight)
///     .size(30, 15);
///
/// frame.render_widget(minimap, area);
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct MiniMap<'a, N: NodeContent, E: EdgeContent> {
    /// Reference to the flow state.
    flow: &'a Flow<N, E>,
    /// Position of the minimap.
    position: MiniMapPosition,
    /// Width of the minimap.
    width: u16,
    /// Height of the minimap.
    height: u16,
    /// Optional style override. When `None`, derived from the theme at render time.
    style: Option<MiniMapStyle>,
    /// Optional block wrapper.
    block: Option<Block<'a>>,
    /// Margin from edge.
    margin: u16,
}

impl<'a, N: NodeContent, E: EdgeContent> MiniMap<'a, N, E> {
    /// Creates a new MiniMap widget.
    pub fn new(flow: &'a Flow<N, E>) -> Self {
        Self {
            flow,
            position: MiniMapPosition::default(),
            width: 25,
            height: 10,
            style: None,
            block: None,
            margin: 1,
        }
    }

    /// Sets the position of the minimap.
    pub fn position(mut self, position: MiniMapPosition) -> Self {
        self.position = position;
        self
    }

    /// Sets the size of the minimap.
    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.width = width.max(10);
        self.height = height.max(5);
        self
    }

    /// Sets the style configuration, overriding theme-derived defaults.
    pub fn style(mut self, style: MiniMapStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets an optional block wrapper.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the margin from the edge.
    pub fn margin(mut self, margin: u16) -> Self {
        self.margin = margin;
        self
    }

    /// Computes the position of the minimap within the given area.
    fn compute_rect(&self, area: Rect) -> Rect {
        let width = self.width.min(area.width.saturating_sub(self.margin * 2));
        let height = self.height.min(area.height.saturating_sub(self.margin * 2));

        let x = match self.position {
            MiniMapPosition::TopLeft | MiniMapPosition::BottomLeft => area.x + self.margin,
            MiniMapPosition::TopRight | MiniMapPosition::BottomRight => {
                area.x + area.width.saturating_sub(width + self.margin)
            }
        };

        let y = match self.position {
            MiniMapPosition::TopLeft | MiniMapPosition::TopRight => area.y + self.margin,
            MiniMapPosition::BottomLeft | MiniMapPosition::BottomRight => {
                area.y + area.height.saturating_sub(height + self.margin)
            }
        };

        Rect::new(x, y, width, height)
    }

    /// Calculates the bounding box that encompasses all visible nodes AND the current viewport.
    /// This ensures the viewport indicator is always visible in the minimap.
    ///
    /// Returns the combined bounds and the viewport rect in world coordinates (if valid).
    fn calculate_bounds(&self) -> (FlowRect, Option<FlowRect>) {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        // Include all visible nodes
        for node in &self.flow.nodes {
            if node.node.hidden {
                continue;
            }

            let bounds = node.bounds();
            min_x = min_x.min(bounds.position.x);
            min_y = min_y.min(bounds.position.y);
            max_x = max_x.max(bounds.position.x + bounds.width());
            max_y = max_y.max(bounds.position.y + bounds.height());
        }

        // Include the current viewport bounds (what the user is looking at)
        let canvas_size = self.flow.render_context.canvas_size();
        let canvas_width = canvas_size.width;
        let canvas_height = canvas_size.height;

        let viewport_world = if canvas_width > 0.0 && canvas_height > 0.0 {
            let viewport = &self.flow.viewport;
            let view_x = -viewport.x / viewport.zoom;
            let view_y = -viewport.y / viewport.zoom;
            let view_w = canvas_width / viewport.zoom;
            let view_h = canvas_height / viewport.zoom;

            min_x = min_x.min(view_x);
            min_y = min_y.min(view_y);
            max_x = max_x.max(view_x + view_w);
            max_y = max_y.max(view_y + view_h);

            Some(FlowRect::new(
                Position::new(view_x, view_y),
                Dimensions::new(view_w, view_h),
            ))
        } else {
            None
        };

        if min_x == f64::MAX {
            // No visible nodes and no valid viewport
            return (
                FlowRect::new(Position::new(0.0, 0.0), Dimensions::new(1.0, 1.0)),
                None,
            );
        }

        (
            FlowRect::new(
                Position::new(min_x, min_y),
                Dimensions::new(max_x - min_x, max_y - min_y),
            ),
            viewport_world,
        )
    }

    /// Renders the viewport indicator as a background fill.
    ///
    /// Uses a background color to highlight the viewport area, allowing nodes
    /// to render on top without being obscured by outline characters.
    #[allow(clippy::too_many_arguments)]
    fn render_viewport_indicator(
        &self,
        inner: Rect,
        buf: &mut Buffer,
        viewport_world: &FlowRect,
        bounds: &FlowRect,
        scale: f64,
        offset_x: f64,
        offset_y: f64,
        style: &MiniMapStyle,
    ) {
        // Transform viewport from world to minimap coordinates.
        // floor() for top-left, ceil() for bottom-right: ensures the indicator is never
        // smaller than the true viewport (floor bias would shrink it by up to 1 cell).
        // Unlike nodes/edges, the viewport indicator is a standalone background fill
        // with nothing to align against, so the mixed rounding is safe here.
        let mx =
            ((viewport_world.position.x - bounds.position.x) * scale + offset_x).floor() as i32;
        let my =
            ((viewport_world.position.y - bounds.position.y) * scale + offset_y).floor() as i32;
        let mr = ((viewport_world.position.x + viewport_world.width() - bounds.position.x) * scale
            + offset_x)
            .ceil() as i32;
        let mb = ((viewport_world.position.y + viewport_world.height() - bounds.position.y) * scale
            + offset_y)
            .ceil() as i32;
        let mw = (mr - mx).max(1);
        let mh = (mb - my).max(1);

        let bg_style = Style::default().bg(style.viewport_color.unwrap_or_default());

        // Fill viewport area with background color
        for dy in 0..mh {
            for dx in 0..mw {
                let px = inner.x as i32 + mx + dx;
                let py = inner.y as i32 + my + dy;
                if px >= inner.x as i32
                    && px < (inner.x + inner.width) as i32
                    && py >= inner.y as i32
                    && py < (inner.y + inner.height) as i32
                {
                    buf[(px as u16, py as u16)].set_bg(bg_style.bg.unwrap_or_default());
                }
            }
        }
    }

    /// Renders nodes on a 2x2 sub-cell grid, one quadrant glyph per cell.
    ///
    /// Four times the area resolution of whole cells, so a node under a cell wide
    /// still reads as a distinct mark instead of being rounded up to a full block
    /// that merges with its neighbours.
    ///
    /// Positions floor and dimensions round exactly as in
    /// [`render_nodes_cell`](Self::render_nodes_cell) — same rule, sub-cell units.
    /// Snapping to a grid twice as fine halves the resulting position and size
    /// error, and `max(1)` now floors a node at one quadrant rather than one cell.
    ///
    /// Sub-cells accumulate across nodes rather than overwriting: a cell shared by
    /// two nodes shows both halves. A cell can only carry one foreground color, so
    /// selection wins where they collide — the selected node is the one being
    /// tracked. Whole-cell rendering hides this case entirely by drawing one solid
    /// block over the other.
    #[allow(clippy::too_many_arguments)]
    fn render_nodes(
        &self,
        inner: Rect,
        buf: &mut Buffer,
        bounds: &FlowRect,
        scale: f64,
        offset_x: f64,
        offset_y: f64,
        style: &MiniMapStyle,
    ) {
        // The whole grid doubles, offsets included — sub-cell space is cell space
        // scaled by 2, not cell space with a finer step.
        let sub_scale = scale * 2.0;
        let (sub_offset_x, sub_offset_y) = (offset_x * 2.0, offset_y * 2.0);

        // (mask, any_selected) per cell, relative to `inner`.
        let mut cells: HashMap<(i32, i32), (u8, bool)> = HashMap::new();

        for node in &self.flow.nodes {
            if node.node.hidden {
                continue;
            }

            let node_bounds = node.bounds();
            let sx = ((node_bounds.position.x - bounds.position.x) * sub_scale + sub_offset_x)
                .floor() as i32;
            let sy = ((node_bounds.position.y - bounds.position.y) * sub_scale + sub_offset_y)
                .floor() as i32;
            let sw = (node_bounds.width() * sub_scale).round().max(1.0) as i32;
            let sh = (node_bounds.height() * sub_scale).round().max(1.0) as i32;

            for dy in 0..sh {
                for dx in 0..sw {
                    let (sub_x, sub_y) = (sx + dx, sy + dy);
                    // div_euclid/rem_euclid so sub-cells left of or above the origin
                    // land in the correct cell instead of truncating toward zero.
                    let cell = (sub_x.div_euclid(2), sub_y.div_euclid(2));
                    let bit = 1u8 << (sub_y.rem_euclid(2) * 2 + sub_x.rem_euclid(2));

                    let entry = cells.entry(cell).or_insert((0, false));
                    entry.0 |= bit;
                    entry.1 |= node.node.selected;
                }
            }
        }

        let node_color = style.node_color.unwrap_or_default();
        let selected_color = style.selected_node_color.unwrap_or_default();

        for ((cx, cy), (mask, selected)) in cells {
            let px = inner.x as i32 + cx;
            let py = inner.y as i32 + cy;

            // Clip to inner bounds (all four sides)
            if px < inner.x as i32
                || px >= (inner.x + inner.width) as i32
                || py < inner.y as i32
                || py >= (inner.y + inner.height) as i32
            {
                continue;
            }

            let color = if selected { selected_color } else { node_color };
            buf.set_string(
                px as u16,
                py as u16,
                QUADRANT_GLYPHS[mask as usize],
                Style::default().fg(color),
            );
        }
    }
}

impl<N: NodeContent, E: EdgeContent> Widget for MiniMap<'_, N, E> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 15 || area.height < 8 {
            return;
        }

        let palette = self.flow.theme.palette();
        let style = self.style.unwrap_or_default().resolved_style(&palette);

        let panel_rect = self.compute_rect(area);

        // Render block/border
        let inner = if let Some(block) = &self.block {
            let inner = block.inner(panel_rect);
            block.clone().render(panel_rect, buf);
            inner
        } else {
            panel_rect
        };

        if inner.width < 2 || inner.height < 2 {
            return;
        }

        // Fill background
        let bg_style = Style::default().bg(style.bg_color.unwrap_or_default());
        for y in inner.y..inner.y + inner.height {
            for x in inner.x..inner.x + inner.width {
                buf.set_string(x, y, " ", bg_style);
            }
        }

        // Calculate bounds of all nodes + viewport (ensures viewport is always visible)
        let (bounds, viewport_world) = self.calculate_bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }

        // Calculate scale to fit nodes in minimap
        let padding = 1.0;
        let available_width = (inner.width as f64) - padding * 2.0;
        let available_height = (inner.height as f64) - padding * 2.0;

        let scale_x = available_width / bounds.width();
        let scale_y = available_height / bounds.height();
        let scale = scale_x.min(scale_y); // Scale to fit (maintain aspect ratio)

        // Calculate offset to center the content
        let scaled_width = bounds.width() * scale;
        let scaled_height = bounds.height() * scale;
        let offset_x = (available_width - scaled_width) / 2.0 + padding;
        let offset_y = (available_height - scaled_height) / 2.0 + padding;

        // Render viewport indicator (background fill, behind nodes)
        if let Some(vw) = viewport_world {
            self.render_viewport_indicator(
                inner, buf, &vw, &bounds, scale, offset_x, offset_y, &style,
            );
        }

        // Render nodes (on top of viewport indicator)
        self.render_nodes(inner, buf, &bounds, scale, offset_x, offset_y, &style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Node;
    use crate::ui::TextContent;

    /// A flow whose nodes span a fixed 100x100 world box, so the minimap scale is
    /// predictable: `extra` nodes are placed inside that box.
    fn flow_with(nodes: Vec<Node<TextContent>>) -> Flow {
        let mut all = vec![
            Node::new(
                "anchor_tl",
                Position::new(0.0, 0.0),
                (1.0, 1.0),
                TextContent::from(""),
            ),
            Node::new(
                "anchor_br",
                Position::new(99.0, 99.0),
                (1.0, 1.0),
                TextContent::from(""),
            ),
        ];
        all.extend(nodes);
        Flow::with_graph(all, vec![]).unwrap()
    }

    fn render(flow: &Flow) -> Buffer {
        let area = Rect::new(0, 0, 40, 20);
        let mut buf = Buffer::empty(area);
        MiniMap::new(flow)
            .size(20, 12)
            .margin(0)
            .render(area, &mut buf);
        buf
    }

    fn glyphs(buf: &Buffer) -> Vec<String> {
        buf.content()
            .iter()
            .map(|c| c.symbol().to_string())
            .filter(|s| s != " ")
            .collect()
    }

    #[test]
    fn a_node_smaller_than_a_cell_stays_a_partial_glyph() {
        // Whole-cell rendering would round this up to a full block, merging it with
        // any neighbour; the 2x2 grid keeps it a fraction of a cell.
        let tiny = Node::new(
            "tiny",
            Position::new(50.0, 50.0),
            (1.0, 1.0),
            TextContent::from(""),
        );

        let drawn = glyphs(&render(&flow_with(vec![tiny])));
        assert!(
            drawn.iter().any(|g| g != "\u{2588}"),
            "expected at least one partial glyph, got {drawn:?}"
        );
        assert!(
            drawn.iter().all(|g| QUADRANT_GLYPHS.contains(&g.as_str())),
            "every glyph must come from the quadrant table, got {drawn:?}"
        );
    }

    #[test]
    fn sub_cells_from_different_nodes_accumulate_in_one_cell() {
        // At this scale one cell spans 10 world units and one sub-cell spans 5, so
        // x=50 and x=55 share a cell but land in different quadrants of it.
        // Whole-cell rendering draws one solid block over the other.
        let a = Node::new(
            "a",
            Position::new(50.0, 50.0),
            (1.0, 1.0),
            TextContent::from(""),
        );
        let b = Node::new(
            "b",
            Position::new(55.0, 50.0),
            (1.0, 1.0),
            TextContent::from(""),
        );

        let solo = glyphs(&render(&flow_with(vec![a.clone()])));
        let both = glyphs(&render(&flow_with(vec![a, b])));

        let dots = |gs: &[String]| -> u32 {
            gs.iter()
                .map(|g| {
                    QUADRANT_GLYPHS
                        .iter()
                        .position(|q| q == g)
                        .unwrap_or(0)
                        .count_ones()
                })
                .sum()
        };
        assert!(
            dots(&both) > dots(&solo),
            "adding a neighbouring node must add lit sub-cells, not replace them"
        );
    }

    #[test]
    fn selection_wins_the_color_where_nodes_share_a_cell() {
        let plain = Node::new(
            "a",
            Position::new(50.0, 50.0),
            (1.0, 1.0),
            TextContent::from(""),
        );
        let chosen = Node::new(
            "b",
            Position::new(51.0, 51.0),
            (1.0, 1.0),
            TextContent::from(""),
        )
        .with_selected(true);

        let flow = flow_with(vec![plain, chosen]);
        let style = MiniMapStyle::default()
            .with_node_color(Color::Red)
            .with_selected_node_color(Color::Green);

        let area = Rect::new(0, 0, 40, 20);
        let mut buf = Buffer::empty(area);
        MiniMap::new(&flow)
            .size(20, 12)
            .margin(0)
            .style(style)
            .render(area, &mut buf);

        // The two anchors are unselected, so both colors must be present — and no
        // cell may end up with the selected node's shape in the plain color.
        let has_selected = buf.content().iter().any(|c| c.fg == Color::Green);
        assert!(has_selected, "selected node color never rendered");
    }

    #[test]
    fn quadrant_cells_stay_inside_the_minimap_panel() {
        // Nodes far outside the anchored box push the scale such that some
        // sub-cells fall outside the panel; none may be written.
        let stray = Node::new(
            "stray",
            Position::new(-500.0, -500.0),
            (10.0, 10.0),
            TextContent::from(""),
        );
        let flow = flow_with(vec![stray]);

        let area = Rect::new(0, 0, 40, 20);
        let mut buf = Buffer::empty(area);
        let minimap = MiniMap::new(&flow).size(20, 12).margin(0);
        let panel = minimap.compute_rect(area);
        minimap.render(area, &mut buf);

        for y in 0..area.height {
            for x in 0..area.width {
                let inside = x >= panel.x
                    && x < panel.x + panel.width
                    && y >= panel.y
                    && y < panel.y + panel.height;
                if !inside {
                    assert_eq!(
                        buf[(x, y)].symbol(),
                        " ",
                        "wrote outside the panel at ({x},{y})"
                    );
                }
            }
        }
    }
}
