use rataflow::{
    Edge, EdgeContent, EdgeMarker, EdgePathContext, EdgeRenderContext, EdgeStyle, Flow,
    HandlePosition, Node, Path, Position, TextContent, compute_step_path, compute_straight_path,
};
use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
    text::Text,
};

/// Enum-based edge type — each variant demonstrates a distinct `EdgeContent` capability.
#[derive(Clone, Debug)]
pub enum MyEdge {
    /// `EdgeStyle` builders: dotted line, colored chars, start/end markers.
    Styled {
        style: EdgeStyle,
        selected_style: EdgeStyle,
    },
    /// `EdgeStroke::Braille`: the same straight path at sub-cell resolution.
    Braille {
        style: EdgeStyle,
        selected_style: EdgeStyle,
    },
    /// `render_path` with a label and custom `stem_length` for wide step routing.
    Labeled {
        stem_length: f64,
        style: EdgeStyle,
        selected_style: EdgeStyle,
    },
    /// `world_to_terminal` + `is_in_bounds` escape hatch: animated edge with raw badge at midpoint.
    Decorated {
        badge: String,
        style: EdgeStyle,
        selected_style: EdgeStyle,
    },
    /// Custom `compute_path` returning a hand-built orthogonal zigzag `Path`.
    CustomPath {
        amplitude: f64,
        steps: usize,
        style: EdgeStyle,
        selected_style: EdgeStyle,
    },
}

impl Default for MyEdge {
    fn default() -> Self {
        MyEdge::Styled {
            style: EdgeStyle::default(),
            selected_style: EdgeStyle::default(),
        }
    }
}

impl MyEdge {
    pub fn styled() -> Self {
        MyEdge::Styled {
            style: EdgeStyle::dotted()
                .with_stroke_style(Style::default().fg(Color::Indexed(133)))
                .with_marker_start(EdgeMarker::Circle)
                .with_marker_end(EdgeMarker::Diamond),
            selected_style: EdgeStyle::dotted()
                .with_stroke_style(
                    Style::default()
                        .fg(Color::Indexed(133))
                        .add_modifier(Modifier::BOLD),
                )
                .with_marker_start(EdgeMarker::Circle)
                .with_marker_end(EdgeMarker::Diamond),
        }
    }

    /// The same geometry as [`styled`](Self::styled), drawn with braille dots.
    ///
    /// A character cell holds a 2×4 grid of them, so a diagonal climbs in eighths
    /// of a cell instead of jumping a whole one. Worth it exactly where a path is
    /// not axis-aligned — on a horizontal run, braille and `─` say the same thing.
    pub fn braille() -> Self {
        MyEdge::Braille {
            style: EdgeStyle::braille()
                .with_stroke_style(Style::default().fg(Color::Indexed(45)))
                .with_marker_end(EdgeMarker::Arrow),
            selected_style: EdgeStyle::braille()
                .with_stroke_style(
                    Style::default()
                        .fg(Color::Indexed(45))
                        .add_modifier(Modifier::BOLD),
                )
                .with_marker_end(EdgeMarker::Arrow),
        }
    }

    pub fn labeled() -> Self {
        MyEdge::Labeled {
            stem_length: 8.0,
            style: EdgeStyle::default()
                .with_stroke_style(Style::default().fg(Color::Indexed(71)))
                .with_label_style(
                    Style::default()
                        .fg(Color::Indexed(71))
                        .add_modifier(Modifier::ITALIC),
                )
                .with_marker_end(EdgeMarker::Arrow),
            selected_style: EdgeStyle::default()
                .with_label_style(
                    Style::default()
                        .fg(Color::Indexed(179))
                        .add_modifier(Modifier::ITALIC | Modifier::BOLD),
                )
                .with_marker_end(EdgeMarker::Arrow),
        }
    }

