//! Overview demo — WASM-portable version of the native overview example.
//!
//! Ports the graph topology and portable node types (Text, Sparkline, RawBuffer)
//! from the native overview, replacing terminal-only nodes (ratatui-image, tachyonfx)
//! with a horizontal bar chart node.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, BorderType, Paragraph, Sparkline, Widget};
use ratatui::Frame;

use rataflow::{
    compute_step_path, compute_straight_path, Background, BackgroundStyle, BackgroundVariant,
    Controls, Edge, EdgeContent, EdgeMarker, EdgePathContext, EdgeRenderContext, EdgeStyle,
    EventResponse, Flow, FlowEvent, FlowOps, Handle, HandlePosition, HandleStyle, MiniMap, Node,
    NodeContent, NodeExtent, NodeRenderContext, Path, TextContent,
};

use crate::demo::{Demo, DemoApp};
use crate::DemoEntry;

// ============================================================================
// Node types
// ============================================================================

/// All node types in the overview example.
///
/// Demonstrates four rendering approaches: built-in `TextContent`, ratatui
/// `Sparkline`, raw buffer manipulation, and custom horizontal bars.
#[derive(Clone, Debug)]
pub enum OverviewNode {
    Text(TextContent),
    Sparkline {
        label: String,
        data: Vec<u64>,
    },
    RawBuffer {
        label: String,
    },
    Bars {
        label: String,
        items: Vec<(String, f64, Color)>,
    },
}

impl Default for OverviewNode {
    fn default() -> Self {
        OverviewNode::Text(TextContent::default())
    }
}

impl NodeContent for OverviewNode {
    fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {
        match self {
            OverviewNode::Text(tc) => render_text_node(ctx, tc, buf),
            OverviewNode::Sparkline { label, data } => render_sparkline(ctx, label, data, buf),
            OverviewNode::RawBuffer { label } => render_raw_buffer(ctx, label, buf),
            OverviewNode::Bars { label, items } => render_bars(ctx, label, items, buf),
        }
    }
}

fn render_text_node(ctx: &NodeRenderContext, content: &TextContent, buf: &mut Buffer) {
    let border_style = if ctx.selected {
        Style::default()
            .fg(Color::Indexed(80))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(75))
    };

    let block = Block::bordered().border_style(border_style);
    let paragraph = Paragraph::new(content.text.clone()).block(block);
    paragraph.render(ctx.area, buf);
}

fn render_sparkline(ctx: &NodeRenderContext, label: &str, data: &[u64], buf: &mut Buffer) {
    let border_style = if ctx.selected {
        Style::default()
            .fg(Color::Indexed(80))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(71))
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(label)
        .style(Style::default().bg(Color::Indexed(234)));
    let inner = block.inner(ctx.area);
    block.render(ctx.area, buf);

    Sparkline::default()
        .data(data)
        .style(
            Style::default()
                .fg(Color::Indexed(179))
                .bg(Color::Indexed(234)),
        )
        .render(inner, buf);
}

fn render_raw_buffer(ctx: &NodeRenderContext, label: &str, buf: &mut Buffer) {
    let area = ctx.area;
    if area.width < 3 || area.height < 3 {
        return;
    }

    let border_color = if ctx.selected {
        Color::Indexed(80)
    } else {
        Color::Indexed(75)
    };
    let border_style = Style::default().fg(border_color);
    let fill_style = Style::default().bg(Color::Indexed(234));

    let cx = area.x + area.width / 2;
    let cy = area.y + area.height / 2;
    let rx = (area.width / 2).saturating_sub(1) as f64;
    let ry = (area.height / 2).saturating_sub(1) as f64;

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let dx = (x as f64 - cx as f64) / rx.max(1.0);
            let dy = (y as f64 - cy as f64) / ry.max(1.0);
            let dist = dx * dx + dy * dy;

            if dist <= 1.0 {
                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_style(fill_style);
            }
        }
    }

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let dx = (x as f64 - cx as f64) / rx.max(1.0);
            let dy = (y as f64 - cy as f64) / ry.max(1.0);
            let dist = dx * dx + dy * dy;

            if dist > 0.6 && dist <= 1.1 {
                let cell = buf.cell_mut((x, y)).unwrap();
                let ch = if dist > 0.95 {
                    if dy.abs() > dx.abs() * 0.5 {
                        if dy < 0.0 {
                            '~'
                        } else {
                            '_'
                        }
                    } else if dx < 0.0 {
                        '('
                    } else {
                        ')'
                    }
                } else {
                    ' '
                };
                cell.set_char(ch);
                cell.set_style(border_style);
            }
        }
    }

    if !label.is_empty() && area.height >= 1 && area.width > label.len() as u16 {
        let label_x = cx.saturating_sub(label.len() as u16 / 2);
        let label_style = Style::default()
            .fg(Color::Indexed(231))
            .add_modifier(Modifier::BOLD);
        buf.set_string(label_x, cy, label, label_style);
    }
}

