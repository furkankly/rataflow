use ratatui::buffer::Buffer;
use ratatui::text::Text;

use rataflow::{
    Edge, EdgeContent, EdgePathContext, EdgeRenderContext, EdgeStyle, FloatingAttachment,
    FloatingEdge, Flow, Node, Path, StepEdge, TextContent,
};

/// Where an edge decides to attach, as one type so a key can cycle the choices.
///
/// Attachment is a property of the edge type, not a mode on the graph — which is
/// why this is an enum over contents rather than a flag somewhere.
///
/// The middle two are the pair worth watching, and they are the same built-in
/// with one field changed: route and attachment are separate settings, so a
/// straight edge can snap and a stepped one can slide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Attach {
    /// [`FloatingEdge`] routed orthogonally — the built-in default.
    Stepped,
    /// [`FloatingEdge`] as a direct line, which defaults to attaching wherever the
    /// line between the two centers crosses the outline.
    Straight,
    /// The same straight edge pinned back to midpoints with
    /// [`FloatingAttachment::Midpoint`](rataflow::FloatingAttachment::Midpoint).
    StraightSnapped,
    /// Uses the handles the nodes declare. On default nodes that is right to left,
    /// whatever direction the target actually lies in.
    Pinned,
}

impl Attach {
    pub const CYCLE: [Attach; 4] = [
        Attach::Stepped,
        Attach::Straight,
        Attach::StraightSnapped,
        Attach::Pinned,
    ];

    pub fn next(self) -> Self {
        let i = Self::CYCLE.iter().position(|a| *a == self).unwrap_or(0);
        Self::CYCLE[(i + 1) % Self::CYCLE.len()]
    }

    pub fn label(self) -> &'static str {
        match self {
            Attach::Stepped => "follows the node, turning at right angles",
            Attach::Straight => "follows the node, sliding along the side as it moves",
            Attach::StraightSnapped => "the same edge, snapped to the middle of a side",
            Attach::Pinned => "pinned to the handles: always right to left",
        }
    }
}

/// Dispatches to whichever attachment the example is currently showing.
#[derive(Debug, Clone)]
pub struct DemoEdge {
    pub attach: Attach,
}

impl Default for DemoEdge {
    fn default() -> Self {
        Self {
            attach: Attach::Stepped,
        }
    }
}

impl EdgeContent for DemoEdge {
    fn compute_path(&self, ctx: &EdgePathContext) -> Path {
        match self.attach {
            Attach::Stepped => FloatingEdge::default().compute_path(ctx),
            Attach::Straight => FloatingEdge::straight().compute_path(ctx),
            // Route and attachment are separate settings, so the snapping variant is
            // the line above with one field changed.
            Attach::StraightSnapped => FloatingEdge::straight()
                .with_attachment(FloatingAttachment::Midpoint)
                .compute_path(ctx),
            Attach::Pinned => StepEdge::default().compute_path(ctx),
        }
    }

    fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
        // Braille for the straight routes, characters for the stepped ones — a
        // diagonal needs sub-cell resolution, an orthogonal run does not.
        let style = match self.attach {
            Attach::Straight | Attach::StraightSnapped => EdgeStyle::braille(),
            Attach::Stepped | Attach::Pinned => EdgeStyle::default(),
        };
        let label = ctx.label.map(Text::raw);
        ctx.render_path(&style, label.as_ref(), buf);
    }
}

/// A hub ringed by satellites, one edge from the hub to each.
///
/// The nodes declare nothing special — no handles on the relevant sides, no flag on
/// the edges. Attachment is computed from the node rectangles alone.
pub fn create_flow() -> Flow<TextContent, DemoEdge> {
    // Offset across each axis rather than straight out from the hub. On a perfectly
    // axis-aligned ray the outline is met exactly at the side's midpoint, so the two
    // floating modes would draw the same thing and the comparison would be invisible
    // until something moved.
    let satellites = [
        ("north", "drag me", (56.0, 2.0)),
        ("east", "or me", (76.0, 28.0)),
        ("south", "or me", (24.0, 34.0)),
        ("west", "or me", (4.0, 8.0)),
    ];

    let mut nodes = vec![Node::from_text("hub", (38.0, 17.0), "hub")];
    let mut edges = Vec::new();

    for (id, label, position) in satellites {
        nodes.push(Node::from_text(id, position, label));
        edges.push(Edge::new(format!("e_{id}"), "hub", id));
    }

    Flow::with_graph(nodes, edges).expect("valid graph")
}

/// Switches every edge to the given attachment.
pub fn set_attach(flow: &mut Flow<TextContent, DemoEdge>, attach: Attach) {
    let ids: Vec<String> = flow.edges().iter().map(|e| e.id.clone()).collect();
    for id in &ids {
        if let Some(content) = flow.edge_content_mut(id) {
            content.attach = attach;
        }
    }
}