    pub fn decorated(badge: impl Into<String>) -> Self {
        MyEdge::Decorated {
            badge: badge.into(),
            style: EdgeStyle::default()
                .with_stroke_style(Style::default().fg(Color::Indexed(80)))
                .with_marker_end(EdgeMarker::ArrowClosed),
            selected_style: EdgeStyle::default().with_marker_end(EdgeMarker::ArrowClosed),
        }
    }

    pub fn custom_path(amplitude: f64, steps: usize) -> Self {
        let star_style = EdgeStyle::default()
            .with_line_chars('*', '*')
            .with_corner_chars(['*'; 4])
            .with_stroke_style(Style::default().fg(Color::Indexed(179)))
            .with_marker_start(EdgeMarker::Custom('⟐'))
            .with_marker_end(EdgeMarker::Custom('⟐'));
        let selected_star_style = EdgeStyle::default()
            .with_line_chars('*', '*')
            .with_corner_chars(['*'; 4])
            .with_stroke_style(
                Style::default()
                    .fg(Color::Indexed(179))
                    .add_modifier(Modifier::BOLD),
            )
            .with_marker_start(EdgeMarker::Custom('⟐'))
            .with_marker_end(EdgeMarker::Custom('⟐'));
        MyEdge::CustomPath {
            amplitude,
            steps,
            style: star_style,
            selected_style: selected_star_style,
        }
    }
}

impl EdgeContent for MyEdge {
    fn compute_path(&self, ctx: &EdgePathContext) -> Path {
        match self {
            MyEdge::Styled { .. } | MyEdge::Braille { .. } => {
                compute_straight_path(ctx.from, ctx.to, ctx.source_position, ctx.target_position)
            }
            MyEdge::Decorated { .. } => compute_step_path(
                ctx.from,
                ctx.to,
                ctx.source_position,
                ctx.target_position,
                3.0,
            ),
            MyEdge::Labeled { stem_length, .. } => compute_step_path(
                ctx.from,
                ctx.to,
                ctx.source_position,
                ctx.target_position,
                *stem_length,
            ),
            MyEdge::CustomPath {
                amplitude, steps, ..
            } => compute_zigzag_path(
                ctx.from,
                ctx.to,
                ctx.source_position,
                ctx.target_position,
                *amplitude,
                *steps,
            ),
        }
    }

    fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
        let (style, selected_style) = match self {
            MyEdge::Styled {
                style,
                selected_style,
            }
            | MyEdge::Braille {
                style,
                selected_style,
            }
            | MyEdge::Labeled {
                style,
                selected_style,
                ..
            }
            | MyEdge::Decorated {
                style,
                selected_style,
                ..
            }
            | MyEdge::CustomPath {
                style,
                selected_style,
                ..
            } => (style, selected_style),
        };

        let active_style = if ctx.selected { selected_style } else { style };

        // For Labeled variant, pass edge label through render_path.
        let label = if matches!(self, MyEdge::Labeled { .. }) {
            ctx.label
        } else {
            None
        };
        let label_text = label.map(Text::from);
        ctx.render_path(active_style, label_text.as_ref(), buf);

        // For Decorated variant, draw badges at start, midpoint, and end of the edge.
        if let MyEdge::Decorated { badge, .. } = self {
            let badge_color = if ctx.selected {
                Color::Indexed(179)
            } else {
                Color::Indexed(80)
            };
            let badge_style = Style::default()
                .fg(Color::Indexed(232))
                .bg(badge_color)
                .add_modifier(Modifier::BOLD);

            // Start badge: left edge at start point, extends rightward (away from source node).
            // Mid badge: centered on label position.
            // End badge: right edge at end point, extends leftward (away from target node).
            let start = ctx.path.start().unwrap_or(ctx.path.label_position);
            let end = ctx.path.end().unwrap_or(ctx.path.label_position);

            let badges: &[(&str, Position, i32)] = &[
                ("[SRC]", start, 0),
                (badge, ctx.path.label_position, -(badge.len() as i32) / 2),
                ("[DST]", end, -("[DST]".len() as i32)),
            ];

            for &(text, world_pos, x_offset) in badges {
                let (tx, ty) = ctx.world_to_terminal(world_pos);
                let tx = tx + x_offset;
                let ty = ty - 1;
                for (i, ch) in text.chars().enumerate() {
                    let bx = tx + i as i32;
                    if ctx.is_in_bounds(bx, ty) {
                        buf[(bx as u16, ty as u16)]
                            .set_char(ch)
                            .set_style(badge_style);
                    }
                }
            }
        }
    }
}