fn render_bars(
    ctx: &NodeRenderContext,
    label: &str,
    items: &[(String, f64, Color)],
    buf: &mut Buffer,
) {
    let border_style = if ctx.selected {
        Style::default()
            .fg(Color::Indexed(80))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(133))
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(label)
        .style(Style::default().bg(Color::Indexed(234)));
    let inner = block.inner(ctx.area);
    block.render(ctx.area, buf);

    if inner.width < 8 || inner.height < 1 {
        return;
    }

    let max_label_len = items.iter().map(|(l, _, _)| l.len()).max().unwrap_or(0);
    let bar_start = inner.x + max_label_len as u16 + 1;
    let bar_width = inner.width.saturating_sub(max_label_len as u16 + 1);

    for (i, (item_label, value, color)) in items.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }
        buf.set_string(
            inner.x,
            y,
            item_label,
            Style::default().fg(Color::Indexed(248)),
        );
        let filled = ((bar_width as f64 * value).round() as u16).min(bar_width);
        for x in 0..bar_width {
            if let Some(cell) = buf.cell_mut((bar_start + x, y)) {
                if x < filled {
                    cell.set_char('#');
                    cell.set_style(Style::default().fg(*color));
                } else {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(Color::Indexed(236)));
                }
            }
        }
    }
}

// ============================================================================
// Edge types
// ============================================================================

/// Edge type with step and straight path variants.
#[derive(Clone, Debug)]
pub enum OverviewEdge {
    Step {
        style: EdgeStyle,
        selected_style: EdgeStyle,
    },
    Straight {
        style: EdgeStyle,
        selected_style: EdgeStyle,
    },
}

impl Default for OverviewEdge {
    fn default() -> Self {
        OverviewEdge::Step {
            style: EdgeStyle::default(),
            selected_style: EdgeStyle::default(),
        }
    }
}

impl EdgeContent for OverviewEdge {
    fn compute_path(&self, ctx: &EdgePathContext) -> Path {
        match self {
            OverviewEdge::Step { .. } => compute_step_path(
                ctx.from,
                ctx.to,
                ctx.source_position,
                ctx.target_position,
                3.0,
            ),
            OverviewEdge::Straight { .. } => {
                compute_straight_path(ctx.from, ctx.to, ctx.source_position, ctx.target_position)
            }
        }
    }

    fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
        let style = match self {
            OverviewEdge::Step {
                style,
                selected_style,
            }
            | OverviewEdge::Straight {
                style,
                selected_style,
            } => {
                if ctx.selected {
                    selected_style
                } else {
                    style
                }
            }
        };

        let label = ctx.label.map(Text::raw);
        ctx.render_path(style, label.as_ref(), buf);
    }
}

// ============================================================================
// Graph construction
// ============================================================================

