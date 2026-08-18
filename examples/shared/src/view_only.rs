use rataflow::{Flow, StepEdge, Sugiyama, TextContent};

pub fn create_flow() -> Flow<TextContent, StepEdge> {
    let mut flow = Flow::from_edges(
        &[
            ("Start", "Branch A"),
            ("Start", "Branch B"),
            ("Branch A", "Merge"),
            ("Branch B", "Merge"),
            ("Merge", "End"),
        ],
        Sugiyama::vertical(),
    )
    .expect("valid graph");

    // Hide handles — read-only graphs don't need visible connection points
    let ids: Vec<_> = flow.nodes().map(|n| n.id.clone()).collect();
    for id in &ids {
        flow.set_handles_hidden(id, true);
    }

    flow
}
