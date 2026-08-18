//! Built-in layout algorithm for automatic node positioning.
//!
//! Everything in this module requires the `sugiyama` feature. Computing positions
//! with your own algorithm needs nothing from here: apply the result with
//! [`Flow::set_node_positions`], which writes the whole batch and resolves the
//! hierarchy once.
//!
//! # Sugiyama layout (requires `sugiyama` feature)
//!
//! ```no_run
//! # use rataflow::{Edge, Flow, Node, StepEdge, Sugiyama};
//! # fn main() -> Result<(), rataflow::Error> {
//! # let nodes = vec![Node::from_text("a", (0.0, 0.0), "A")];
//! # let edges: Vec<Edge<StepEdge>> = vec![];
//! let mut flow = Flow::with_graph(nodes, edges)?;
//! flow.apply_layout(Sugiyama::vertical());
//! # Ok(())
//! # }
//! ```
//!
//! # Node dimensions and spacing
//!
//! Layout spacing is derived from **existing node dimensions**:
//!
//! - `apply_layout()` reads `max_width` and `max_height` from nodes in the graph
//! - Default spacing is direction-aware: gaps are proportional to the relevant dimension
//!   (e.g., horizontal gaps use width, vertical gaps use height)
//! - This means nodes must have dimensions set before calling `apply_layout()`
//! - Handle positions are set automatically based on layout direction
//!
//! `from_edges()` uses uniform dimensions (based on the longest label)
//! to ensure consistent spacing and straight edge alignment.

#[cfg(feature = "sugiyama")]
use std::collections::HashMap;

#[cfg(feature = "sugiyama")]
use crate::content::{EdgeContent, NodeContent};
#[cfg(feature = "sugiyama")]
use crate::state::Flow;
#[cfg(feature = "sugiyama")]
use crate::types::Position;

#[cfg(feature = "sugiyama")]
use crate::types::{Connection, Edge, HandlePosition, Node};

#[cfg(feature = "sugiyama")]
use crate::ui::{StepEdge, TextContent};

#[cfg(feature = "sugiyama")]
use crate::error::Error;

// ============================================================================
// Core API (always available)
// ============================================================================

// ============================================================================
// Sugiyama layout (feature-gated)
// ============================================================================

#[cfg(feature = "sugiyama")]
mod sugiyama {
    use super::*;
    use rust_sugiyama::from_edges;

