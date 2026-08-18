use rataflow::{Edge, Flow, Node, StepEdge, TextContent};

/// 3-node chain demonstrating animated edges.
pub fn animating_edges() -> Flow<TextContent, StepEdge> {
    let nodes = vec![
        Node::from_text("a", (5.0, 2.0), "Node A").with_selected(true),
        Node::from_text("b", (30.0, 2.0), "Node B"),
        Node::from_text("c", (55.0, 2.0), "Node C"),
    ];

    let edges: Vec<Edge<StepEdge>> = vec![
        Edge::new("e1", "a", "b")
            .with_animated(true)
            .with_label("animated"),
        Edge::new("e2", "b", "c"),
    ];

    Flow::with_graph(nodes, edges)
        .expect("valid graph")
        .with_animation_speed(80)
}
