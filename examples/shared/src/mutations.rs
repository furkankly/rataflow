use rataflow::{Edge, Flow, Handle, HandlePosition, Node, StepEdge, TextContent};

pub fn create_flow() -> Flow<TextContent, StepEdge> {
    let nodes = vec![
        Node::from_text("A", (20.0, 2.0), "Node A")
            .with_selected(true)
            .with_handles(vec![Handle::source(HandlePosition::Bottom)]),
        Node::from_text("B", (5.0, 10.0), "Node B").with_handles(vec![
            Handle::target(HandlePosition::Top),
            Handle::source(HandlePosition::Bottom),
        ]),
        Node::from_text("C", (35.0, 10.0), "Node C").with_handles(vec![
            Handle::target(HandlePosition::Top),
            Handle::source(HandlePosition::Bottom),
        ]),
        Node::from_text("D", (20.0, 18.0), "Node D")
            .with_handles(vec![Handle::target(HandlePosition::Top)]),
    ];

    let edges: Vec<Edge<StepEdge>> = vec![
        Edge::new("e1", "A", "B"),
        Edge::new("e2", "A", "C"),
        Edge::new("e3", "B", "D"),
        Edge::new("e4", "C", "D"),
    ];

    Flow::with_graph(nodes, edges)
        .expect("valid graph")
        .with_node_drag_threshold(3.0)
        .with_animation_speed(80)
}
