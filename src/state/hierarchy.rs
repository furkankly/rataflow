//! Hierarchy resolution — computes absolute positions and handle bounds for all nodes.
//!
//! The central function is [`resolve_hierarchy()`](Flow::resolve_hierarchy), called after
//! any mutation that affects positions or dimensions. It runs a 3-phase algorithm:
//!
//! 1. **Top-down BFS** — compute absolute positions, apply extent constraints.
//! 2. **Bottom-up expansion** — expand parents to contain `expand_parent` children.
//! 3. **Re-resolve positions** — propagate updated positions after expansion.

use std::collections::HashMap;

use crate::content::{EdgeContent, NodeContent};
use crate::types::{CoordinateExtent, NodeExtent, Rect as FlowRect, get_position_with_origin};

use super::Flow;
use super::mouse::DragState;

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    /// Resolves the hierarchy if a drag operation deferred it.
    ///
    /// After resolving, updates the cached `parent_absolute` in the drag state
    /// so that subsequent drag events use the correct parent reference frame.
    /// Without this, `expand_parent` leftward/upward shifts would desync the
    /// drag offset, causing amplified movement.
    pub(crate) fn resolve_drag_hierarchy_if_pending(&mut self) {
        if self.drag_hierarchy_pending {
            self.resolve_hierarchy();
            self.drag_hierarchy_pending = false;

            // Refresh cached parent_absolute in drag state.
            let new_abs = if let DragState::MovingNode { node_id, .. } = &self.drag_state {
                self.parent_absolute_of(node_id)
            } else {
                None
            };
            if let DragState::MovingNode {
                parent_absolute, ..
            } = &mut self.drag_state
            {
                *parent_absolute = new_abs;
            }
        }
    }

    /// Resolves the node hierarchy, computing absolute positions and handle bounds for all nodes.
    ///
    /// This also enforces extent constraints (e.g., `NodeExtent::Parent`) to ensure
    /// child nodes stay within their allowed bounds, and expands parent nodes when
    /// children have `expand_parent = true`.
    ///
    /// Three phases:
    /// 1. **Top-down BFS** — compute absolute positions, apply extent constraints,
    ///    collect BFS levels and detect `expand_parent` usage.
    /// 2. **Bottom-up expansion** — iterate levels deepest-first, expanding parents
    ///    to contain their `expand_parent` children. Only runs if any node uses
    ///    `expand_parent`.
    /// 3. **Re-resolve positions** — re-run phase 1 to propagate updated positions
    ///    after expansion. Only runs if phase 2 actually changed something.
    pub(crate) fn resolve_hierarchy(&mut self) {
        let (parent_children, root_nodes) = self.build_hierarchy_maps();
        let (has_expand_parent, levels) =
            self.resolve_positions_top_down(&parent_children, &root_nodes);

        if has_expand_parent {
            let expansion_occurred = self.expand_parents_bottom_up(&parent_children, &levels);

            if expansion_occurred {
                // Re-resolve all positions after expansion
                self.resolve_positions_top_down(&parent_children, &root_nodes);
            }
        }
    }

    /// Builds parent->children index mapping and identifies root nodes.
    fn build_hierarchy_maps(&self) -> (HashMap<usize, Vec<usize>>, Vec<usize>) {
        let mut parent_children: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut root_nodes: Vec<usize> = Vec::new();

        for (idx, node) in self.nodes.iter().enumerate() {
            if let Some(parent_id) = &node.node.parent_id {
                if let Some(&parent_idx) = self.node_lookup.get(parent_id) {
                    parent_children.entry(parent_idx).or_default().push(idx);
                } else {
                    root_nodes.push(idx);
                }
            } else {
                root_nodes.push(idx);
            }
        }

        (parent_children, root_nodes)
    }

    /// Phase 1: Top-down BFS to compute absolute positions and handle bounds.
    ///
    /// Returns `(has_expand_parent, levels)` where `levels` contains parent indices
    /// per BFS level (for the bottom-up pass).
    fn resolve_positions_top_down(
        &mut self,
        parent_children: &HashMap<usize, Vec<usize>>,
        root_nodes: &[usize],
    ) -> (bool, Vec<Vec<usize>>) {
        let mut has_expand_parent = false;
        let mut levels: Vec<Vec<usize>> = Vec::new();

        // Update root nodes
        for &idx in root_nodes {
            let node = &mut self.nodes[idx];
            node.position_absolute = get_position_with_origin(&node.node);
            node.update_handle_bounds();
        }

        // BFS level by level
        let mut to_process: Vec<usize> = root_nodes.to_vec();

        while !to_process.is_empty() {
            let mut next_level: Vec<usize> = Vec::new();
            // Collect parents that have children at this level
            let mut level_parents: Vec<usize> = Vec::new();

            for &parent_idx in &to_process {
                let parent_pos = self.nodes[parent_idx].position_absolute;
                let parent_w = self.nodes[parent_idx].node.width;
                let parent_h = self.nodes[parent_idx].node.height;

                if let Some(children) = parent_children.get(&parent_idx) {
                    if !children.is_empty() {
                        level_parents.push(parent_idx);
                    }

                    for &child_idx in children {
                        // Track expand_parent flag
                        if self.nodes[child_idx].node.expand_parent {
                            has_expand_parent = true;
                        }

                        let child = &mut self.nodes[child_idx];
                        child.update_position_from_parent(parent_pos);

                        // Apply extent constraints — skip Parent clamping for expand_parent children
                        if let Some(extent) = &child.node.extent {
                            let should_clamp = match extent {
                                NodeExtent::Parent => !child.node.expand_parent,
                                NodeExtent::Coordinates(_) => true,
                            };

                            if should_clamp {
                                let coords = match extent {
                                    NodeExtent::Parent => CoordinateExtent::from_coords(
                                        0.0,
                                        0.0,
                                        (parent_w - child.node.width).max(0.0),
                                        (parent_h - child.node.height).max(0.0),
                                    ),
                                    NodeExtent::Coordinates(c) => *c,
                                };

                                let clamped_relative = child.node.position.clamp(&coords);
                                if clamped_relative != child.node.position {
                                    child.node.position = clamped_relative;
                                    child.update_position_from_parent(parent_pos);
                                }
                            }
                        }

                        child.update_handle_bounds();
                        next_level.push(child_idx);
                    }
                }
            }

            if !level_parents.is_empty() {
                levels.push(level_parents);
            }
            to_process = next_level;
        }

        (has_expand_parent, levels)
    }

    /// Phase 2: Bottom-up expansion of parents to contain their `expand_parent` children.
    ///
    /// Returns `true` if any parent was expanded.
    fn expand_parents_bottom_up(
        &mut self,
        parent_children: &HashMap<usize, Vec<usize>>,
        levels: &[Vec<usize>],
    ) -> bool {
        let mut expansion_occurred = false;

        // Iterate from deepest level to shallowest
        for level in levels.iter().rev() {
            for &parent_idx in level {
                let parent = &self.nodes[parent_idx];
                let parent_rect = FlowRect::new(parent.position_absolute, parent.dimensions());
                let mut expanded_rect = parent_rect;

                // Union with all expand_parent children
                if let Some(children) = parent_children.get(&parent_idx) {
                    for &child_idx in children {
                        if self.nodes[child_idx].node.expand_parent {
                            let child = &self.nodes[child_idx];
                            let child_rect =
                                FlowRect::new(child.position_absolute, child.dimensions());
                            expanded_rect = expanded_rect.union(&child_rect);
                        }
                    }
                }

                // Skip if no expansion needed
                if expanded_rect == parent_rect {
                    continue;
                }

                expansion_occurred = true;

                // Compute shifts
                let x_shift = parent_rect.x() - expanded_rect.x();
                let y_shift = parent_rect.y() - expanded_rect.y();
                let old_w = self.nodes[parent_idx].node.width;
                let old_h = self.nodes[parent_idx].node.height;
                let new_w = expanded_rect.width();
                let new_h = expanded_rect.height();

                // Update parent dimensions
                self.nodes[parent_idx].node.width = new_w;
                self.nodes[parent_idx].node.height = new_h;

                // Update parent position (origin-aware)
                let origin = self.nodes[parent_idx].node.origin;
                let width_change = (new_w - old_w) * origin.x;
                let height_change = (new_h - old_h) * origin.y;
                self.nodes[parent_idx].node.position.x += width_change - x_shift;
                self.nodes[parent_idx].node.position.y += height_change - y_shift;

                // Recompute parent's position_absolute for cascade correctness
                let origin_pos = get_position_with_origin(&self.nodes[parent_idx].node);
                let gp_idx = self.nodes[parent_idx]
                    .node
                    .parent_id
                    .as_ref()
                    .and_then(|gid| self.node_lookup.get(gid.as_str()).copied());
                self.nodes[parent_idx].position_absolute = match gp_idx {
                    Some(idx) => self.nodes[idx].position_absolute + origin_pos,
                    None => origin_pos,
                };

                // Counter-adjust ALL children to preserve their absolute positions
                if let Some(children) = parent_children.get(&parent_idx) {
                    for &child_idx in children {
                        self.nodes[child_idx].node.position.x += x_shift;
                        self.nodes[child_idx].node.position.y += y_shift;
                    }
                }
            }
        }

        expansion_occurred
    }
}

