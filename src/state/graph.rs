//! Graph operations for Flow.
//!
//! Provides methods for querying and mutating nodes and edges.

use super::{DragState, Flow};
use crate::content::{EdgeContent, NodeContent};
use crate::error::Error;
use crate::types::{
    ComputedHandle, Connection, Edge, HandleType, InternalNode, Node, Position, Reconnectable, Rect,
};
use crate::ui::HandleStyle;

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    // ========== Internal Accessors (for library use) ==========

    /// Internal: Returns an InternalNode by ID.
    pub(crate) fn internal_node(&self, id: &str) -> Option<&InternalNode<N>> {
        self.node_lookup.get(id).and_then(|&i| self.nodes.get(i))
    }

    /// Internal: Returns a mutable InternalNode by ID.
    pub(crate) fn internal_node_mut(&mut self, id: &str) -> Option<&mut InternalNode<N>> {
        self.node_lookup
            .get(id)
            .copied()
            .and_then(|i| self.nodes.get_mut(i))
    }

    /// Internal: Returns the absolute position of a node's parent, if any.
    pub(crate) fn parent_absolute_of(&self, node_id: &str) -> Option<crate::types::Position> {
        self.internal_node(node_id)
            .and_then(|n| n.node.parent_id.as_ref())
            .and_then(|pid| self.internal_node(pid).map(|p| p.position_absolute))
    }

    /// Resolves an edge's endpoints to the handles it currently attaches to.
    ///
    /// The single place edge attachment is decided. Rendering and hit testing both
    /// go through it, so what is drawn and what is clickable cannot disagree.
    ///
    /// Returns `None` if either endpoint node is missing; render-time is defensive
    /// about orphan edges rather than panicking.
    pub(crate) fn resolve_edge_handles(
        &self,
        edge: &Edge<E>,
    ) -> Option<(&ComputedHandle, &ComputedHandle)> {
        let source = self.internal_node(edge.source.as_str())?;
        let target = self.internal_node(edge.target.as_str())?;
        Some((
            source.handle_bounds.get(
                edge.source_handle.as_deref(),
                HandleType::Source,
                self.connection_mode,
            ),
            target.handle_bounds.get(
                edge.target_handle.as_deref(),
                HandleType::Target,
                self.connection_mode,
            ),
        ))
    }

    // ========== Read Accessors ==========

    /// Returns an iterator over all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &Node<N>> {
        self.nodes.iter().map(|internal| &internal.node)
    }

    /// Returns a slice of all edges.
    pub fn edges(&self) -> &[Edge<E>] {
        &self.edges
    }

    /// Returns a node by ID.
    pub fn node(&self, id: &str) -> Option<&Node<N>> {
        self.internal_node(id).map(|internal| &internal.node)
    }

    /// Returns an edge by ID.
    pub fn edge(&self, id: &str) -> Option<&Edge<E>> {
        self.edge_lookup.get(id).and_then(|&i| self.edges.get(i))
    }

    /// Returns a node's bounding rectangle in world coordinates.
    ///
    /// Parent offsets are already resolved, so this is the node's true position in
    /// the graph rather than the parent-relative one stored on [`Node::position`].
    /// Use it to persist a hierarchical graph to a format that stores absolute
    /// coordinates, or to reason about where a node actually sits.
    ///
    /// Returns an owned value — it answers where the node is without handing out a
    /// borrow of the layout the flow maintains.
    pub fn node_bounds(&self, id: &str) -> Option<Rect> {
        self.internal_node(id).map(|internal| internal.bounds())
    }

    /// Returns the IDs of nodes whose bounds intersect `area`, in world coordinates.
    ///
    /// Hidden nodes are skipped. Order follows insertion, not z-order — this is a
    /// spatial query, not a pick; for "what is under the cursor" react to
    /// [`FlowEvent::NodeClicked`](crate::FlowEvent::NodeClicked) instead.
    pub fn nodes_in(&self, area: Rect) -> impl Iterator<Item = &str> {
        self.nodes
            .iter()
            .filter(move |internal| !internal.node.hidden && internal.bounds().intersects(&area))
            .map(|internal| internal.node.id.as_str())
    }

    // ========== Add Operations ==========

    /// Adds a node to the graph.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A node with the same ID already exists
    /// - The node has a parent_id that doesn't exist
    /// - Multiple handles of the same type lack IDs (ambiguous)
    /// - Duplicate handle IDs exist within the same type
    pub fn add_node(&mut self, node: Node<N>) -> Result<(), Error> {
        // Check for duplicate ID (O(1) with HashMap)
        if self.node_lookup.contains_key(&node.id) {
            return Err(Error::DuplicateNodeId {
                node_id: node.id.clone(),
            });
        }

        // Validate parent reference if present (O(1) with HashMap)
        if let Some(parent_id) = &node.parent_id
            && !self.node_lookup.contains_key(parent_id)
        {
            return Err(Error::InvalidParentReference {
                node_id: node.id.clone(),
                parent_id: parent_id.clone(),
            });
        }

        // Validate handles
        Self::validate_handles(&node.id, &node.handles)?;

        let internal = InternalNode::from_node(node);
        let node_id = internal.id().to_owned();
        self.nodes.push(internal);
        self.node_lookup.insert(node_id, self.nodes.len() - 1);

        self.invalidate_z_order();
        self.resolve_hierarchy();

        Ok(())
    }

    /// Adds an edge to the graph.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - An edge with the same ID already exists
    /// - The source or target node doesn't exist
    /// - The source and target are the same node (self-referential)
    pub fn add_edge(&mut self, edge: Edge<E>) -> Result<(), Error> {
        // Check for duplicate ID (O(1) with HashMap)
        if self.edge_lookup.contains_key(&edge.id) {
            return Err(Error::DuplicateEdgeId {
                edge_id: edge.id.clone(),
            });
        }

        // Check for self-referential edge
        if edge.source == edge.target {
            return Err(Error::SelfReferentialEdge {
                edge_id: edge.id.clone(),
                node_id: edge.source.clone(),
            });
        }

        // Validate source exists (O(1) with HashMap)
        if !self.node_lookup.contains_key(&edge.source) {
            return Err(Error::InvalidEdgeReference {
                edge_id: edge.id.clone(),
                node_id: edge.source.clone(),
            });
        }

        // Validate target exists (O(1) with HashMap)
        if !self.node_lookup.contains_key(&edge.target) {
            return Err(Error::InvalidEdgeReference {
                edge_id: edge.id.clone(),
                node_id: edge.target.clone(),
            });
        }

        let edge_id = edge.id.clone();
        self.edges.push(edge);
        self.edge_lookup.insert(edge_id, self.edges.len() - 1);
        Ok(())
    }

    /// Adds an edge from a completed connection.
    ///
    /// Generates a deterministic edge ID from connection endpoints, constructs
    /// the edge, and delegates to [`add_edge`](Self::add_edge).
    ///
    /// Returns `None` if an edge with the same endpoints already exists.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rataflow::{Flow, FlowEvent, StepEdge};
    /// # let mut flow: Flow = Flow::new();
    /// # let event = FlowEvent::ConnectionCancelled;
    /// if let FlowEvent::ConnectionCompleted(conn) = event {
    ///     flow.add_edge_from_connection(conn, StepEdge::default());
    /// }
    /// ```
    pub fn add_edge_from_connection(
        &mut self,
        connection: Connection,
        content: E,
    ) -> Option<String> {
        let id = connection.edge_id();

        if self.edge_lookup.contains_key(&id) {
            return None;
        }

        let edge = Edge::new(&id, &connection.source, &connection.target)
            .with_content(content)
            .with_source_handle(connection.source_handle)
            .with_target_handle(connection.target_handle);

        let _ = self.add_edge(edge);
        Some(id)
    }

    /// Checks if an edge exists with the given source, target, and handles.
    pub fn connection_exists(
        &self,
        source: &str,
        source_handle: Option<&str>,
        target: &str,
        target_handle: Option<&str>,
    ) -> bool {
        self.edges.iter().any(|e| {
            e.source == source
                && e.source_handle.as_deref() == source_handle
                && e.target == target
                && e.target_handle.as_deref() == target_handle
        })
    }

    // ========== Remove Operations ==========

    /// Internal: Removes a node by index and all its connected edges.
    ///
    /// Removes a node by index. Preserves insertion order for z-order tiebreaking
    /// and Tab cycling. Updates node_lookup accordingly.
    /// Selection is cleared if the removed node was selected.
    /// Returns None if index is out of bounds.
    pub(crate) fn remove_node_at(&mut self, idx: usize) -> Option<Node<N>> {
        if idx >= self.nodes.len() {
            return None;
        }

        let internal = self.nodes.remove(idx);
        let node_id = internal.id().to_owned();
        self.node_lookup.remove(&node_id);

        // Shift indices for all nodes after the removed one
        self.node_lookup
            .values_mut()
            .filter(|i| **i > idx)
            .for_each(|i| *i -= 1);
        self.invalidate_z_order();

        // Remove all edges connected to this node and rebuild edge_lookup
        self.edges
            .retain(|e| e.source != node_id && e.target != node_id);
        self.rebuild_edge_lookup();

        Some(internal.node)
    }

    /// Internal: Removes an edge by index.
    ///
    /// Selection is cleared if the removed edge was selected.
    /// Returns None if index is out of bounds.
    pub(crate) fn remove_edge_at(&mut self, idx: usize) -> Option<Edge<E>> {
        if idx >= self.edges.len() {
            return None;
        }

        let edge = self.edges.remove(idx);
        self.edge_lookup.remove(&edge.id);

        // Shift indices for all edges after the removed one
        self.edge_lookup
            .values_mut()
            .filter(|i| **i > idx)
            .for_each(|i| *i -= 1);

        Some(edge)
    }

    /// Rebuilds the node_lookup from the current nodes vec.
    fn rebuild_node_lookup(&mut self) {
        self.node_lookup.clear();
        for (i, node) in self.nodes.iter().enumerate() {
            self.node_lookup.insert(node.node.id.clone(), i);
        }
    }

    /// Rebuilds the edge_lookup from the current edges vec.
    fn rebuild_edge_lookup(&mut self) {
        self.edge_lookup.clear();
        for (i, edge) in self.edges.iter().enumerate() {
            self.edge_lookup.insert(edge.id.clone(), i);
        }
    }

    /// Removes a node by ID and all its connected edges.
    ///
    /// Also adjusts the selection if needed.
    ///
    /// Returns the removed node, or `None` if no node with that ID exists.
    pub fn remove_node(&mut self, id: &str) -> Option<Node<N>> {
        let idx = self.node_lookup.get(id).copied()?;
        self.remove_node_at(idx)
    }

    /// Removes an edge by ID.
    ///
    /// Also adjusts the selection if needed.
    ///
    /// Returns the removed edge, or `None` if no edge with that ID exists.
    pub fn remove_edge(&mut self, id: &str) -> Option<Edge<E>> {
        let idx = self.edge_lookup.get(id).copied()?;
        self.remove_edge_at(idx)
    }

    /// Retains only the nodes for which the predicate returns `true`.
    /// Edges connected to removed nodes are also removed.
    pub fn retain_nodes(&mut self, mut f: impl FnMut(&Node<N>) -> bool) {
        let removed_ids: Vec<String> = self
            .nodes
            .iter()
            .filter(|n| !f(&n.node))
            .map(|n| n.node.id.clone())
            .collect();

        if removed_ids.is_empty() {
            return;
        }

        self.nodes.retain(|n| f(&n.node));
        self.rebuild_node_lookup();

        // Remove edges connected to removed nodes
        self.edges
            .retain(|e| !removed_ids.contains(&e.source) && !removed_ids.contains(&e.target));
        self.rebuild_edge_lookup();

        self.invalidate_z_order();
        self.resolve_hierarchy();
    }

    /// Retains only the edges for which the predicate returns `true`.
    pub fn retain_edges(&mut self, f: impl FnMut(&Edge<E>) -> bool) {
        self.edges.retain(f);
        self.rebuild_edge_lookup();
    }

    /// Replaces all nodes. Edges referencing removed nodes are also removed.
    ///
    /// Validates the new node set (duplicate IDs, parent refs, handles).
    pub fn set_nodes(&mut self, nodes: Vec<Node<N>>) -> Result<(), Error> {
        let internal_nodes: Vec<InternalNode<N>> =
            nodes.into_iter().map(InternalNode::from_node).collect();

        // Validate: duplicate IDs, parent refs, handles
        let mut node_ids = std::collections::HashSet::with_capacity(internal_nodes.len());
        for node in &internal_nodes {
            if !node_ids.insert(node.id()) {
                return Err(Error::DuplicateNodeId {
                    node_id: node.id().to_string(),
                });
            }
        }
        for node in &internal_nodes {
            if let Some(parent_id) = &node.node.parent_id
                && !node_ids.contains(parent_id.as_str())
            {
                return Err(Error::InvalidParentReference {
                    node_id: node.id().to_string(),
                    parent_id: parent_id.clone(),
                });
            }
            Self::validate_handles(node.id(), &node.node.handles)?;
        }

        self.nodes = internal_nodes;
        self.rebuild_node_lookup();

        // Remove orphan edges
        self.edges.retain(|e| {
            self.node_lookup.contains_key(&e.source) && self.node_lookup.contains_key(&e.target)
        });
        self.rebuild_edge_lookup();

        self.invalidate_z_order();
        self.resolve_hierarchy();
        Ok(())
    }

    /// Replaces all edges.
    ///
    /// Validates the new edge set (duplicate IDs, node refs, self-loops).
    pub fn set_edges(&mut self, edges: Vec<Edge<E>>) -> Result<(), Error> {
        let mut edge_ids = std::collections::HashSet::with_capacity(edges.len());
        for edge in &edges {
            if !edge_ids.insert(edge.id.as_str()) {
                return Err(Error::DuplicateEdgeId {
                    edge_id: edge.id.clone(),
                });
            }
            if edge.source == edge.target {
                return Err(Error::SelfReferentialEdge {
                    edge_id: edge.id.clone(),
                    node_id: edge.source.clone(),
                });
            }
            if !self.node_lookup.contains_key(&edge.source) {
                return Err(Error::InvalidEdgeReference {
                    edge_id: edge.id.clone(),
                    node_id: edge.source.clone(),
                });
            }
            if !self.node_lookup.contains_key(&edge.target) {
                return Err(Error::InvalidEdgeReference {
                    edge_id: edge.id.clone(),
                    node_id: edge.target.clone(),
                });
            }
        }

        self.edges = edges;
        self.rebuild_edge_lookup();
        Ok(())
    }

    // ========== Update Operations ==========

    /// Sets the position of a node.
    ///
    /// Resolves the hierarchy on every call. To set many at once, use
    /// [`set_node_positions`](Self::set_node_positions), which resolves once for
    /// the whole batch.
    pub fn set_node_position(&mut self, id: &str, position: impl Into<Position>) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.position = position.into();
            self.resolve_hierarchy();
        }
    }

    /// Sets the positions of many nodes, resolving the hierarchy once.
    ///
    /// Takes anything that iterates ID and position pairs, so a layout algorithm's
    /// `HashMap` and a borrowed slice both work without rebuilding either. Unknown
    /// IDs are skipped.
    ///
    /// Prefer this to calling [`set_node_position`](Self::set_node_position) in a
    /// loop: each single-node call re-resolves the hierarchy, so a batch of N costs
    /// N passes over the graph instead of one.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rataflow::{Flow, Position};
    /// # let mut flow: Flow = Flow::new();
    /// flow.set_node_positions([
    ///     ("node1", Position::new(0.0, 0.0)),
    ///     ("node2", Position::new(50.0, 30.0)),
    /// ]);
    /// ```
    pub fn set_node_positions<I, S, P>(&mut self, positions: I)
    where
        I: IntoIterator<Item = (S, P)>,
        S: AsRef<str>,
        P: Into<Position>,
    {
        let mut changed = false;
        for (id, position) in positions {
            if let Some(&idx) = self.node_lookup.get(id.as_ref())
                && let Some(internal) = self.nodes.get_mut(idx)
            {
                internal.node.position = position.into();
                changed = true;
            }
        }
        if changed {
            self.resolve_hierarchy();
        }
    }

    /// Moves a node by a relative delta.
    pub fn move_node(&mut self, id: &str, delta: impl Into<Position>) {
        let delta = delta.into();
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.position.x += delta.x;
            internal.node.position.y += delta.y;
            self.resolve_hierarchy();
        }
    }

    /// Sets the dimensions of a node.
    pub fn set_node_dimensions(&mut self, id: &str, width: f64, height: f64) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.width = width;
            internal.node.height = height;
            self.resolve_hierarchy();
        }
    }

    /// Sets the z-index of a node.
    ///
    /// Z-index controls layering: higher values render on top.
    /// This does not affect positioning or hierarchy.
    pub fn set_node_z_index(&mut self, id: &str, z_index: i32) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.z_index = z_index;
            self.invalidate_z_order();
        }
    }

    /// Re-parents a node, keeping it where it appears on screen.
    ///
    /// [`Node::position`] is stored relative to the parent, so moving between
    /// parents means rebasing it. This does that rebase, which is the reason to
    /// reach for it rather than editing `parent_id` through
    /// [`set_nodes`](Self::set_nodes): the node stays visually put, and the caller
    /// never converts between relative and absolute coordinates.
    ///
    /// Pass `None` to detach a node to the top level.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NodeNotFound`] if either node is missing, and
    /// [`Error::CyclicParent`] if `parent` is `id` itself or sits below it — a
    /// cycle would leave both nodes unreachable from any root, and the layout pass
    /// walks down from roots.
    pub fn set_node_parent(&mut self, id: &str, parent: Option<&str>) -> Result<(), Error> {
        if self.internal_node(id).is_none() {
            return Err(Error::NodeNotFound {
                node_id: id.to_string(),
            });
        }

        if let Some(parent_id) = parent {
            if self.internal_node(parent_id).is_none() {
                return Err(Error::NodeNotFound {
                    node_id: parent_id.to_string(),
                });
            }
            // Walk up from the proposed parent: meeting `id` means it is a
            // descendant, so adopting it would close a loop.
            let mut ancestor = Some(parent_id.to_string());
            while let Some(current) = ancestor {
                if current == id {
                    return Err(Error::CyclicParent {
                        node_id: id.to_string(),
                        parent_id: parent_id.to_string(),
                    });
                }
                ancestor = self
                    .internal_node(&current)
                    .and_then(|n| n.node.parent_id.clone());
            }
        }

        // `absolute = parent_absolute + position + origin_offset`, and the origin
        // offset does not change here, so shifting by the difference between the
        // two parents holds the node still.
        let old_parent = self.parent_absolute_of(id).unwrap_or_default();
        let new_parent = parent
            .and_then(|p| self.internal_node(p))
            .map(|n| n.position_absolute)
            .unwrap_or_default();

        if let Some(node) = self.internal_node_mut(id) {
            node.node.position = node.node.position + old_parent - new_parent;
            node.node.parent_id = parent.map(str::to_string);
        }

        self.invalidate_z_order();
        self.resolve_hierarchy();
        Ok(())
    }

    /// Sets the hidden state of a node.
    pub fn set_node_hidden(&mut self, id: &str, hidden: bool) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.hidden = hidden;
        }
    }

    /// Sets whether a node can be selected.
    pub fn set_node_selectable(&mut self, id: &str, selectable: bool) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.selectable = selectable;
        }
    }

    /// Sets whether a node can be deleted.
    pub fn set_node_deletable(&mut self, id: &str, deletable: bool) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.deletable = deletable;
        }
    }

    /// Sets whether a node can be dragged.
    pub fn set_node_draggable(&mut self, id: &str, draggable: bool) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.draggable = draggable;
        }
    }

    /// Sets whether a node can be resized by dragging its bottom-right grip.
    pub fn set_node_resizable(&mut self, id: &str, resizable: bool) {
        if let Some(node) = self.internal_node_mut(id) {
            node.node.resizable = resizable;
        }
    }

    /// Sets whether a node's handles can participate in connections.
    pub fn set_node_connectable(&mut self, id: &str, connectable: bool) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.connectable = connectable;
        }
    }

    /// Sets whether a node blocks content behind it.
    pub fn set_node_opaque(&mut self, id: &str, opaque: bool) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.opaque = opaque;
        }
    }

    /// Sets the handle style for all handles on a node.
    ///
    /// Pass `None` to clear per-handle styles (revert to theme defaults).
    /// For direction-aware characters, use [`HandleStyle::directional`].
    pub fn set_handle_styles(&mut self, id: &str, style: Option<HandleStyle>) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.ensure_explicit_handles();
            for handle in &mut internal.node.handles {
                handle.style = style;
            }
            internal.update_handle_bounds();
        }
    }

    /// Sets the handle style for a single handle on a node.
    ///
    /// Pass `None` to clear the per-handle style (revert to theme default).
    pub fn set_handle_style(&mut self, node_id: &str, handle_id: &str, style: Option<HandleStyle>) {
        if let Some(&idx) = self.node_lookup.get(node_id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.ensure_explicit_handles();
            for handle in &mut internal.node.handles {
                if handle.id.as_deref() == Some(handle_id) {
                    handle.style = style;
                }
            }
            internal.update_handle_bounds();
        }
    }

    /// Sets the handle disabled style for all handles on a node.
    ///
    /// Same as [`set_handle_styles`](Self::set_handle_styles) but for the
    /// disabled (non-connectable) state.
    pub fn set_handle_disabled_styles(&mut self, id: &str, style: Option<HandleStyle>) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.ensure_explicit_handles();
            for handle in &mut internal.node.handles {
                handle.disabled_style = style;
            }
            internal.update_handle_bounds();
        }
    }

    /// Sets the handle disabled style for a single handle on a node.
    ///
    /// Same as [`set_handle_style`](Self::set_handle_style) but for the
    /// disabled (non-connectable) state.
    pub fn set_handle_disabled_style(
        &mut self,
        node_id: &str,
        handle_id: &str,
        style: Option<HandleStyle>,
    ) {
        if let Some(&idx) = self.node_lookup.get(node_id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.ensure_explicit_handles();
            for handle in &mut internal.node.handles {
                if handle.id.as_deref() == Some(handle_id) {
                    handle.disabled_style = style;
                }
            }
            internal.update_handle_bounds();
        }
    }

    /// Sets the hidden state for all handles on a node.
    pub fn set_handles_hidden(&mut self, id: &str, hidden: bool) {
        if let Some(&idx) = self.node_lookup.get(id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.ensure_explicit_handles();
            for handle in &mut internal.node.handles {
                handle.hidden = hidden;
            }
            internal.update_handle_bounds();
        }
    }

    /// Sets the hidden state for a single handle on a node.
    pub fn set_handle_hidden(&mut self, node_id: &str, handle_id: &str, hidden: bool) {
        if let Some(&idx) = self.node_lookup.get(node_id)
            && let Some(internal) = self.nodes.get_mut(idx)
        {
            internal.node.ensure_explicit_handles();
            for handle in &mut internal.node.handles {
                if handle.id.as_deref() == Some(handle_id) {
                    handle.hidden = hidden;
                }
            }
            internal.update_handle_bounds();
        }
    }

    /// Returns a mutable reference to a node's content.
    ///
    /// Use this to mutate custom data stored in the node's content.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #![allow(unused)]
    /// # use ratatui::buffer::Buffer;
    /// # use rataflow::{Flow, NodeContent, NodeRenderContext, StepEdge};
    /// # #[derive(Debug)]
    /// # struct MyContent { label: String, value: i32 }
    /// # impl NodeContent for MyContent {
    /// #     fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {}
    /// # }
    /// # let mut flow: Flow<MyContent, StepEdge> = Flow::new();
    /// if let Some(content) = flow.node_content_mut("node1") {
    ///     content.label = "New Label".to_string();
    ///     content.value = 42;
    /// }
    /// ```
    pub fn node_content_mut(&mut self, id: &str) -> Option<&mut N> {
        self.node_lookup
            .get(id)
            .and_then(|&idx| self.nodes.get_mut(idx))
            .map(|internal| &mut internal.node.content)
    }

    /// Returns an iterator over every node's ID and a mutable reference to its content.
    ///
    /// Use this to mutate custom data across every node in one pass. Pairing the ID
    /// with the content avoids the collect-then-loop that [`nodes`](Self::nodes) plus
    /// [`node_content_mut`](Self::node_content_mut) requires, where the read borrow
    /// has to end before the first write can start.
    ///
    /// Content only. Identity and geometry keep their setters, since those recompute
    /// derived state between writes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #![allow(unused)]
    /// # use ratatui::buffer::Buffer;
    /// # use rataflow::{Flow, NodeContent, NodeRenderContext, StepEdge};
    /// # #[derive(Debug)]
    /// # struct MyContent { label: String, editing: bool }
    /// # impl NodeContent for MyContent {
    /// #     fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {}
    /// # }
    /// # let mut flow: Flow<MyContent, StepEdge> = Flow::new();
    /// # let editing_id: Option<String> = None;
    /// for (id, content) in flow.nodes_content_mut() {
    ///     content.editing = editing_id.as_deref() == Some(id);
    /// }
    /// ```
    pub fn nodes_content_mut(&mut self) -> impl Iterator<Item = (&str, &mut N)> {
        self.nodes.iter_mut().map(|internal| {
            let Node { id, content, .. } = &mut internal.node;
            (id.as_str(), content)
        })
    }

    /// Sets the hidden state of an edge.
    pub fn set_edge_hidden(&mut self, id: &str, hidden: bool) {
        if let Some(&idx) = self.edge_lookup.get(id)
            && let Some(edge) = self.edges.get_mut(idx)
        {
            edge.hidden = hidden;
        }
    }

    /// Sets the label of an edge.
    pub fn set_edge_label(&mut self, id: &str, label: Option<String>) {
        if let Some(&idx) = self.edge_lookup.get(id)
            && let Some(edge) = self.edges.get_mut(idx)
        {
            edge.label = label;
        }
    }

    /// Sets whether an edge can be selected.
    pub fn set_edge_selectable(&mut self, id: &str, selectable: bool) {
        if let Some(&idx) = self.edge_lookup.get(id)
            && let Some(edge) = self.edges.get_mut(idx)
        {
            edge.selectable = selectable;
        }
    }

    /// Sets whether an edge can be deleted.
    pub fn set_edge_deletable(&mut self, id: &str, deletable: bool) {
        if let Some(&idx) = self.edge_lookup.get(id)
            && let Some(edge) = self.edges.get_mut(idx)
        {
            edge.deletable = deletable;
        }
    }

    /// Sets whether an edge is animated.
    pub fn set_edge_animated(&mut self, id: &str, animated: bool) {
        if let Some(&idx) = self.edge_lookup.get(id)
            && let Some(edge) = self.edges.get_mut(idx)
        {
            edge.animated = animated;
        }
    }

    /// Sets whether an edge can be reconnected.
    pub fn set_edge_reconnectable(&mut self, id: &str, reconnectable: Reconnectable) {
        if let Some(&idx) = self.edge_lookup.get(id)
            && let Some(edge) = self.edges.get_mut(idx)
        {
            edge.reconnectable = reconnectable;
        }
    }

    /// Returns a mutable reference to an edge's content.
    ///
    /// Use this to mutate custom data stored in the edge's content.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rataflow::{EdgeStyle, Flow};
    /// # let mut flow: Flow = Flow::new();
    /// if let Some(content) = flow.edge_content_mut("edge1") {
    ///     content.style = Some(EdgeStyle::dotted());
    /// }
    /// ```
    pub fn edge_content_mut(&mut self, id: &str) -> Option<&mut E> {
        self.edge_lookup
            .get(id)
            .and_then(|&idx| self.edges.get_mut(idx))
            .map(|edge| &mut edge.content)
    }

    /// Returns an iterator over every edge's ID and a mutable reference to its content.
    ///
    /// Use this to mutate custom data across every edge in one pass. Pairing the ID
    /// with the content avoids the collect-then-loop that [`edges`](Self::edges) plus
    /// [`edge_content_mut`](Self::edge_content_mut) requires, where the read borrow
    /// has to end before the first write can start.
    ///
    /// Content only. Endpoints keep [`reconnect_edge`](Self::reconnect_edge), since
    /// changing them rebuilds the edge index.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rataflow::{EdgeStyle, Flow};
    /// # let mut flow: Flow = Flow::new();
    /// # let dimmed: Vec<String> = Vec::new();
    /// for (id, content) in flow.edges_content_mut() {
    ///     if dimmed.iter().any(|d| d == id) {
    ///         content.style = Some(EdgeStyle::dotted());
    ///     }
    /// }
    /// ```
    pub fn edges_content_mut(&mut self) -> impl Iterator<Item = (&str, &mut E)> {
        self.edges.iter_mut().map(|edge| {
            let Edge { id, content, .. } = edge;
            (id.as_str(), content)
        })
    }

    // ========== Reconnection ==========

    /// Reconnects an existing edge to new endpoints, preserving all other properties.
    ///
    /// Removes the old edge and adds a new one with the same content, visibility,
    /// selectability, animation, label, and reconnectable settings but with updated
    /// source/target endpoints. Returns the new edge's ID, or `None` if the original
    /// edge was not found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rataflow::{Flow, FlowEvent};
    /// # let mut flow: Flow = Flow::new();
    /// # let event = FlowEvent::ConnectionCancelled;
    /// if let FlowEvent::ReconnectionCompleted { edge_id, new_connection, .. } = event {
    ///     flow.reconnect_edge(&edge_id, new_connection);
    /// }
    /// ```
    pub fn reconnect_edge(&mut self, edge_id: &str, new_connection: Connection) -> Option<String> {
        let old_edge = self.remove_edge(edge_id)?;

        let new_id = new_connection.edge_id();
        let new_edge = Edge::new(&new_id, &new_connection.source, &new_connection.target)
            .with_content(old_edge.content)
            .with_source_handle(new_connection.source_handle)
            .with_target_handle(new_connection.target_handle)
            .with_hidden(old_edge.hidden)
            .with_deletable(old_edge.deletable)
            .with_selectable(old_edge.selectable)
            .with_selected(old_edge.selected)
            .with_animated(old_edge.animated)
            .with_reconnectable(old_edge.reconnectable);

        let new_edge = if let Some(label) = old_edge.label {
            new_edge.with_label(label)
        } else {
            new_edge
        };

        let _ = self.add_edge(new_edge);
        Some(new_id)
    }

    // ========== Bulk Operations ==========

    /// Clears all nodes and edges.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.node_lookup.clear();
        self.edge_lookup.clear();
        self.drag_state = DragState::None;
        self.edge_preview = None;
        self.invalidate_z_order();
    }
}

