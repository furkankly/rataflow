#[cfg(feature = "serde")]
pub use inner::*;

#[cfg(feature = "serde")]
mod inner {
    use rataflow::{Flow, FlowSnapshot, StepEdge, TextContent};

    /// Serialize the current flow to a JSON string.
    pub fn save(flow: &Flow<TextContent, StepEdge>) -> String {
        let snapshot = flow.to_snapshot();
        serde_json::to_string(&snapshot).expect("serialization should not fail")
    }

    /// Deserialize a JSON string back into a flow.
    pub fn restore(json: &str) -> Option<Flow<TextContent, StepEdge>> {
        let snapshot: FlowSnapshot<TextContent, StepEdge> = serde_json::from_str(json).ok()?;
        Flow::from_snapshot(snapshot).ok()
    }

    /// Pretty-print a JSON string for display.
    pub fn pretty_json(json: &str) -> String {
        serde_json::from_str::<serde_json::Value>(json)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_else(|| json.to_string())
    }
}
