use rataflow::{
    Edge, Flow, Handle, HandlePosition, HandleStyle, Node, NodeContent, NodeRenderContext,
    StepEdge, TextContent,
};
use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        Block, Borders, Paragraph, Widget,
        canvas::{Canvas, Circle, Rectangle},
    },
};

/// Enum-based node type — each variant demonstrates a different rendering approach.
#[derive(Clone, Debug)]
pub enum MyNode {
    /// Uses built-in TextContent from rataflow with builder-style customization.
    TCFlow(TextContent),
    /// Uses ratatui Paragraph widget with styled lines.
    RichText { title: String, status: Status },
    /// Renders shapes on a ratatui Canvas widget using Braille markers.
    CanvasRatatui { label: String },
    /// Raw buffer manipulation — draws a decorative diamond pattern.
    Diamond { label: String, symbol: char },
}

#[derive(Clone, Debug)]
pub enum Status {
    Online,
    Busy,
    Offline,
}

impl NodeContent for MyNode {
    fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {
        match self {
            MyNode::TCFlow(text) => text.render(ctx, buf),
            MyNode::RichText { title, status } => render_paragraph(ctx, title, status, buf),
            MyNode::CanvasRatatui { label } => render_canvas(ctx, label, buf),
            MyNode::Diamond { label, symbol } => render_diamond(ctx, label, *symbol, buf),
        }
    }
}

// --- Node B: Paragraph widget ---

fn render_paragraph(ctx: &NodeRenderContext, title: &str, status: &Status, buf: &mut Buffer) {
    let border_style = if ctx.selected {
        Style::default()
            .fg(Color::Indexed(114))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(231))
    };

    // Build styled lines for the paragraph
    let (indicator, color, status_label) = match status {
        Status::Online => ("●", Color::Indexed(71), "Online"),
        Status::Busy => ("●", Color::Indexed(179), "Busy"),
        Status::Offline => ("○", Color::Indexed(167), "Offline"),
    };

    let lines = vec![Line::from(vec![
        Span::styled(indicator, Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(status_label, Style::default().fg(Color::Indexed(248))),
    ])];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title),
    );

    paragraph.render(ctx.area, buf);
}

// --- Node C: Canvas with Braille shapes ---

fn render_canvas(ctx: &NodeRenderContext, label: &str, buf: &mut Buffer) {
    let color = if ctx.selected {
        Color::Indexed(80)
    } else {
        Color::Indexed(133)
    };

    let label_style = if ctx.selected {
        Style::default()
            .fg(Color::Indexed(80))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(133))
    };

    // Title at top
    let title_line = Line::from(Span::styled(label, label_style));
    buf.set_line(ctx.area.x, ctx.area.y, &title_line, ctx.area.width);

    // Canvas area below title
    let canvas_area = ratatui::layout::Rect {
        x: ctx.area.x,
        y: ctx.area.y + 1,
        width: ctx.area.width,
        height: ctx.area.height.saturating_sub(1),
    };

    if canvas_area.height < 2 || canvas_area.width < 4 {
        return;
    }

    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .x_bounds([0.0, 20.0])
        .y_bounds([0.0, 20.0])
        .paint(|c| {
            // Outer circle
            c.draw(&Circle {
                x: 10.0,
                y: 10.0,
                radius: 8.0,
                color,
            });
            // Inner circle
            c.draw(&Circle {
                x: 10.0,
                y: 10.0,
                radius: 4.0,
                color,
            });
            // Cross lines using rectangles
            c.draw(&Rectangle {
                x: 9.0,
                y: 2.0,
                width: 2.0,
                height: 16.0,
                color,
            });
            c.draw(&Rectangle {
                x: 2.0,
                y: 9.0,
                width: 16.0,
                height: 2.0,
                color,
            });
        });

    canvas.render(canvas_area, buf);
}

// --- Node D: Raw buffer diamond pattern ---

