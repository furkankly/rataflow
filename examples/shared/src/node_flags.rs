use rataflow::{Edge, Flow, Node, StepEdge, TextContent};

/// Compute a display label from a node's current flags.
fn flag_label(n: &Node<TextContent>) -> String {
    let mut parts = Vec::new();
    if !n.draggable {
        parts.push("!draggable".to_string());
    }
    if !n.selectable {
        parts.push("!selectable".to_string());
    }
    if !n.deletable {
        parts.push("!deletable".to_string());
    }
    if !n.connectable {
        parts.push("!connectable".to_string());
    }
    if n.hidden {
        parts.push("hidden".to_string());
    }
    if n.resizable {
        parts.push("resizable".to_string());
    }
    if n.z_index != 0 {
        parts.push(format!("z-index: {}", n.z_index));
    }
    if parts.is_empty() {
        "Default".to_string()
    } else {
        parts.join("\n")
    }
}

/// Update a node's text and dimensions to reflect its current flags.
pub fn update_flag_label(flow: &mut Flow<TextContent, StepEdge>, id: &str) {
    let label = match flow.node(id) {
        Some(n) => flag_label(n),
        None => return,
    };
    let width = label.lines().map(|l| l.len()).max().unwrap_or(0) as f64 + 2.0;
    let height = label.lines().count() as f64 + 2.0;
    if let Some(content) = flow.node_content_mut(id) {
        content.text = label.into();
    }
    flow.set_node_dimensions(id, width, height);
}

pub fn create_flow() -> Flow<TextContent, StepEdge> {
    let nodes = vec![
        Node::from_text("default", (5.0, 2.0), "Default").with_selected(true),
        Node::from_text("no_drag", (25.0, 2.0), "!draggable").with_draggable(false),
        Node::from_text("no_select", (47.0, 2.0), "!selectable").with_selectable(false),
        Node::from_text("no_delete", (5.0, 10.0), "!deletable").with_deletable(false),
        Node::from_text("no_connect", (25.0, 10.0), "!connectable").with_connectable(false),
        Node::from_text("hidden", (47.0, 10.0), "hidden").with_hidden(true),
        Node::from_text("z_high", (40.0, 4.0), "z-index: 5").with_z_index(5),
        // Drag the ◢ grip at its bottom-right. Resizing is opt-in per node, so a
        // node without this flag draws no grip and ignores the drag entirely.
        Node::from_text("resizable", (25.0, 18.0), "resizable").with_resizable(true),
    ];

    let edges: Vec<Edge<StepEdge>> = vec![
        Edge::new("e1", "default", "no_drag"),
        Edge::new("e2", "no_drag", "no_select"),
        Edge::new("e3", "no_delete", "no_connect"),
        Edge::new("e4", "default", "no_delete"),
        Edge::new("e5", "no_connect", "resizable"),
    ];

    Flow::with_graph(nodes, edges).expect("valid graph")
}
