use rataflow::{Edge, Flow, Handle, HandlePosition, HandleStyle, Node, StepEdge, TextContent};
use ratatui::style::{Color, Style};

pub fn create_flow() -> Flow<TextContent, StepEdge> {
    let nodes = vec![
        // Row 1: baseline pair
        Node::from_text("source", (5.0, 2.0), "Source").with_selected(true),
        Node::from_text("target", (25.0, 2.0), "Target"),
        // Row 2: one node per validation mechanism
        Node::from_text("rejected", (5.0, 10.0), "Rejected"),
        Node::from_text("no_outgoing", (25.0, 10.0), "No outgoing").with_handles(vec![
            Handle::source(HandlePosition::Right)
                .with_connectable_start(false)
                .with_style(HandleStyle::new(
                    '\u{25D0}',
                    Style::default().fg(Color::Indexed(133)),
                )),
            Handle::target(HandlePosition::Left),
        ]),
        Node::from_text("no_incoming", (47.0, 10.0), "No incoming").with_handles(vec![
            Handle::source(HandlePosition::Right),
            Handle::target(HandlePosition::Left)
                .with_connectable_end(false)
                .with_style(HandleStyle::new(
                    '\u{25D0}',
                    Style::default().fg(Color::Indexed(80)),
                )),
        ]),
    ];

    let edges: Vec<Edge<StepEdge>> = vec![Edge::new("e1", "source", "target")];

    let mut flow = Flow::with_graph(nodes, edges).expect("valid graph");
    flow.set_connection_validator(|conn| conn.target != "rejected");

    flow
}