fn render_diamond(ctx: &NodeRenderContext, label: &str, symbol: char, buf: &mut Buffer) {
    let (border_color, inner_color, symbol_color, label_color) = if ctx.selected {
        (
            Color::Indexed(232),
            Color::Indexed(80),
            Color::Indexed(232),
            Color::Indexed(80),
        )
    } else {
        (
            Color::Indexed(232),
            Color::Indexed(179),
            Color::Indexed(232),
            Color::Indexed(179),
        )
    };

    // Quadrant colors (top-left, top-right, bottom-left, bottom-right)
    let quadrant_colors = if ctx.selected {
        [
            Color::Indexed(242),
            Color::Indexed(242),
            Color::Indexed(242),
            Color::Indexed(242),
        ]
    } else {
        [
            Color::Indexed(167),
            Color::Indexed(71),
            Color::Indexed(75),
            Color::Indexed(133),
        ]
    };

    // Title at top
    let title_line = Line::from(Span::styled(
        label,
        Style::default()
            .fg(label_color)
            .add_modifier(Modifier::BOLD),
    ));
    buf.set_line(ctx.area.x, ctx.area.y, &title_line, ctx.area.width);

    // Diamond area below title
    let area = ratatui::layout::Rect {
        x: ctx.area.x,
        y: ctx.area.y + 1,
        width: ctx.area.width,
        height: ctx.area.height.saturating_sub(1),
    };

    let w = area.width as i32;
    let h = area.height as i32;
    let cx = w / 2;
    let cy = h / 2;

    // Fill each cell based on position relative to diamond
    for y in 0..h {
        for x in 0..w {
            let dx = (x - cx).abs() as f64;
            let dy = (y - cy).abs() as f64;
            let half_w = (w as f64) / 2.0;
            let half_h = (h as f64) / 2.0;
            let dist = dx / half_w + dy / half_h;

            let px = area.x + x as u16;
            let py = area.y + y as u16;

            if (0.7..=1.0).contains(&dist) {
                // Diamond outline
                buf[(px, py)]
                    .set_char('◆')
                    .set_fg(border_color)
                    .set_bg(inner_color);
            } else if dist < 0.7 {
                // Inside diamond
                buf[(px, py)].set_char(' ').set_bg(inner_color);
            } else {
                // Outside diamond - color by quadrant
                let quadrant = match (x < cx, y < cy) {
                    (true, true) => 0,   // top-left
                    (false, true) => 1,  // top-right
                    (true, false) => 2,  // bottom-left
                    (false, false) => 3, // bottom-right
                };
                buf[(px, py)]
                    .set_char(' ')
                    .set_bg(quadrant_colors[quadrant]);
            }
        }
    }

    // Center symbol
    let center_x = area.x + cx as u16;
    let center_y = area.y + cy as u16;
    if center_x < area.right() && center_y < area.bottom() {
        buf[(center_x, center_y)].set_char(symbol).set_style(
            Style::default()
                .fg(symbol_color)
                .bg(inner_color)
                .add_modifier(Modifier::BOLD),
        );
    }

    // Corner accents
    let corners = [
        (area.left(), area.top(), '╭', quadrant_colors[0]),
        (
            area.right().saturating_sub(1),
            area.top(),
            '╮',
            quadrant_colors[1],
        ),
        (
            area.left(),
            area.bottom().saturating_sub(1),
            '╰',
            quadrant_colors[2],
        ),
        (
            area.right().saturating_sub(1),
            area.bottom().saturating_sub(1),
            '╯',
            quadrant_colors[3],
        ),
    ];
    for (x, y, ch, bg) in corners {
        buf[(x, y)].set_char(ch).set_fg(border_color).set_bg(bg);
    }
}

pub fn create_flow() -> Flow<MyNode, StepEdge> {
    let nodes = vec![
        // Node A: TextContent with builder customization
        Node::new(
            "a",
            (8.0, 2.0),
            (24.0, 6.0),
            MyNode::TCFlow(
                TextContent::new("Built-in node type\nwith styling options")
                    .with_title("A - TextContent(Flow)")
                    .with_border_style(Style::default().fg(Color::Indexed(80)))
                    .with_text_style(Style::default().fg(Color::Indexed(231)))
                    .with_background(Color::Indexed(232))
                    .with_selected_border_style(
                        Style::default()
                            .fg(Color::Indexed(209))
                            .add_modifier(Modifier::BOLD),
                    )
                    .with_selected_text_style(
                        Style::default()
                            .fg(Color::Indexed(231))
                            .add_modifier(Modifier::BOLD),
                    )
                    .with_selected_background(Color::Indexed(236)),
            ),
        )
        .with_selected(true)
        .with_handles(vec![
            Handle::source(HandlePosition::Right).with_style(HandleStyle::new(
                '◉',
                Style::default().fg(Color::Indexed(231)),
            )),
            Handle::target(HandlePosition::Left).with_style(HandleStyle::new(
                '◎',
                Style::default().fg(Color::Indexed(248)),
            )),
        ]),
        // Node B: Paragraph with status
        Node::new(
            "b",
            (43.0, 2.0),
            (24.0, 6.0),
            MyNode::RichText {
                title: "B - Paragraph(Ratatui)".to_string(),
                status: Status::Online,
            },
        )
        .with_handles(vec![
            Handle::source(HandlePosition::Bottom).with_style(HandleStyle::new(
                '⌬',
                Style::default().fg(Color::Indexed(71)),
            )),
            Handle::target(HandlePosition::Left).with_style(HandleStyle::new(
                '⏣',
                Style::default().fg(Color::Indexed(71)),
            )),
        ]),
        // Node C: Canvas with Braille shapes
        Node::new(
            "c",
            (45.0, 14.0),
            (20.0, 8.0),
            MyNode::CanvasRatatui {
                label: "C - Canvas(Ratatui)".to_string(),
            },
        )
        .with_handles(vec![
            Handle::target(HandlePosition::Top)
                .with_id("top")
                .with_style(HandleStyle::new(
                    '⌑',
                    Style::default().fg(Color::Indexed(133)),
                )),
            Handle::target(HandlePosition::Left)
                .with_id("left")
                .with_style(HandleStyle::new(
                    '⎈',
                    Style::default().fg(Color::Indexed(133)),
                )),
        ]),
        // Node D: Diamond pattern
        Node::new(
            "d",
            (11.0, 14.0),
            (18.0, 8.0),
            MyNode::Diamond {
                label: "D - (Raw Buffer)".to_string(),
                symbol: '★',
            },
        )
        .with_handles(vec![
            Handle::source(HandlePosition::Right).with_style(HandleStyle::new(
                '▣',
                Style::default().fg(Color::Indexed(242)),
            )),
            Handle::target(HandlePosition::Left).with_style(HandleStyle::new(
                '▢',
                Style::default().fg(Color::Indexed(232)),
            )),
        ]),
    ];

    let edges: Vec<Edge<StepEdge>> = vec![
        Edge::new("e1", "a", "b"),
        Edge::new("e2", "b", "c").with_target_handle(Some("top".to_string())),
        Edge::new("e3", "d", "c").with_target_handle(Some("left".to_string())),
    ];

    Flow::with_graph(nodes, edges).expect("valid graph")
}