    /// Direction of the layout flow.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum LayoutDirection {
        /// Nodes flow from top to bottom.
        #[default]
        TopToBottom,
        /// Nodes flow from left to right.
        LeftToRight,
        /// Nodes flow from bottom to top.
        BottomToTop,
        /// Nodes flow from right to left.
        RightToLeft,
    }

    impl LayoutDirection {
        pub(crate) fn source_position(self) -> HandlePosition {
            match self {
                LayoutDirection::TopToBottom => HandlePosition::Bottom,
                LayoutDirection::LeftToRight => HandlePosition::Right,
                LayoutDirection::BottomToTop => HandlePosition::Top,
                LayoutDirection::RightToLeft => HandlePosition::Left,
            }
        }

        pub(crate) fn target_position(self) -> HandlePosition {
            self.source_position().opposite()
        }
    }

    /// Sugiyama layout algorithm (layered graph drawing).
    ///
    /// Arranges nodes in layers to minimize edge crossings.
    /// Well-suited for DAGs like flowcharts and dependency graphs.
    ///
    /// # Spacing defaults
    ///
    /// When `node_spacing`, `rank_spacing`, or `margin` are `None` (the default),
    /// values are derived from existing node dimensions with direction-aware logic:
    ///
    /// - `node_spacing`: equals the dimension perpendicular to flow
    /// - `rank_spacing`: equals the dimension along the flow
    ///
    /// For example, in a horizontal layout with 8×3 nodes:
    /// `node_spacing` defaults to 3 (height-based), `rank_spacing` to 8 (width-based).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rataflow::Flow;
    /// use rataflow::Sugiyama;
    ///
    /// # let mut flow: Flow = Flow::new();
    /// // Auto spacing (derived from node dimensions)
    /// flow.apply_layout(Sugiyama::vertical());
    ///
    /// // Custom spacing
    /// flow.apply_layout(Sugiyama::vertical().with_node_spacing(10.0));
    /// ```
    #[derive(Debug, Clone, Default)]
    pub struct Sugiyama {
        /// Layout flow direction (default: top-to-bottom).
        pub direction: LayoutDirection,
        /// Gap between nodes in the same rank. None = auto (perpendicular dimension).
        pub node_spacing: Option<f64>,
        /// Gap between ranks. None = auto (flow dimension).
        pub rank_spacing: Option<f64>,
        /// Margin around the graph. None = auto (smaller node dimension).
        pub margin: Option<f64>,
        /// If true, nodes in `from_edges` are sized to fit their content.
        /// If false (default), all nodes use uniform dimensions for cleaner alignment.
        pub auto_size: bool,
    }

    impl Sugiyama {
        /// Creates a new layout with default settings (top-to-bottom).
        pub fn new() -> Self {
            Self::default()
        }

        /// Creates a top-to-bottom layout (same as `new`).
        pub fn vertical() -> Self {
            Self::default()
        }

        /// Creates a left-to-right layout.
        pub fn horizontal() -> Self {
            Self {
                direction: LayoutDirection::LeftToRight,
                ..Default::default()
            }
        }

        /// Sets the layout direction.
        pub fn with_direction(mut self, direction: LayoutDirection) -> Self {
            self.direction = direction;
            self
        }

        /// Sets the gap between nodes in the same rank.
        pub fn with_node_spacing(mut self, spacing: f64) -> Self {
            self.node_spacing = Some(spacing);
            self
        }

        /// Sets the gap between ranks.
        pub fn with_rank_spacing(mut self, spacing: f64) -> Self {
            self.rank_spacing = Some(spacing);
            self
        }

        /// Sets the margin around the graph.
        pub fn with_margin(mut self, margin: f64) -> Self {
            self.margin = Some(margin);
            self
        }

        /// Sizes each node to its label in `from_edges`.
        /// Default is uniform dimensions (longest label) for cleaner edges.
        pub fn with_auto_size(mut self) -> Self {
            self.auto_size = true;
            self
        }
    }

    impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
        /// Applies Sugiyama layout to position nodes.
        ///
        /// # Example
        ///
        /// ```no_run
        /// # use rataflow::Flow;
        /// use rataflow::Sugiyama;
        ///
        /// # let mut flow: Flow = Flow::new();
        /// flow.apply_layout(Sugiyama::vertical());
        /// ```
        pub fn apply_layout(&mut self, layout: Sugiyama) {
            if self.nodes.is_empty() {
                return;
            }

            let id_to_idx: HashMap<&str, usize> = self
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| (n.id(), i))
                .collect();

            let graph_edges: Vec<(u32, u32)> = self
                .edges
                .iter()
                .filter_map(|e| {
                    let source = *id_to_idx.get(e.source.as_str())?;
                    let target = *id_to_idx.get(e.target.as_str())?;
                    Some((source as u32, target as u32))
                })
                .collect();

            let max_width = self.nodes.iter().map(|n| n.node.width).fold(0.0, f64::max);
            let max_height = self.nodes.iter().map(|n| n.node.height).fold(0.0, f64::max);

            // Direction-aware dimensions:
            // - perpendicular: siblings arranged along this axis
            // - parallel: layers stacked along this axis
            let (perpendicular_dim, parallel_dim) = match layout.direction {
                LayoutDirection::TopToBottom | LayoutDirection::BottomToTop => {
                    (max_width, max_height)
                }
                LayoutDirection::LeftToRight | LayoutDirection::RightToLeft => {
                    (max_height, max_width)
                }
            };

            let node_spacing = layout.node_spacing.unwrap_or(perpendicular_dim);
            let rank_spacing = layout.rank_spacing.unwrap_or(parallel_dim);
            let margin = layout.margin.unwrap_or(max_width.min(max_height));

            // Center-to-center spacing
            let sibling_spacing = perpendicular_dim + node_spacing;
            let rank_spacing_total = parallel_dim + rank_spacing;

            let config = rust_sugiyama::configure::Config {
                vertex_spacing: 1.0,
                ..Default::default()
            };
            let layouts = from_edges(&graph_edges, &config);

            for (result, _, _) in &layouts {
                for (vertex_id, (x, y)) in result {
                    if *vertex_id >= self.nodes.len() {
                        continue;
                    }

                    // sugiyama x = sibling position, y = rank/layer
                    let (pos_x, pos_y) = match layout.direction {
                        LayoutDirection::TopToBottom => (
                            *x * sibling_spacing + margin,
                            *y * rank_spacing_total + margin,
                        ),
                        LayoutDirection::BottomToTop => (
                            *x * sibling_spacing + margin,
                            -*y * rank_spacing_total + margin,
                        ),
                        LayoutDirection::LeftToRight => (
                            *y * rank_spacing_total + margin,
                            *x * sibling_spacing + margin,
                        ),
                        LayoutDirection::RightToLeft => (
                            -*y * rank_spacing_total + margin,
                            *x * sibling_spacing + margin,
                        ),
                    };

                    let node = &mut self.nodes[*vertex_id];
                    node.node.position = Position::new(pos_x, pos_y);
                    node.node.source_position = layout.direction.source_position();
                    node.node.target_position = layout.direction.target_position();
                }
            }

            self.resolve_hierarchy();
        }
    }

    /// Conversion trait for edge tuples in [`Flow::from_edges`].
    ///
    /// Accepts `(&str, &str)` for unlabelled edges or `(&str, &str, &str)` for labelled edges,
    /// following the same pattern as petgraph's `IntoWeightedEdge`.
    pub trait IntoEdge<'a> {
        /// Converts into a `(source, target, optional_label)` tuple.
        fn into_edge(self) -> (&'a str, &'a str, Option<&'a str>);
    }

    impl<'a> IntoEdge<'a> for (&'a str, &'a str) {
        fn into_edge(self) -> (&'a str, &'a str, Option<&'a str>) {
            (self.0, self.1, None)
        }
    }

    impl<'a> IntoEdge<'a> for (&'a str, &'a str, &'a str) {
        fn into_edge(self) -> (&'a str, &'a str, Option<&'a str>) {
            (self.0, self.1, Some(self.2))
        }
    }

    impl Flow<TextContent, StepEdge> {
        /// Creates a graph from edge tuples with Sugiyama layout.
        ///
        /// Each tuple defines an edge between two nodes. Nodes are created automatically
        /// from the unique names, with the name used as both the node ID and display text.
        /// Nodes use [`TextContent`] and edges use [`StepEdge`] as defaults.
        ///
        /// Accepts `(&str, &str)` for unlabelled edges or `(&str, &str, &str)` to set
        /// edge labels inline.
        ///
        /// For custom content types, use [`Flow::with_graph`] + [`Flow::apply_layout`]
        /// instead.
        ///
        /// Handle positions are set automatically based on layout direction
        /// (e.g., vertical uses Bottom→Top).
        ///
        /// By default, all nodes use **uniform dimensions** (longest name width)
        /// for clean edge alignment. Use `.with_auto_size()` for per-node sizing.
        ///
        /// # Arguments
        ///
        /// * `edges` - Slice of `(source, target)` or `(source, target, label)` tuples
        /// * `layout` - Sugiyama layout configuration
        ///
        /// # Example
        ///
        /// ```no_run
        /// use rataflow::{Flow, Sugiyama};
        ///
        /// # fn main() -> Result<(), rataflow::Error> {
        /// // Unlabelled edges
        /// let flow: Flow = Flow::from_edges(
        ///     &[("Start", "Process"), ("Process", "End")],
        ///     Sugiyama::vertical(),
        /// )?;
        ///
        /// // Labelled edges
        /// let flow: Flow = Flow::from_edges(
        ///     &[("Start", "Process", "step 1"), ("Process", "End", "step 2")],
        ///     Sugiyama::vertical(),
        /// )?;
        /// # Ok(())
        /// # }
        /// ```
        pub fn from_edges<'a, D: IntoEdge<'a> + Copy>(
            edges: &[D],
            layout: Sugiyama,
        ) -> Result<Self, Error> {
            let mut labels: Vec<&str> = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for edge in edges {
                let (source, target, _) = (*edge).into_edge();
                if seen.insert(source) {
                    labels.push(source);
                }
                if seen.insert(target) {
                    labels.push(target);
                }
            }

            let nodes: Vec<Node<TextContent>> = if layout.auto_size {
                // Each node sized to its own content
                labels
                    .iter()
                    .map(|label| Node::from_text(*label, (0.0, 0.0), *label))
                    .collect()
            } else {
                // Uniform dimensions from longest label
                let max_width = labels.iter().map(|l| l.len()).max().unwrap_or(0) + 2;
                let height = 3;
                labels
                    .iter()
                    .map(|label| {
                        Node::new(
                            *label,
                            (0.0, 0.0),
                            (max_width as f64, height as f64),
                            TextContent::from(*label),
                        )
                    })
                    .collect()
            };

            let flow_edges: Vec<Edge<StepEdge>> = edges
                .iter()
                .map(|edge| {
                    let (src, tgt, label) = (*edge).into_edge();
                    let conn = Connection::new(src, None, tgt, None);
                    let mut e = Edge::new(conn.edge_id(), src, tgt);
                    if let Some(l) = label {
                        e = e.with_label(l);
                    }
                    e
                })
                .collect();

            let mut state = Self::with_graph(nodes, flow_edges)?;
            state.apply_layout(layout);
            Ok(state)
        }
    }
}