/// Computes an orthogonal square-wave (zigzag) path between two points.
///
/// The path alternates: right(step) → down(amp) → right(step) → up(amp) → ...
/// All segments are axis-aligned so they render cleanly with box-drawing characters.
fn compute_zigzag_path(
    from: Position,
    to: Position,
    source_position: HandlePosition,
    target_position: HandlePosition,
    amplitude: f64,
    steps: usize,
) -> Path {
    if steps == 0 {
        return compute_straight_path(from, to, source_position, target_position);
    }

    let dx = to.x - from.x;
    let step_x = dx / (steps as f64 * 2.0 + 1.0);

    let mut points = Vec::with_capacity(steps * 4 + 2);
    points.push(from);

    let mut x = from.x;
    let mid_y = (from.y + to.y) / 2.0;

    for i in 0..steps {
        let sign = if i % 2 == 0 { 1.0 } else { -1.0 };

        // Horizontal segment to turn point
        x += step_x;
        points.push(Position::new(x, mid_y));

        // Vertical segment (amplitude)
        points.push(Position::new(x, mid_y + sign * amplitude));

        // Horizontal segment past amplitude
        x += step_x;
        points.push(Position::new(x, mid_y + sign * amplitude));

        // Vertical back to midline
        points.push(Position::new(x, mid_y));
    }

    // Final horizontal to destination
    points.push(to);

    Path::new(points, source_position, target_position)
}

pub fn create_flow() -> Flow<TextContent, MyEdge> {
    let src_x = 5.0;
    let dst_x = 45.0;

    let nodes = vec![
        // Row 1: Styled
        Node::from_text("a1", (src_x, 2.0), "Styled").with_selected(true),
        Node::from_text("a2", (dst_x, 6.0), "Dotted + Markers"),
        // Row 2: Braille — same diagonal as row 1, for comparison
        Node::from_text("e1n", (src_x, 14.0), "Braille"),
        Node::from_text("e2n", (dst_x, 18.0), "Sub-cell Diagonal"),
        // Row 3: Labeled
        Node::from_text("b1", (src_x, 26.0), "Labeled"),
        Node::from_text("b2", (dst_x, 30.0), "Step + Label"),
        // Row 4: Decorated
        Node::from_text("c1", (src_x, 40.0), "Decorated"),
        Node::from_text("c2", (dst_x, 40.0), "Midpoint Badge"),
        // Row 5: Custom path
        Node::from_text("d1", (src_x, 50.0), "Custom"),
        Node::from_text("d2", (dst_x, 50.0), "Zigzag Geometry"),
    ];

    let edges = vec![
        // Magenta dotted with Circle start + Diamond end
        Edge::new("e1", "a1", "a2").with_content(MyEdge::styled()),
        // The same diagonal, at braille resolution
        Edge::new("e5", "e1n", "e2n").with_content(MyEdge::braille()),
        // Green step route with label
        Edge::new("e2", "b1", "b2")
            .with_content(MyEdge::labeled())
            .with_label("data flow"),
        // Cyan animated step with [OK] badge at midpoint
        Edge::new("e3", "c1", "c2")
            .with_content(MyEdge::decorated("[OK]"))
            .with_animated(true),
        // Yellow asterisk zigzag
        Edge::new("e4", "d1", "d2").with_content(MyEdge::custom_path(3.0, 3)),
    ];

    Flow::with_graph(nodes, edges).expect("valid graph")
}
