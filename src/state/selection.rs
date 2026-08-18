//! Selection operations for Flow.

use super::Flow;
use crate::content::{EdgeContent, NodeContent};
use crate::types::{Edge, Node, Position};

/// Direction for spatial node navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Directional bias weight for spatial navigation scoring.
///
/// Controls how strongly direction matters vs. proximity.
/// At k=2, a node at 45° off-axis scores 2× its raw distance, while a
/// perfectly aligned node scores 1×. Higher values make navigation more
/// strictly directional; lower values favor proximity.
const DIRECTION_BIAS: f64 = 2.0;

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    /// Clears the current selection (deselects all nodes and edges).
    pub fn clear_selection(&mut self) {
        for node in &mut self.nodes {
            node.node.selected = false;
        }
        for edge in &mut self.edges {
            edge.selected = false;
        }
        self.invalidate_z_order();
    }

    /// Returns true if any node is selected.
    pub fn has_selected_nodes(&self) -> bool {
        self.nodes.iter().any(|n| n.node.selected)
    }

    /// Returns true if any edge is selected.
    pub fn has_selected_edges(&self) -> bool {
        self.edges.iter().any(|e| e.selected)
    }

    // ========== Node Selection ==========

    /// Selects every node, leaving edge selection as it is.
    ///
    /// The inverse of [`clear_selection`](Self::clear_selection) for nodes alone.
    pub fn select_all_nodes(&mut self) {
        for node in &mut self.nodes {
            node.node.selected = true;
        }
        self.invalidate_z_order();
    }

    /// Selects a node by ID, clearing all other selection.
    ///
    /// If the node doesn't exist, selection is cleared.
    pub fn select_node(&mut self, id: &str) {
        self.clear_selection();

        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(node) = self.nodes.get_mut(idx)
        {
            node.node.selected = true;
        }
    }

    /// Toggles a node's selection without clearing others.
    pub fn toggle_node_selection(&mut self, id: &str) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(node) = self.nodes.get_mut(idx)
        {
            node.node.selected = !node.node.selected;
            self.invalidate_z_order();
        }
    }

    /// Returns an iterator over all selected nodes.
    pub fn selected_nodes(&self) -> impl Iterator<Item = &Node<N>> {
        self.nodes
            .iter()
            .filter(|n| n.node.selected)
            .map(|n| &n.node)
    }

    /// Returns the ID of the first selected node, if any.
    pub fn first_selected_node_id(&self) -> Option<String> {
        self.nodes
            .iter()
            .find(|n| n.node.selected)
            .map(|n| n.node.id.clone())
    }

    /// Selects the next node in insertion order.
    pub fn select_next_node(&mut self) {
        if self.nodes.is_empty() {
            return;
        }

        let current_idx = self.nodes.iter().position(|n| n.node.selected);

        let len = self.nodes.len();
        let next_idx = current_idx.map_or(0, |idx| (idx + 1) % len);

        self.clear_selection();
        if let Some(node) = self.nodes.get_mut(next_idx) {
            node.node.selected = true;
        }
    }

    /// Selects the previous node in insertion order.
    pub fn select_prev_node(&mut self) {
        if self.nodes.is_empty() {
            return;
        }

        let current_idx = self.nodes.iter().position(|n| n.node.selected);

        let len = self.nodes.len();
        let prev_idx = current_idx.map_or(len - 1, |idx| (idx + len - 1) % len);

        self.clear_selection();
        if let Some(node) = self.nodes.get_mut(prev_idx) {
            node.node.selected = true;
        }
    }

    /// Selects the nearest node in the given spatial direction.
    ///
    /// Uses a weighted nearest-neighbor algorithm with directional bias:
    /// candidates within the 180° forward hemisphere are scored by
    /// `distance * (1 + k * angular_penalty)`, where the angular penalty
    /// grows as the candidate deviates from the pure direction. The lowest-scoring
    /// candidate wins.
    ///
    /// If nothing is selected, selects the first node. If no candidate exists
    /// in the forward hemisphere, does nothing (no wrapping in 2D).
    pub fn select_node_in_direction(&mut self, direction: Direction) {
        if self.nodes.is_empty() {
            return;
        }

        // If nothing selected, select the first node (same as SelectNext from nothing)
        let Some(current_idx) = self.nodes.iter().position(|n| n.node.selected) else {
            self.clear_selection();
            if let Some(node) = self.nodes.first_mut() {
                node.node.selected = true;
            }
            return;
        };

        let current_center = self.node_center(current_idx);

        // Direction vector for the primary axis
        let dir = match direction {
            Direction::Up => Position::new(0.0, -1.0),
            Direction::Down => Position::new(0.0, 1.0),
            Direction::Left => Position::new(-1.0, 0.0),
            Direction::Right => Position::new(1.0, 0.0),
        };

        let mut best_idx = None;
        let mut best_score = f64::INFINITY;

        for (i, node) in self.nodes.iter().enumerate() {
            if i == current_idx || node.node.hidden {
                continue;
            }

            let candidate_center = self.node_center(i);
            let dx = candidate_center.x - current_center.x;
            let dy = candidate_center.y - current_center.y;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance < f64::EPSILON {
                continue;
            }

            // Dot product with direction vector — positive means forward hemisphere
            let dot = dx * dir.x + dy * dir.y;
            if dot <= 0.0 {
                continue; // Behind us, exclude
            }

            // Angular penalty: 0.0 (perfectly aligned) to 1.0 (at 90°)
            // sin(angle) = |cross product| / distance
            let cross = (dx * dir.y - dy * dir.x).abs();
            let angular_penalty = cross / distance;

            let score = distance * (1.0 + DIRECTION_BIAS * angular_penalty);

            if score < best_score {
                best_score = score;
                best_idx = Some(i);
            }
        }

        if let Some(idx) = best_idx {
            self.clear_selection();
            if let Some(node) = self.nodes.get_mut(idx) {
                node.node.selected = true;
            }
        }
    }

    /// Returns the center position of a node by index (using absolute position).
    fn node_center(&self, idx: usize) -> Position {
        let node = &self.nodes[idx];
        Position::new(
            node.position_absolute.x + node.node.width / 2.0,
            node.position_absolute.y + node.node.height / 2.0,
        )
    }

    /// Removes all selected nodes and their connected edges.
    ///
    /// Respects the `deletable` flag — nodes with `deletable=false` are skipped.
    /// Returns the removed nodes.
    pub fn remove_selected_nodes(&mut self) -> Vec<Node<N>> {
        let ids: Vec<String> = self
            .nodes
            .iter()
            .filter(|n| n.node.selected && n.node.deletable)
            .map(|n| n.id().to_owned())
            .collect();

        let mut removed = Vec::new();
        for id in ids {
            if let Some(node) = self.remove_node(&id) {
                removed.push(node);
            }
        }
        removed
    }

    // ========== Edge Selection ==========

    /// Selects every edge, leaving node selection as it is.
    ///
    /// The inverse of [`clear_selection`](Self::clear_selection) for edges alone.
    pub fn select_all_edges(&mut self) {
        for edge in &mut self.edges {
            edge.selected = true;
        }
        self.invalidate_z_order();
    }

    /// Selects an edge by ID, clearing all other selection.
    ///
    /// If the edge doesn't exist, selection is cleared.
    pub fn select_edge(&mut self, id: &str) {
        self.clear_selection();

        if let Some(&idx) = self.edge_lookup.get(id)
            && let Some(edge) = self.edges.get_mut(idx)
        {
            edge.selected = true;
        }
    }

    /// Toggles an edge's selection without clearing others.
    pub fn toggle_edge_selection(&mut self, id: &str) {
        if let Some(&idx) = self.edge_lookup.get(id)
            && let Some(edge) = self.edges.get_mut(idx)
        {
            edge.selected = !edge.selected;
        }
    }

    /// Returns an iterator over all selected edges.
    pub fn selected_edges(&self) -> impl Iterator<Item = &Edge<E>> {
        self.edges.iter().filter(|e| e.selected)
    }

    /// Returns the ID of the first selected edge, if any.
    pub fn first_selected_edge_id(&self) -> Option<String> {
        self.edges.iter().find(|e| e.selected).map(|e| e.id.clone())
    }

    /// Selects the next edge in insertion order.
    pub fn select_next_edge(&mut self) {
        if self.edges.is_empty() {
            return;
        }

        let current_idx = self.edges.iter().position(|e| e.selected);

        let len = self.edges.len();
        let next_idx = current_idx.map_or(0, |idx| (idx + 1) % len);

        self.clear_selection();
        if let Some(edge) = self.edges.get_mut(next_idx) {
            edge.selected = true;
        }
    }

    /// Selects the previous edge in insertion order.
    pub fn select_prev_edge(&mut self) {
        if self.edges.is_empty() {
            return;
        }

        let current_idx = self.edges.iter().position(|e| e.selected);

        let len = self.edges.len();
        let prev_idx = current_idx.map_or(len - 1, |idx| (idx + len - 1) % len);

        self.clear_selection();
        if let Some(edge) = self.edges.get_mut(prev_idx) {
            edge.selected = true;
        }
    }

    /// Removes all selected edges.
    ///
    /// Respects the `deletable` flag — edges with `deletable=false` are skipped.
    /// Returns the removed edges.
    pub fn remove_selected_edges(&mut self) -> Vec<Edge<E>> {
        let ids: Vec<String> = self
            .edges
            .iter()
            .filter(|e| e.selected && e.deletable)
            .map(|e| e.id.clone())
            .collect();

        let mut removed = Vec::new();
        for id in ids {
            if let Some(edge) = self.remove_edge(&id) {
                removed.push(edge);
            }
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::Direction;
    use crate::state::Flow;
    use crate::types::{Edge, Node, Position};
    use crate::ui::{StepEdge, TextContent};

    fn make_abc_state() -> Flow<TextContent, StepEdge> {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(0.0, 0.0),
                (10.0, 5.0),
                TextContent::from("A"),
            ),
            Node::new(
                "b",
                Position::new(20.0, 0.0),
                (10.0, 5.0),
                TextContent::from("B"),
            ),
            Node::new(
                "c",
                Position::new(40.0, 0.0),
                (10.0, 5.0),
                TextContent::from("C"),
            ),
        ];
        let edges: Vec<Edge<StepEdge>> = vec![Edge::new("e1", "a", "b"), Edge::new("e2", "b", "c")];
        Flow::with_graph(nodes, edges).unwrap()
    }

    #[test]
    fn test_multi_select_toggle() {
        let mut state = make_abc_state();

        // Toggle a on
        state.toggle_node_selection("a");
        assert!(state.node("a").unwrap().selected);
        assert!(!state.node("b").unwrap().selected);

        // Toggle b on — both selected
        state.toggle_node_selection("b");
        assert!(state.node("a").unwrap().selected);
        assert!(state.node("b").unwrap().selected);

        // Toggle a off — only b selected
        state.toggle_node_selection("a");
        assert!(!state.node("a").unwrap().selected);
        assert!(state.node("b").unwrap().selected);
    }

    #[test]
    fn test_select_node_clears_others() {
        let mut state = make_abc_state();

        // Multi-select a and b
        state.toggle_node_selection("a");
        state.toggle_node_selection("b");
        assert!(state.node("a").unwrap().selected);
        assert!(state.node("b").unwrap().selected);

        // select_node("c") should clear all, then select c
        state.select_node("c");
        assert!(!state.node("a").unwrap().selected);
        assert!(!state.node("b").unwrap().selected);
        assert!(state.node("c").unwrap().selected);
    }

    #[test]
    fn test_with_graph_honors_selected() {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(0.0, 0.0),
                (10.0, 5.0),
                TextContent::from("A"),
            )
            .with_selected(true),
            Node::new(
                "b",
                Position::new(20.0, 0.0),
                (10.0, 5.0),
                TextContent::from("B"),
            ),
        ];
        let state: Flow<TextContent, StepEdge> = Flow::with_graph(nodes, vec![]).unwrap();

        assert!(state.node("a").unwrap().selected);
        assert!(!state.node("b").unwrap().selected);
        assert!(state.has_selected_nodes() || state.has_selected_edges());
    }

    #[test]
    fn test_remove_all_selected() {
        let mut state = make_abc_state();

        // Multi-select a and c
        state.toggle_node_selection("a");
        state.toggle_node_selection("c");

        let removed = state.remove_selected_nodes();
        assert_eq!(removed.len(), 2);
        assert_eq!(state.nodes.len(), 1);
        assert!(state.node("b").is_some());
        // Edges connected to removed nodes should also be gone
        assert_eq!(state.edges().len(), 0);
    }

    #[test]
    fn test_select_next_prev_node() {
        let mut state = make_abc_state();

        // No selection — next selects first
        state.select_next_node();
        assert!(state.node("a").unwrap().selected);

        // Next from a -> b
        state.select_next_node();
        assert!(!state.node("a").unwrap().selected);
        assert!(state.node("b").unwrap().selected);

        // Next from c wraps to a
        state.select_next_node(); // c
        state.select_next_node(); // wrap to a
        assert!(state.node("a").unwrap().selected);

        // Prev from a wraps to c
        state.select_prev_node();
        assert!(state.node("c").unwrap().selected);
    }

    #[test]
    fn test_remove_selected_edges() {
        let mut state = make_abc_state();

        state.toggle_edge_selection("e1");
        state.toggle_edge_selection("e2");

        let removed = state.remove_selected_edges();
        assert_eq!(removed.len(), 2);
        assert_eq!(state.edges().len(), 0);
        // Nodes should still exist
        assert_eq!(state.nodes.len(), 3);
    }

    #[test]
    fn test_remove_selected_combined() {
        let mut state = make_abc_state();

        state.toggle_node_selection("a");
        state.toggle_edge_selection("e2"); // b->c

        let removed_nodes = state.remove_selected_nodes();
        let removed_edges = state.remove_selected_edges();
        assert!(!removed_nodes.is_empty() || !removed_edges.is_empty());
        // Node a removed (along with e1 which connects a->b)
        // Edge e2 was selected and removed
        assert_eq!(state.nodes.len(), 2);
        assert_eq!(state.edges().len(), 0);
    }

    // ========== Directional Navigation ==========

    /// Horizontal row: a(0,0) — b(20,0) — c(40,0)
    #[test]
    fn test_directional_horizontal_row() {
        let mut state = make_abc_state();

        // Select a, press Right → b
        state.select_node("a");
        state.select_node_in_direction(Direction::Right);
        assert!(state.node("b").unwrap().selected);

        // From b, press Right → c
        state.select_node_in_direction(Direction::Right);
        assert!(state.node("c").unwrap().selected);

        // From c, press Right → no candidate (rightmost), stay on c
        state.select_node_in_direction(Direction::Right);
        assert!(state.node("c").unwrap().selected);

        // From c, press Left → b
        state.select_node_in_direction(Direction::Left);
        assert!(state.node("b").unwrap().selected);

        // From b, press Left → a
        state.select_node_in_direction(Direction::Left);
        assert!(state.node("a").unwrap().selected);
    }

    /// Grid layout:
    ///   a(0,0)   b(20,0)
    ///   c(0,20)  d(20,20)
    fn make_grid_state() -> Flow<TextContent, StepEdge> {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(0.0, 0.0),
                (10.0, 5.0),
                TextContent::from("A"),
            ),
            Node::new(
                "b",
                Position::new(20.0, 0.0),
                (10.0, 5.0),
                TextContent::from("B"),
            ),
            Node::new(
                "c",
                Position::new(0.0, 20.0),
                (10.0, 5.0),
                TextContent::from("C"),
            ),
            Node::new(
                "d",
                Position::new(20.0, 20.0),
                (10.0, 5.0),
                TextContent::from("D"),
            ),
        ];
        Flow::with_graph(nodes, vec![]).unwrap()
    }

    #[test]
    fn test_directional_grid_navigation() {
        let mut state = make_grid_state();

        // Start at a, go Right → b
        state.select_node("a");
        state.select_node_in_direction(Direction::Right);
        assert!(state.node("b").unwrap().selected);

        // From b, go Down → d
        state.select_node_in_direction(Direction::Down);
        assert!(state.node("d").unwrap().selected);

        // From d, go Left → c
        state.select_node_in_direction(Direction::Left);
        assert!(state.node("c").unwrap().selected);

        // From c, go Up → a
        state.select_node_in_direction(Direction::Up);
        assert!(state.node("a").unwrap().selected);
    }

    #[test]
    fn test_directional_no_selection_selects_first() {
        let mut state = make_grid_state();

        // Nothing selected, any direction selects first node
        state.select_node_in_direction(Direction::Right);
        assert!(state.node("a").unwrap().selected);
    }

    #[test]
    fn test_directional_no_candidate_stays_put() {
        let mut state = make_grid_state();

        // Select a, go Up — nothing above, stays on a
        state.select_node("a");
        state.select_node_in_direction(Direction::Up);
        assert!(state.node("a").unwrap().selected);

        // Select a, go Left — nothing to the left, stays on a
        state.select_node_in_direction(Direction::Left);
        assert!(state.node("a").unwrap().selected);
    }

    #[test]
    fn test_directional_diagonal_prefers_aligned() {
        // Three nodes: center, one directly right, one diagonally up-right
        //   diag(15, -15)
        //   center(0, 0) ———— right(20, 0)
        let nodes = vec![
            Node::new(
                "center",
                Position::new(0.0, 0.0),
                (5.0, 5.0),
                TextContent::from("C"),
            ),
            Node::new(
                "right",
                Position::new(20.0, 0.0),
                (5.0, 5.0),
                TextContent::from("R"),
            ),
            Node::new(
                "diag",
                Position::new(15.0, -15.0),
                (5.0, 5.0),
                TextContent::from("D"),
            ),
        ];
        let mut state: Flow = Flow::with_graph(nodes, vec![]).unwrap();

        // From center, Right should pick "right" over "diag" because "right" is
        // perfectly aligned (angular penalty = 0) while "diag" has ~45° penalty
        state.select_node("center");
        state.select_node_in_direction(Direction::Right);
        assert!(state.node("right").unwrap().selected);
    }

    #[test]
    fn test_directional_closer_off_axis_wins_over_distant_aligned() {
        // A very close node slightly off-axis should beat a far node perfectly on-axis
        //   center(0,0)   close(8, 3)              far(100, 0)
        let nodes = vec![
            Node::new(
                "center",
                Position::new(0.0, 0.0),
                (5.0, 5.0),
                TextContent::from("C"),
            ),
            Node::new(
                "close",
                Position::new(8.0, 3.0),
                (5.0, 5.0),
                TextContent::from("N"),
            ),
            Node::new(
                "far",
                Position::new(100.0, 0.0),
                (5.0, 5.0),
                TextContent::from("F"),
            ),
        ];
        let mut state: Flow = Flow::with_graph(nodes, vec![]).unwrap();

        state.select_node("center");
        state.select_node_in_direction(Direction::Right);
        assert!(state.node("close").unwrap().selected);
    }

    #[test]
    fn test_directional_skips_hidden_nodes() {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(0.0, 0.0),
                (5.0, 5.0),
                TextContent::from("A"),
            ),
            Node::new(
                "b",
                Position::new(20.0, 0.0),
                (5.0, 5.0),
                TextContent::from("B"),
            )
            .with_hidden(true),
            Node::new(
                "c",
                Position::new(40.0, 0.0),
                (5.0, 5.0),
                TextContent::from("C"),
            ),
        ];
        let mut state: Flow = Flow::with_graph(nodes, vec![]).unwrap();

        // From a, Right should skip hidden b and go to c
        state.select_node("a");
        state.select_node_in_direction(Direction::Right);
        assert!(state.node("c").unwrap().selected);
    }

    #[test]
    fn test_directional_single_node_stays() {
        let nodes = vec![Node::new(
            "only",
            Position::new(0.0, 0.0),
            (5.0, 5.0),
            TextContent::from("O"),
        )];
        let mut state: Flow = Flow::with_graph(nodes, vec![]).unwrap();

        state.select_node("only");
        state.select_node_in_direction(Direction::Right);
        assert!(state.node("only").unwrap().selected);
    }

    #[test]
    fn test_directional_all_four_directions_reachable() {
        // Diamond layout — each node reachable from center in exactly one direction
        //         top(0, -20)
        //  left(-20, 0)  center(0, 0)  right(20, 0)
        //         bottom(0, 20)
        let nodes = vec![
            Node::new(
                "center",
                Position::new(0.0, 0.0),
                (5.0, 5.0),
                TextContent::from("C"),
            ),
            Node::new(
                "top",
                Position::new(0.0, -20.0),
                (5.0, 5.0),
                TextContent::from("T"),
            ),
            Node::new(
                "bottom",
                Position::new(0.0, 20.0),
                (5.0, 5.0),
                TextContent::from("B"),
            ),
            Node::new(
                "left",
                Position::new(-20.0, 0.0),
                (5.0, 5.0),
                TextContent::from("L"),
            ),
            Node::new(
                "right",
                Position::new(20.0, 0.0),
                (5.0, 5.0),
                TextContent::from("R"),
            ),
        ];
        let mut state: Flow = Flow::with_graph(nodes, vec![]).unwrap();

        state.select_node("center");

        state.select_node_in_direction(Direction::Up);
        assert!(state.node("top").unwrap().selected);

        state.select_node("center");
        state.select_node_in_direction(Direction::Down);
        assert!(state.node("bottom").unwrap().selected);

        state.select_node("center");
        state.select_node_in_direction(Direction::Left);
        assert!(state.node("left").unwrap().selected);

        state.select_node("center");
        state.select_node_in_direction(Direction::Right);
        assert!(state.node("right").unwrap().selected);
    }

    /// The contract that differs from `clear_selection`, which clears both:
    /// selecting all of one kind must leave the other kind alone.
    #[test]
    fn test_select_all_nodes_leaves_edges_alone() {
        let mut state = make_abc_state();
        state.select_edge("e1");

        state.select_all_nodes();

        assert_eq!(state.selected_nodes().count(), 3);
        assert!(state.edge("e1").unwrap().selected);
        assert!(!state.edge("e2").unwrap().selected);
    }

    #[test]
    fn test_select_all_edges_leaves_nodes_alone() {
        let mut state = make_abc_state();
        state.select_node("a");

        state.select_all_edges();

        assert_eq!(state.selected_edges().count(), 2);
        assert!(state.node("a").unwrap().selected);
        assert!(!state.node("b").unwrap().selected);
    }
}
