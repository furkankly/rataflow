use ratatui::buffer::Buffer;
use ratatui::style::Style;
use ratatui::text::Text;

use rataflow::{
    Edge, EdgeContent, EdgePathContext, EdgeRenderContext, EdgeStyle, Flow, Handle, HandlePosition,
    Node, Path, StepEdge, StraightEdge, TextContent, compute_step_path, compute_straight_path,
};

#[derive(Clone, Debug)]
pub enum RoutingEdge {
    Step(StepEdge),
    Straight(StraightEdge),
}

impl Default for RoutingEdge {
    fn default() -> Self {
        RoutingEdge::Step(StepEdge::default())
    }
}

impl RoutingEdge {
    fn step() -> Self {
        RoutingEdge::Step(StepEdge::default())
    }

    fn straight() -> Self {
        // Mirrors `StraightEdge`'s own default so the section shows what the
        // builtin produces; this enum reimplements `render`, so it has to say so.
        RoutingEdge::Straight(
            StraightEdge::default()
                .with_style(EdgeStyle::braille())
                .with_selected_style(EdgeStyle::braille()),
        )
    }
}

impl EdgeContent for RoutingEdge {
    fn compute_path(&self, ctx: &EdgePathContext) -> Path {
        match self {
            RoutingEdge::Step(e) => compute_step_path(
                ctx.from,
                ctx.to,
                ctx.source_position,
                ctx.target_position,
                e.stem_length,
            ),
            RoutingEdge::Straight(_) => {
                compute_straight_path(ctx.from, ctx.to, ctx.source_position, ctx.target_position)
            }
        }
    }

    fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
        let (style_opt, selected_opt) = match self {
            RoutingEdge::Step(e) => (&e.style, &e.selected_style),
            RoutingEdge::Straight(e) => (&e.style, &e.selected_style),
        };
        let default_style;
        let style = if ctx.selected {
            match selected_opt.as_ref() {
                Some(s) => s,
                None => {
                    let palette = ctx.theme.palette();
                    default_style =
                        EdgeStyle::default().with_stroke_style(Style::default().fg(palette.accent));
                    &default_style
                }
            }
        } else {
            match style_opt.as_ref() {
                Some(s) => s,
                None => {
                    default_style = EdgeStyle::default();
                    &default_style
                }
            }
        };
        let label = ctx.label.map(Text::raw);
        ctx.render_path(style, label.as_ref(), buf);
    }
}

const POSITIONS: [HandlePosition; 4] = [
    HandlePosition::Top,
    HandlePosition::Right,
    HandlePosition::Bottom,
    HandlePosition::Left,
];

/// Grid geometry shared by both sections: the size of one cell, and where the
/// target node sits inside it relative to the source.
struct SectionGrid {
    cell_w: f64,
    cell_h: f64,
    target_dx: f64,
    target_dy: f64,
}

fn abbr(p: HandlePosition) -> &'static str {
    match p {
        HandlePosition::Top => "T",
        HandlePosition::Right => "R",
        HandlePosition::Bottom => "B",
        HandlePosition::Left => "L",
    }
}

/// All 16 source→target handle position combinations for both Step and Straight edges.
///
/// Top section: Step edges (orthogonal routing).
/// Bottom section: Straight edges (direct line).
/// Each section is a 4×4 grid — rows vary source handle, columns vary target handle.
pub fn create_flow() -> Flow<TextContent, RoutingEdge> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let grid = SectionGrid {
        cell_w: 30.0,
        cell_h: 14.0,
        target_dx: 16.0,
        target_dy: 6.0,
    };
    let section_gap = 8.0;

    // Section 1: Step edges
    let step_y_offset = 4.0;
    let header = Node::new(
        "header_step",
        (0.0, 0.0),
        (15.0, 3.0),
        TextContent::new("Step Edges"),
    );
    nodes.push(header);

    add_section(
        &mut nodes,
        &mut edges,
        "step",
        step_y_offset,
        &grid,
        RoutingEdge::step,
    );

    // Section 2: Straight edges
    let straight_y_offset = step_y_offset + 4.0 * grid.cell_h + section_gap;
    let header = Node::new(
        "header_straight",
        (0.0, straight_y_offset - 4.0),
        (18.0, 3.0),
        TextContent::new("Straight Edges"),
    );
    nodes.push(header);

    add_section(
        &mut nodes,
        &mut edges,
        "straight",
        straight_y_offset,
        &grid,
        RoutingEdge::straight,
    );

    Flow::with_graph(nodes, edges).expect("valid graph")
}

fn add_section(
    nodes: &mut Vec<Node<TextContent>>,
    edges: &mut Vec<Edge<RoutingEdge>>,
    prefix: &str,
    y_offset: f64,
    grid: &SectionGrid,
    make_edge: fn() -> RoutingEdge,
) {
    for (row, &src_pos) in POSITIONS.iter().enumerate() {
        for (col, &tgt_pos) in POSITIONS.iter().enumerate() {
            let x = col as f64 * grid.cell_w;
            let y = y_offset + row as f64 * grid.cell_h;

            let label = format!("{}→{}", abbr(src_pos), abbr(tgt_pos));
            let src_id = format!("{}_src_{}", prefix, label);
            let tgt_id = format!("{}_tgt_{}", prefix, label);
            let edge_id = format!("{}_e_{}", prefix, label);

            let src_node = Node::from_text(&src_id, (x, y), label.as_str())
                .with_handles(vec![Handle::source(src_pos)]);

            let tgt_node = Node::from_text(
                &tgt_id,
                (x + grid.target_dx, y + grid.target_dy),
                label.as_str(),
            )
            .with_handles(vec![Handle::target(tgt_pos)]);

            let edge = Edge::new(&edge_id, &src_id, &tgt_id)
                .with_content(make_edge())
                .with_label(label);

            nodes.push(src_node);
            nodes.push(tgt_node);
            edges.push(edge);
        }
    }
}
