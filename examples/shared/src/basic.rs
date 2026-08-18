use rataflow::{Flow, StepEdge, Sugiyama, TextContent};

/// 4-node horizontal chain: Node A → Node B → Node C → Node D.
pub fn basic() -> Flow<TextContent, StepEdge> {
    Flow::from_edges(
        &[
            ("Node A", "Node B", "step 1"),
            ("Node B", "Node C", "step 2"),
            ("Node C", "Node D", "step 3"),
        ],
        Sugiyama::horizontal(),
    )
    .expect("valid graph")
}