#[cfg(test)]
mod tests {
    use crate::state::Flow;
    use crate::types::{Node, Position};
    use crate::ui::{StepEdge, TextContent};

    #[test]
    fn test_hierarchy_resolution_single_level() {
        let parent = Node::new(
            "parent",
            Position::new(10.0, 20.0),
            (50.0, 30.0),
            TextContent::from("Parent"),
        );

        let child = Node::new(
            "child",
            Position::new(5.0, 5.0),
            (10.0, 5.0),
            TextContent::from("Child"),
        )
        .with_parent("parent");

        let state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![parent, child], vec![]).unwrap();

        let parent_node = state.nodes.iter().find(|n| n.id() == "parent").unwrap();
        let child_node = state.nodes.iter().find(|n| n.id() == "child").unwrap();

        assert_eq!(parent_node.position_absolute.x, 10.0);
        assert_eq!(parent_node.position_absolute.y, 20.0);

        assert_eq!(child_node.position_absolute.x, 15.0);
        assert_eq!(child_node.position_absolute.y, 25.0);
    }

    #[test]
    fn test_hierarchy_resolution_multi_level() {
        let root = Node::new(
            "root",
            Position::new(0.0, 0.0),
            (100.0, 100.0),
            TextContent::from("Root"),
        );

        let child = Node::new(
            "child",
            Position::new(10.0, 10.0),
            (50.0, 50.0),
            TextContent::from("Child"),
        )
        .with_parent("root");

        let grandchild = Node::new(
            "grandchild",
            Position::new(5.0, 5.0),
            (10.0, 10.0),
            TextContent::from("Grandchild"),
        )
        .with_parent("child");

        let state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![root, child, grandchild], vec![]).unwrap();

        let grandchild_node = state.nodes.iter().find(|n| n.id() == "grandchild").unwrap();

        assert_eq!(grandchild_node.position_absolute.x, 15.0);
        assert_eq!(grandchild_node.position_absolute.y, 15.0);
    }

    #[test]
    fn test_extent_constraint_enforced_on_dimension_change() {
        use crate::types::NodeExtent;

        let parent = Node::new(
            "parent",
            Position::new(25.0, 2.0),
            (35.0, 18.0),
            TextContent::from("Parent"),
        );

        let child = Node::new(
            "child",
            Position::new(5.0, 5.0),
            (15.0, 6.0),
            TextContent::from("Child"),
        )
        .with_parent("parent")
        .with_extent(NodeExtent::Parent);

        let mut state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![parent, child], vec![]).unwrap();

        let child_node = state.internal_node("child").unwrap();
        assert_eq!(child_node.position_absolute.x, 30.0);
        assert_eq!(child_node.position_absolute.y, 7.0);
        assert_eq!(child_node.node.position.x, 5.0);
        assert_eq!(child_node.node.position.y, 5.0);

        state.set_node_dimensions("child", 999.0, 999.0);

        let child_node = state.internal_node("child").unwrap();
        assert_eq!(child_node.node.width, 999.0);
        assert_eq!(child_node.node.height, 999.0);
        assert_eq!(child_node.node.position.x, 0.0);
        assert_eq!(child_node.node.position.y, 0.0);
        assert_eq!(child_node.position_absolute.x, 25.0);
        assert_eq!(child_node.position_absolute.y, 2.0);
    }

    #[test]
    fn test_extent_constraint_clamps_within_parent() {
        use crate::types::NodeExtent;

        let parent = Node::new(
            "parent",
            Position::new(0.0, 0.0),
            (100.0, 100.0),
            TextContent::from("Parent"),
        );

        let child = Node::new(
            "child",
            Position::new(80.0, 80.0),
            (10.0, 10.0),
            TextContent::from("Child"),
        )
        .with_parent("parent")
        .with_extent(NodeExtent::Parent);

        let mut state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![parent, child], vec![]).unwrap();

        let child_node = state.internal_node("child").unwrap();
        assert_eq!(child_node.node.position.x, 80.0);
        assert_eq!(child_node.node.position.y, 80.0);

        state.set_node_dimensions("child", 30.0, 30.0);

        let child_node = state.internal_node("child").unwrap();
        assert_eq!(child_node.node.position.x, 70.0);
        assert_eq!(child_node.node.position.y, 70.0);
        assert_eq!(child_node.position_absolute.x, 70.0);
        assert_eq!(child_node.position_absolute.y, 70.0);
        assert_eq!(child_node.node.width, 30.0);
        assert_eq!(child_node.node.height, 30.0);
    }

    // --- expand_parent tests ---

    #[test]
    fn test_expand_parent_rightward_downward() {
        let parent = Node::new("parent", (0.0, 0.0), (100.0, 100.0), TextContent::from("P"));
        let child = Node::new("child", (60.0, 60.0), (60.0, 60.0), TextContent::from("C"))
            .with_parent("parent")
            .with_expand_parent(true);

        let state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![parent, child], vec![]).unwrap();

        let p = state.internal_node("parent").unwrap();
        assert_eq!(p.node.width, 120.0);
        assert_eq!(p.node.height, 120.0);
        assert_eq!(p.position_absolute.x, 0.0);
        assert_eq!(p.position_absolute.y, 0.0);

        let c = state.internal_node("child").unwrap();
        assert_eq!(c.position_absolute.x, 60.0);
        assert_eq!(c.position_absolute.y, 60.0);
    }

    #[test]
    fn test_expand_parent_leftward_upward() {
        let parent = Node::new(
            "parent",
            (50.0, 50.0),
            (100.0, 100.0),
            TextContent::from("P"),
        );
        let child = Node::new(
            "child",
            (-20.0, -20.0),
            (40.0, 40.0),
            TextContent::from("C"),
        )
        .with_parent("parent")
        .with_expand_parent(true);
        let sibling = Node::new(
            "sibling",
            (10.0, 10.0),
            (20.0, 20.0),
            TextContent::from("S"),
        )
        .with_parent("parent");

        let state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![parent, child, sibling], vec![]).unwrap();

        let p = state.internal_node("parent").unwrap();
        assert_eq!(p.node.width, 120.0);
        assert_eq!(p.node.height, 120.0);
        assert_eq!(p.position_absolute.x, 30.0);
        assert_eq!(p.position_absolute.y, 30.0);

        let c = state.internal_node("child").unwrap();
        assert_eq!(c.position_absolute.x, 30.0);
        assert_eq!(c.position_absolute.y, 30.0);

        let s = state.internal_node("sibling").unwrap();
        assert_eq!(s.position_absolute.x, 60.0);
        assert_eq!(s.position_absolute.y, 60.0);
    }

    #[test]
    fn test_expand_parent_multiple_children() {
        let parent = Node::new(
            "parent",
            (50.0, 50.0),
            (100.0, 100.0),
            TextContent::from("P"),
        );
        let child_left = Node::new("left", (-10.0, 0.0), (30.0, 30.0), TextContent::from("L"))
            .with_parent("parent")
            .with_expand_parent(true);
        let child_right = Node::new("right", (90.0, 0.0), (30.0, 30.0), TextContent::from("R"))
            .with_parent("parent")
            .with_expand_parent(true);

        let state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![parent, child_left, child_right], vec![]).unwrap();

        let p = state.internal_node("parent").unwrap();
        assert_eq!(p.node.width, 130.0);
        assert_eq!(p.node.height, 100.0);
        assert_eq!(p.position_absolute.x, 40.0);
        assert_eq!(p.position_absolute.y, 50.0);
    }

    #[test]
    fn test_expand_parent_bottom_up_cascade() {
        let grandparent = Node::new("gp", (0.0, 0.0), (100.0, 100.0), TextContent::from("GP"));
        let parent = Node::new("p", (10.0, 10.0), (50.0, 50.0), TextContent::from("P"))
            .with_parent("gp")
            .with_expand_parent(true);
        let child = Node::new("c", (40.0, 40.0), (30.0, 30.0), TextContent::from("C"))
            .with_parent("p")
            .with_expand_parent(true);

        let state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![grandparent, parent, child], vec![]).unwrap();

        let p = state.internal_node("p").unwrap();
        assert_eq!(p.node.width, 70.0);
        assert_eq!(p.node.height, 70.0);

        let gp = state.internal_node("gp").unwrap();
        assert_eq!(gp.node.width, 100.0);
        assert_eq!(gp.node.height, 100.0);

        let c = state.internal_node("c").unwrap();
        assert_eq!(c.position_absolute.x, 50.0);
        assert_eq!(c.position_absolute.y, 50.0);
    }

    #[test]
    fn test_expand_parent_with_center_origin() {
        use crate::types::NodeOrigin;

        let parent = Node::new(
            "parent",
            (50.0, 50.0),
            (100.0, 100.0),
            TextContent::from("P"),
        )
        .with_origin(NodeOrigin::CENTER);
        let child = Node::new("child", (60.0, 60.0), (60.0, 60.0), TextContent::from("C"))
            .with_parent("parent")
            .with_expand_parent(true);

        let state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![parent, child], vec![]).unwrap();

        let p = state.internal_node("parent").unwrap();
        assert_eq!(p.node.width, 120.0);
        assert_eq!(p.node.height, 120.0);
        assert_eq!(p.node.position.x, 60.0);
        assert_eq!(p.node.position.y, 60.0);
        assert_eq!(p.position_absolute.x, 0.0);
        assert_eq!(p.position_absolute.y, 0.0);
    }

    #[test]
    fn test_expand_parent_bypasses_parent_extent() {
        use crate::types::NodeExtent;

        let parent = Node::new("parent", (0.0, 0.0), (100.0, 100.0), TextContent::from("P"));
        let child = Node::new("child", (80.0, 80.0), (40.0, 40.0), TextContent::from("C"))
            .with_parent("parent")
            .with_expand_parent(true)
            .with_extent(NodeExtent::Parent);

        let state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![parent, child], vec![]).unwrap();

        let c = state.internal_node("child").unwrap();
        assert_eq!(c.node.position.x, 80.0);
        assert_eq!(c.node.position.y, 80.0);
        assert_eq!(c.position_absolute.x, 80.0);
        assert_eq!(c.position_absolute.y, 80.0);

        let p = state.internal_node("parent").unwrap();
        assert_eq!(p.node.width, 120.0);
        assert_eq!(p.node.height, 120.0);
    }

    #[test]
    fn test_expand_parent_no_expansion_needed() {
        let parent = Node::new("parent", (0.0, 0.0), (100.0, 100.0), TextContent::from("P"));
        let child = Node::new("child", (10.0, 10.0), (20.0, 20.0), TextContent::from("C"))
            .with_parent("parent")
            .with_expand_parent(true);

        let state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![parent, child], vec![]).unwrap();

        let p = state.internal_node("parent").unwrap();
        assert_eq!(p.node.width, 100.0);
        assert_eq!(p.node.height, 100.0);
        assert_eq!(p.position_absolute.x, 0.0);
        assert_eq!(p.position_absolute.y, 0.0);
    }

    #[test]
    fn test_child_without_expand_parent_does_not_expand() {
        let parent = Node::new("parent", (0.0, 0.0), (100.0, 100.0), TextContent::from("P"));
        let child = Node::new("child", (80.0, 80.0), (60.0, 60.0), TextContent::from("C"))
            .with_parent("parent");

        let state: Flow<TextContent, StepEdge> =
            Flow::with_graph(vec![parent, child], vec![]).unwrap();

        let p = state.internal_node("parent").unwrap();
        assert_eq!(p.node.width, 100.0);
        assert_eq!(p.node.height, 100.0);
    }
}
