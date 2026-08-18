use rataflow::{Edge, Flow, Node, Reconnectable, StepEdge, TextContent};

/// Graph demonstrating edge reconnection modes.
///
/// Three pairs of nodes, each connected by an edge with a different
/// `Reconnectable` setting:
/// - Both: drag either end to rewire
/// - Target only: only the target end can be moved
/// - None: cannot be reconnected
pub fn create_flow() -> Flow<TextContent, StepEdge> {
    let nodes = vec![
        Node::from_text("a1", (0.0, 0.0), "Both src"),
        Node::from_text("a2", (30.0, 0.0), "Both dst"),
        Node::from_text("b1", (0.0, 10.0), "Target src"),
        Node::from_text("b2", (30.0, 10.0), "Target dst"),
        Node::from_text("c1", (0.0, 20.0), "None src"),
        Node::from_text("c2", (30.0, 20.0), "None dst"),
    ];

    let edges: Vec<Edge<StepEdge>> = vec![
        Edge::new("e1", "a1", "a2")
            .with_reconnectable(Reconnectable::Both)
            .with_label("Both"),
        Edge::new("e2", "b1", "b2")
            .with_reconnectable(Reconnectable::Target)
            .with_label("Target only"),
        Edge::new("e3", "c1", "c2")
            .with_reconnectable(Reconnectable::None)
            .with_label("None"),
    ];

    Flow::with_graph(nodes, edges).expect("valid graph")
}
