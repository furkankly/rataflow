use std::collections::HashMap;

use rataflow::{Edge, Flow, Node, Position, StepEdge, TextContent};

pub fn build_children_map(edges: &[(usize, usize)]) -> HashMap<usize, Vec<usize>> {
    let mut map: HashMap<usize, Vec<usize>> = HashMap::new();
    for &(parent, child) in edges {
        map.entry(parent).or_default().push(child);
    }
    map
}

pub fn subtree_width(node: usize, children: &HashMap<usize, Vec<usize>>) -> f64 {
    match children.get(&node) {
        Some(kids) => kids
            .iter()
            .map(|&k| subtree_width(k, children))
            .sum::<f64>()
            .max(1.0),
        None => 1.0,
    }
}

pub fn layout_tree(
    node: usize,
    depth: usize,
    x_offset: f64,
    children: &HashMap<usize, Vec<usize>>,
    positions: &mut HashMap<usize, (f64, f64)>,
) {
    let width = subtree_width(node, children);
    let x = x_offset + width / 2.0;
    positions.insert(node, (x, depth as f64));

    if let Some(kids) = children.get(&node) {
        let mut child_offset = x_offset;
        for &kid in kids {
            layout_tree(kid, depth + 1, child_offset, children, positions);
            child_offset += subtree_width(kid, children);
        }
    }
}

pub fn compute_layout(
    graph_edges: &[(usize, usize)],
    node_width: f64,
    node_height: f64,
) -> HashMap<String, Position> {
    let children = build_children_map(graph_edges);
    let mut slot_positions: HashMap<usize, (f64, f64)> = HashMap::new();
    layout_tree(0, 0, 0.0, &children, &mut slot_positions);

    let x_spacing = node_width + 6.0;
    let y_spacing = node_height + 4.0;
    let margin = 5.0;

    slot_positions
        .into_iter()
        .map(|(idx, (x, y))| {
            (
                format!("node_{}", idx),
                (x * x_spacing + margin, y * y_spacing + margin).into(),
            )
        })
        .collect()
}

pub fn create_flow() -> Flow<TextContent, StepEdge> {
    let graph_edges: Vec<(usize, usize)> = vec![
        (0, 1),
        (0, 2),
        (1, 3),
        (1, 4),
        (2, 5),
        (2, 6),
        (3, 7),
        (3, 8),
    ];

    let labels = [
        "Start",
        "Process 1",
        "Process 2",
        "Process 3",
        "Process 4",
        "Process 5",
        "Process 6",
        "Process 7",
        "Process 8",
    ];

    let node_width = 14.0;
    let node_height = 5.0;

    let nodes: Vec<Node<TextContent>> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            Node::new(
                format!("node_{}", i),
                (0.0, 0.0),
                (node_width, node_height),
                TextContent::from(*label),
            )
        })
        .collect();

    let edges: Vec<Edge<StepEdge>> = graph_edges
        .iter()
        .enumerate()
        .map(|(i, (source, target))| {
            Edge::new(
                format!("edge_{}", i),
                format!("node_{}", source),
                format!("node_{}", target),
            )
        })
        .collect();

    Flow::with_graph(nodes, edges).expect("valid graph")
}
