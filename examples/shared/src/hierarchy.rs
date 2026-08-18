use rataflow::{Edge, Flow, Handle, HandlePosition, Node, NodeExtent, StepEdge, TextContent};

pub fn create_flow() -> Flow<TextContent, StepEdge> {
    let group_content = TextContent::from("Group")
        .with_background(None)
        .with_selected_background(None);
    let group = Node::new("group", (3.0, 2.0), (55.0, 22.0), group_content).with_opaque(false);

    let nested_content = TextContent::from("Nested\n(drag me)")
        .with_background(None)
        .with_selected_background(None);
    let nested = Node::new("nested", (3.0, 3.0), (18.0, 13.0), nested_content)
        .with_opaque(false)
        .with_parent("group")
        .with_expand_parent(true);

    let overflows = Node::new(
        "overflows",
        (2.0, 1.0),
        (14.0, 5.0),
        TextContent::from("Overflows\n(drag me)"),
    )
    .with_parent("nested")
    .with_expand_parent(true);

    let bounded = Node::new(
        "bounded",
        (35.0, 4.0),
        (14.0, 5.0),
        TextContent::from("Bounded\n(drag me)"),
    )
    .with_parent("group")
    .with_extent(NodeExtent::Parent)
    .with_handles(vec![
        Handle::target(HandlePosition::Left),
        Handle::source(HandlePosition::Bottom),
    ]);

    let regular = Node::new(
        "regular",
        (33.0, 15.0),
        (18.0, 5.0),
        TextContent::from("Regular child\n(drag me)"),
    )
    .with_parent("group")
    .with_handles(vec![
        Handle::target(HandlePosition::Top),
        Handle::source(HandlePosition::Right),
    ]);

    let standalone = Node::new(
        "standalone",
        (66.0, 4.0),
        (14.0, 5.0),
        TextContent::from("Standalone"),
    );

    let nodes = vec![group, nested, bounded, regular, overflows, standalone];

    let edges: Vec<Edge<StepEdge>> = vec![
        Edge::new("e1", "nested", "bounded"),
        Edge::new("e2", "bounded", "regular"),
        Edge::new("e3", "regular", "standalone"),
        Edge::new("e4", "overflows", "bounded"),
    ];

    Flow::with_graph(nodes, edges).expect("valid graph")
}