#[cfg(feature = "sugiyama")]
pub use sugiyama::{IntoEdge, LayoutDirection, Sugiyama};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Node;
    use crate::ui::TextContent;

    #[cfg(feature = "sugiyama")]
    mod sugiyama_tests {
        use super::*;
        use crate::layout::Sugiyama;
        use crate::types::{Edge, HandlePosition};
        use crate::ui::StepEdge;

        #[test]
        fn vertical_layout_sets_bottom_top_handles() {
            let nodes = vec![
                Node::new("a", (0.0, 0.0), (10.0, 3.0), TextContent::from("A")),
                Node::new("b", (0.0, 0.0), (10.0, 3.0), TextContent::from("B")),
            ];
            let edges: Vec<Edge<StepEdge>> = vec![Edge::new("e1", "a", "b")];
            let mut flow = Flow::with_graph(nodes, edges).unwrap();
            flow.apply_layout(Sugiyama::vertical());

            let a = flow.node("a").unwrap();
            let b = flow.node("b").unwrap();
            assert_eq!(a.source_position, HandlePosition::Bottom);
            assert_eq!(a.target_position, HandlePosition::Top);
            assert_eq!(b.source_position, HandlePosition::Bottom);
            assert_eq!(b.target_position, HandlePosition::Top);
        }

        #[test]
        fn horizontal_layout_sets_right_left_handles() {
            let nodes = vec![
                Node::new("a", (0.0, 0.0), (10.0, 3.0), TextContent::from("A")),
                Node::new("b", (0.0, 0.0), (10.0, 3.0), TextContent::from("B")),
            ];
            let edges: Vec<Edge<StepEdge>> = vec![Edge::new("e1", "a", "b")];
            let mut flow = Flow::with_graph(nodes, edges).unwrap();
            flow.apply_layout(Sugiyama::horizontal());

            let a = flow.node("a").unwrap();
            assert_eq!(a.source_position, HandlePosition::Right);
            assert_eq!(a.target_position, HandlePosition::Left);
        }

        #[test]
        fn layout_positions_nodes_distinctly() {
            let nodes = vec![
                Node::new("a", (0.0, 0.0), (10.0, 3.0), TextContent::from("A")),
                Node::new("b", (0.0, 0.0), (10.0, 3.0), TextContent::from("B")),
                Node::new("c", (0.0, 0.0), (10.0, 3.0), TextContent::from("C")),
            ];
            let edges: Vec<Edge<StepEdge>> =
                vec![Edge::new("e1", "a", "b"), Edge::new("e2", "a", "c")];
            let mut flow = Flow::with_graph(nodes, edges).unwrap();
            flow.apply_layout(Sugiyama::vertical());

            let a = flow.node("a").unwrap().position;
            let b = flow.node("b").unwrap().position;
            let c = flow.node("c").unwrap().position;

            // All three should have distinct positions
            assert!(
                a != b && a != c && b != c,
                "Nodes should have distinct positions: a={a:?}, b={b:?}, c={c:?}"
            );

            // "a" is the root, "b" and "c" are children → a should be in an earlier rank
            assert!(
                a.y < b.y,
                "Root should be above children in vertical layout"
            );
            assert!(
                a.y < c.y,
                "Root should be above children in vertical layout"
            );
        }

        #[test]
        fn from_edges_deduplicates_nodes() {
            let flow =
                Flow::from_edges(&[("A", "B"), ("B", "C"), ("A", "C")], Sugiyama::vertical())
                    .unwrap();

            // 3 unique nodes despite "A", "B", "C" appearing multiple times in edges
            assert_eq!(flow.nodes().count(), 3);
            assert!(flow.node("A").is_some());
            assert!(flow.node("B").is_some());
            assert!(flow.node("C").is_some());
        }

        #[test]
        fn from_edges_with_labels() {
            let flow = Flow::from_edges(
                &[("A", "B", "label1"), ("B", "C", "label2")],
                Sugiyama::vertical(),
            )
            .unwrap();

            assert_eq!(flow.edges().len(), 2);
            assert_eq!(flow.edges()[0].label.as_deref(), Some("label1"));
            assert_eq!(flow.edges()[1].label.as_deref(), Some("label2"));
        }
    }
}