fn create_flow() -> Flow<OverviewNode, OverviewEdge> {
    let nodes = vec![
        // A: Text node — built-in Paragraph rendering
        Node::new(
            "A",
            (-30.0, 10.0),
            (18.0, 5.0),
            OverviewNode::Text(TextContent::from("ratatui Paragraph\nbuiltin")),
        )
        .with_handles(vec![Handle::source(HandlePosition::Right).with_style(
            HandleStyle::new('◉', Style::default().fg(Color::Indexed(71))),
        )])
        .with_selected(true),
        // B: Sparkline — ratatui widget inside a node
        Node::new(
            "B",
            (20.0, 38.0),
            (26.0, 6.0),
            OverviewNode::Sparkline {
                label: "ratatui Sparkline".to_string(),
                data: vec![
                    3, 7, 2, 9, 4, 8, 1, 6, 5, 8, 3, 7, 2, 5, 9, 4, 6, 3, 8, 5, 4, 7, 2, 8, 5, 9,
                    3, 6, 4, 7, 2, 8, 5, 9, 3, 6, 4, 7, 2, 8,
                ],
            },
        )
        .with_handles(vec![
            Handle::target(HandlePosition::Left).with_style(HandleStyle::new(
                '◆',
                Style::default().fg(Color::Indexed(80)),
            )),
            Handle::source(HandlePosition::Right)
                .with_id("right")
                .with_style(HandleStyle::new(
                    '◇',
                    Style::default().fg(Color::Indexed(179)),
                )),
        ]),
        // C: Group parent — transparent container for hierarchy demo
        Node::new(
            "C",
            (30.0, 0.0),
            (35.0, 18.0),
            OverviewNode::Text(TextContent::from("Parent node")),
        )
        .with_opaque(false)
        .with_handles(vec![Handle::target(HandlePosition::Left).with_offset(0.9)]),
        // D: Child of C — constrained to parent bounds
        Node::new(
            "D",
            (5.0, 5.0),
            (15.0, 6.0),
            OverviewNode::Text(TextContent::from("Nested child\n(drag me)")),
        )
        .with_parent("C")
        .with_extent(NodeExtent::Parent)
        .with_handles(vec![
            Handle::target(HandlePosition::Left),
            Handle::source(HandlePosition::Right)
                .with_id("out1")
                .with_offset(0.3),
            Handle::source(HandlePosition::Right)
                .with_id("out2")
                .with_offset(0.7),
        ]),
        // E: Raw buffer — custom oval shape via buf[(x, y)]
        Node::new(
            "E",
            (85.0, 0.0),
            (16.0, 8.0),
            OverviewNode::RawBuffer {
                label: "buf[(x, y)]".to_string(),
            },
        )
        .with_handles(vec![Handle::target(HandlePosition::Left)]),
        // F: Bar chart — custom NodeContent rendering
        Node::new(
            "F",
            (60.0, 32.0),
            (26.0, 7.0),
            OverviewNode::Bars {
                label: "NodeContent".to_string(),
                items: vec![
                    ("CPU".to_string(), 0.75, Color::Indexed(71)),
                    ("MEM".to_string(), 0.55, Color::Indexed(179)),
                    ("NET".to_string(), 0.28, Color::Indexed(80)),
                    ("GPU".to_string(), 0.92, Color::Indexed(167)),
                ],
            },
        )
        .with_handles(vec![Handle::target(HandlePosition::Left)]),
    ];

    let edges: Vec<Edge<OverviewEdge>> = vec![
        Edge::new("e_ab", "A", "B")
            .with_content(OverviewEdge::Straight {
                // Braille, matching `StraightEdge`'s own default — a straight edge
                // runs at whatever angle the nodes leave it at, and one character
                // per cell can only staircase that.
                style: EdgeStyle::braille()
                    .with_stroke_style(Style::default().fg(Color::Indexed(208)))
                    .with_marker_start(EdgeMarker::Custom('■'))
                    .with_marker_end(EdgeMarker::ArrowClosed),
                selected_style: EdgeStyle::braille(),
            })
            .with_animated(true)
            .with_label("EdgeContent trait\nstraight path"),
        Edge::new("e_ad", "A", "D"),
        Edge::new("e_de", "D", "E")
            .with_source_handle(Some("out1".to_string()))
            .with_animated(true),
        Edge::new("e_df", "D", "F")
            .with_source_handle(Some("out2".to_string()))
            .with_content(OverviewEdge::Step {
                // Dotted, to show the stroke is a choice per edge rather than a
                // property of the routing.
                style: EdgeStyle::dotted()
                    .with_stroke_style(Style::default().fg(Color::Indexed(114)))
                    .with_marker_end(EdgeMarker::Arrow),
                selected_style: EdgeStyle::dotted().with_marker_end(EdgeMarker::Arrow),
            }),
        Edge::new("e_bc", "B", "C").with_source_handle(Some("right".to_string())),
    ];

    Flow::with_graph(nodes, edges).expect("valid graph")
}

// ============================================================================
// Demo entry + OverviewDemo
// ============================================================================

pub fn entry_overview() -> DemoEntry {
    DemoEntry {
        demo: Box::new(OverviewDemo::new()),
        meta: rataflow_examples::meta::overview(),
    }
}

struct OverviewDemo {
    app: DemoApp<OverviewNode, OverviewEdge>,
}

impl OverviewDemo {
    fn new() -> Self {
        Self {
            app: DemoApp::from_flow(create_flow()),
        }
    }
}

impl Demo for OverviewDemo {
    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        let r = self.app.flow.handle_controls_key_event(event);
        if matches!(r, EventResponse::NotHandled) {
            self.app.flow.handle_key_event(event);
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        for e in self.app.flow.handle_mouse_event(event).into_events() {
            match e {
                FlowEvent::ConnectionCompleted(conn) => {
                    self.app
                        .flow
                        .add_edge_from_connection(conn, OverviewEdge::default());
                }
                FlowEvent::ReconnectionCompleted {
                    edge_id,
                    new_connection,
                    ..
                } => {
                    self.app.flow.reconnect_edge(&edge_id, new_connection);
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Background::new(&self.app.flow)
                .variant(BackgroundVariant::Dots)
                .gap(10, 5)
                .style(BackgroundStyle::default().with_pattern_color(Color::Indexed(24))),
            area,
        );
        frame.render_widget(&mut self.app.flow, area);
        frame.render_widget(Controls::new(&self.app.flow), area);
        frame.render_widget(MiniMap::new(&self.app.flow), area);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }

    fn tick(&mut self, elapsed_ms: f64) {
        let elapsed = Duration::from_millis(elapsed_ms as u64);
        self.app.flow.tick_animation(elapsed);
        self.app.flow.tick_auto_pan(elapsed);
    }
}