/// The handle of `handle_type` sitting on `side`, or the node's first of that type.
///
/// The fallback keeps a floating edge attached to nodes that only carry handles on
/// some sides, rather than dropping the edge.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Position;
    use crate::ui::{StepEdge, TextContent};

    // --- re-parenting --------------------------------------------------------

    fn parenting_flow() -> Flow {
        let group = Node::new(
            "group",
            Position::new(10.0, 10.0),
            (100.0, 100.0),
            TextContent::from("group"),
        );
        let other = Node::new(
            "other",
            Position::new(200.0, 200.0),
            (50.0, 50.0),
            TextContent::from("other"),
        );
        let loose = Node::new(
            "loose",
            Position::new(30.0, 40.0),
            (10.0, 10.0),
            TextContent::from("loose"),
        );
        Flow::with_graph(vec![group, other, loose], Vec::<Edge<StepEdge>>::new()).unwrap()
    }

    #[test]
    fn re_parenting_holds_the_node_still() {
        let mut flow = parenting_flow();
        let before = flow.node_bounds("loose").unwrap();

        flow.set_node_parent("loose", Some("group")).unwrap();

        // Stored position is now relative to the group...
        assert_eq!(
            flow.node("loose").unwrap().position,
            Position::new(20.0, 30.0)
        );
        assert_eq!(
            flow.node("loose").unwrap().parent_id.as_deref(),
            Some("group")
        );
        // ...but it has not moved on screen, which is the whole point.
        assert_eq!(flow.node_bounds("loose").unwrap().position, before.position);
    }

    #[test]
    fn detaching_restores_absolute_position() {
        let mut flow = parenting_flow();
        flow.set_node_parent("loose", Some("group")).unwrap();
        let attached = flow.node_bounds("loose").unwrap();

        flow.set_node_parent("loose", None).unwrap();

        assert!(flow.node("loose").unwrap().parent_id.is_none());
        assert_eq!(
            flow.node("loose").unwrap().position,
            Position::new(30.0, 40.0)
        );
        assert_eq!(
            flow.node_bounds("loose").unwrap().position,
            attached.position
        );
    }

    #[test]
    fn re_parenting_between_parents_still_holds_position() {
        let mut flow = parenting_flow();
        flow.set_node_parent("loose", Some("group")).unwrap();
        let before = flow.node_bounds("loose").unwrap();

        flow.set_node_parent("loose", Some("other")).unwrap();

        assert_eq!(flow.node_bounds("loose").unwrap().position, before.position);
    }

    #[test]
    fn a_node_cannot_become_its_own_ancestor() {
        let mut flow = parenting_flow();
        flow.set_node_parent("loose", Some("group")).unwrap();

        // Direct self-reference.
        assert!(matches!(
            flow.set_node_parent("group", Some("group")),
            Err(Error::CyclicParent { .. })
        ));
        // And the indirect case: `loose` already sits under `group`.
        assert!(matches!(
            flow.set_node_parent("group", Some("loose")),
            Err(Error::CyclicParent { .. })
        ));
        // The rejected calls changed nothing.
        assert!(flow.node("group").unwrap().parent_id.is_none());
    }

    #[test]
    fn re_parenting_reports_missing_nodes() {
        let mut flow = parenting_flow();
        assert!(matches!(
            flow.set_node_parent("nope", Some("group")),
            Err(Error::NodeNotFound { .. })
        ));
        assert!(matches!(
            flow.set_node_parent("loose", Some("nope")),
            Err(Error::NodeNotFound { .. })
        ));
    }

    // --- world-space queries -------------------------------------------------

    /// Parent at (100, 100), child stored at a parent-relative (10, 10).
    fn parented_flow() -> Flow {
        let parent = Node::new(
            "parent",
            Position::new(100.0, 100.0),
            (200.0, 200.0),
            TextContent::from("p"),
        );
        let child = Node::new(
            "child",
            Position::new(10.0, 10.0),
            (20.0, 20.0),
            TextContent::from("c"),
        )
        .with_parent("parent");
        Flow::with_graph(vec![parent, child], vec![]).unwrap()
    }

    #[test]
    fn node_bounds_resolves_the_parent_offset() {
        let flow = parented_flow();
        // Node::position stays parent-relative; node_bounds answers in world space.
        assert_eq!(
            flow.node("child").unwrap().position,
            Position::new(10.0, 10.0)
        );
        let bounds = flow.node_bounds("child").expect("child exists");
        assert_eq!(bounds.position, Position::new(110.0, 110.0));
        assert_eq!(flow.node_bounds("missing"), None);
    }

    #[test]
    fn nodes_in_reports_intersecting_visible_nodes() {
        let mut flow = parented_flow();

        let hits: Vec<&str> = flow
            .nodes_in(Rect::new(
                Position::new(105.0, 105.0),
                crate::types::Dimensions::new(10.0, 10.0),
            ))
            .collect();
        assert!(
            hits.contains(&"child"),
            "child is at world (110,110): {hits:?}"
        );
        assert!(
            hits.contains(&"parent"),
            "parent spans that area too: {hits:?}"
        );

        // Far away: nothing.
        let none: Vec<&str> = flow
            .nodes_in(Rect::new(
                Position::new(-500.0, -500.0),
                crate::types::Dimensions::new(5.0, 5.0),
            ))
            .collect();
        assert!(none.is_empty(), "expected no hits, got {none:?}");

        // Hidden nodes are excluded.
        flow.set_node_hidden("child", true);
        let hits: Vec<&str> = flow
            .nodes_in(Rect::new(
                Position::new(105.0, 105.0),
                crate::types::Dimensions::new(10.0, 10.0),
            ))
            .collect();
        assert!(
            !hits.contains(&"child"),
            "hidden node must be skipped: {hits:?}"
        );
    }

    #[test]
    fn orphan_edges_resolve_to_none_rather_than_panicking() {
        let a = Node::new(
            "a",
            Position::new(0.0, 0.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let b = Node::new(
            "b",
            Position::new(50.0, 0.0),
            (20.0, 10.0),
            TextContent::from("b"),
        );
        let edge: Edge<StepEdge> = Edge::new("e", "a", "b");
        let mut flow = Flow::with_graph(vec![a, b], vec![edge]).unwrap();
        flow.remove_node("b");
        // remove_node cascades the edge, so re-add a dangling one the blunt way.
        flow.edges
            .push(Edge::<StepEdge>::new("orphan", "a", "gone"));
        assert!(flow.resolve_edge_handles(&flow.edges()[0]).is_none());
    }

    #[test]
    fn test_add_node_with_invalid_parent() {
        let mut state: Flow<TextContent, StepEdge> = Flow::new();

        let node = Node::new(
            "a",
            Position::new(0.0, 0.0),
            (10.0, 10.0),
            TextContent::from("A"),
        )
        .with_parent("nonexistent");

        let result = state.add_node(node);
        assert!(matches!(result, Err(Error::InvalidParentReference { .. })));
    }

    #[test]
    fn test_add_edge_invalid_source() {
        let mut state: Flow<TextContent, StepEdge> = Flow::new();

        state
            .add_node(Node::new(
                "a",
                Position::new(0.0, 0.0),
                (10.0, 10.0),
                TextContent::from("A"),
            ))
            .unwrap();

        let edge: Edge<StepEdge> = Edge::new("e1", "nonexistent", "a");
        let result = state.add_edge(edge);

        assert!(matches!(result, Err(Error::InvalidEdgeReference { .. })));
    }

    #[test]
    fn test_remove_node_removes_connected_edges() {
        let mut state: Flow<TextContent, StepEdge> = Flow::new();

        state
            .add_node(Node::new(
                "a",
                Position::new(0.0, 0.0),
                (10.0, 10.0),
                TextContent::from("A"),
            ))
            .unwrap();
        state
            .add_node(Node::new(
                "b",
                Position::new(20.0, 0.0),
                (10.0, 10.0),
                TextContent::from("B"),
            ))
            .unwrap();
        state
            .add_node(Node::new(
                "c",
                Position::new(40.0, 0.0),
                (10.0, 10.0),
                TextContent::from("C"),
            ))
            .unwrap();

        state.add_edge(Edge::new("e1", "a", "b")).unwrap();
        state.add_edge(Edge::new("e2", "b", "c")).unwrap();
        state.add_edge(Edge::new("e3", "a", "c")).unwrap();

        assert_eq!(state.edges().len(), 3);

        let removed = state.remove_node("b");
        assert!(removed.is_some());
        assert_eq!(state.nodes.len(), 2);
        assert_eq!(state.edges().len(), 1); // Only e3 remains
        assert!(state.edge("e3").is_some());
    }

    #[test]
    fn test_remove_node_adjusts_selection() {
        let mut state: Flow<TextContent, StepEdge> = Flow::new();

        state
            .add_node(Node::new(
                "a",
                Position::new(0.0, 0.0),
                (10.0, 10.0),
                TextContent::from("A"),
            ))
            .unwrap();
        state
            .add_node(Node::new(
                "b",
                Position::new(20.0, 0.0),
                (10.0, 10.0),
                TextContent::from("B"),
            ))
            .unwrap();
        state
            .add_node(Node::new(
                "c",
                Position::new(40.0, 0.0),
                (10.0, 10.0),
                TextContent::from("C"),
            ))
            .unwrap();

        // Select node c (by ID)
        state.select_node("c");
        assert!(state.node("c").unwrap().selected);

        // Remove node a - selection should remain on c (per-entity, order-preserving remove)
        state.remove_node("a");

        // c is still selected
        assert!(state.node("c").unwrap().selected);
    }

    #[test]
    fn test_add_node_ambiguous_handles() {
        use crate::types::{Handle, HandlePosition};

        let mut state: Flow<TextContent, StepEdge> = Flow::new();

        // Two source handles without IDs — ambiguous
        let node = Node::new(
            "a",
            Position::new(0.0, 0.0),
            (10.0, 10.0),
            TextContent::from("A"),
        )
        .with_handles(vec![
            Handle::source(HandlePosition::Right),
            Handle::source(HandlePosition::Bottom),
        ]);

        let result = state.add_node(node);
        assert!(matches!(result, Err(Error::AmbiguousHandles { .. })));
    }

    #[test]
    fn test_add_node_duplicate_handle_ids() {
        use crate::types::{Handle, HandlePosition};

        let mut state: Flow<TextContent, StepEdge> = Flow::new();

        // Two source handles with the same ID
        let node = Node::new(
            "a",
            Position::new(0.0, 0.0),
            (10.0, 10.0),
            TextContent::from("A"),
        )
        .with_handles(vec![
            Handle::source(HandlePosition::Right).with_id("out"),
            Handle::source(HandlePosition::Bottom).with_id("out"),
        ]);

        let result = state.add_node(node);
        assert!(matches!(result, Err(Error::DuplicateHandleId { .. })));
    }

    #[test]
    fn test_add_node_valid_multiple_handles() {
        use crate::types::{Handle, HandlePosition};

        let mut state: Flow<TextContent, StepEdge> = Flow::new();

        // Two source handles with distinct IDs — valid
        let node = Node::new(
            "a",
            Position::new(0.0, 0.0),
            (10.0, 10.0),
            TextContent::from("A"),
        )
        .with_handles(vec![
            Handle::source(HandlePosition::Right).with_id("out1"),
            Handle::source(HandlePosition::Bottom).with_id("out2"),
            Handle::target(HandlePosition::Left), // Single target, no ID needed
        ]);

        let result = state.add_node(node);
        assert!(result.is_ok());
    }

    // --- bulk content access -------------------------------------------------

    /// The pairing is the part worth guarding: a bulk iterator that zipped IDs and
    /// contents from separate passes could hand out a correct-looking mismatch.
    #[test]
    fn test_nodes_content_mut_pairs_id_with_own_content() {
        let mut flow: Flow<TextContent, StepEdge> = Flow::new();
        for id in ["a", "b", "c"] {
            flow.add_node(Node::new(
                id,
                Position::new(0.0, 0.0),
                (10.0, 10.0),
                TextContent::from(id),
            ))
            .unwrap();
        }

        for (id, content) in flow.nodes_content_mut() {
            *content = TextContent::from(format!("{id}!"));
        }

        for id in ["a", "b", "c"] {
            let content = flow.node(id).map(|n| n.content.text.clone()).unwrap();
            assert_eq!(
                content,
                format!("{id}!").into(),
                "content landed on the wrong node"
            );
        }
    }

    #[test]
    fn test_edges_content_mut_pairs_id_with_own_content() {
        let mut flow: Flow<TextContent, StepEdge> = Flow::new();
        for id in ["a", "b"] {
            flow.add_node(Node::new(
                id,
                Position::new(0.0, 0.0),
                (10.0, 10.0),
                TextContent::from(id),
            ))
            .unwrap();
        }
        flow.add_edge(Edge::new("e1", "a", "b")).unwrap();
        flow.add_edge(Edge::new("e2", "b", "a")).unwrap();

        for (id, content) in flow.edges_content_mut() {
            if id == "e1" {
                content.style = Some(crate::ui::EdgeStyle::dotted());
            }
        }

        assert!(flow.edge("e1").unwrap().content.style.is_some());
        assert!(
            flow.edge("e2").unwrap().content.style.is_none(),
            "style landed on the wrong edge"
        );
    }

    // --- bulk position writes -------------------------------------------------

    #[test]
    fn test_set_node_positions_applies_whole_batch() {
        let nodes = vec![
            Node::new("a", (0.0, 0.0), (10.0, 5.0), TextContent::from("A")),
            Node::new("b", (0.0, 0.0), (10.0, 5.0), TextContent::from("B")),
        ];
        let mut flow: Flow = Flow::with_graph(nodes, vec![]).unwrap();

        let mut positions: std::collections::HashMap<String, Position> =
            std::collections::HashMap::new();
        positions.insert("a".to_string(), Position::new(10.0, 20.0));
        positions.insert("b".to_string(), Position::new(30.0, 40.0));
        flow.set_node_positions(positions);

        assert_eq!(flow.node("a").unwrap().position, Position::new(10.0, 20.0));
        assert_eq!(flow.node("b").unwrap().position, Position::new(30.0, 40.0));
    }

    /// A child's absolute position is derived, so the batch is only correct if the
    /// hierarchy resolves after every write rather than between them.
    #[test]
    fn test_set_node_positions_resolves_hierarchy_after_the_batch() {
        let parent = Node::new("p", (0.0, 0.0), (100.0, 100.0), TextContent::from("P"));
        let child =
            Node::new("c", (5.0, 5.0), (10.0, 5.0), TextContent::from("C")).with_parent("p");
        let mut flow: Flow = Flow::with_graph(vec![parent, child], vec![]).unwrap();

        flow.set_node_positions([
            ("p", Position::new(50.0, 50.0)),
            ("c", Position::new(5.0, 5.0)),
        ]);

        assert_eq!(
            flow.node_bounds("c").unwrap().position.x,
            55.0,
            "child should sit at parent origin plus its relative position"
        );
    }

    #[test]
    fn test_set_node_positions_ignores_unknown_ids() {
        let nodes = vec![Node::new(
            "a",
            (5.0, 5.0),
            (10.0, 5.0),
            TextContent::from("A"),
        )];
        let mut flow: Flow = Flow::with_graph(nodes, vec![]).unwrap();

        flow.set_node_positions([("nonexistent", Position::new(99.0, 99.0))]);

        assert_eq!(flow.node("a").unwrap().position, Position::new(5.0, 5.0));
    }
}
