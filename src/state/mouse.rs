//! Mouse interaction operations for Flow.

use super::Flow;
use crate::actions::{EventResponse, FlowEvent};
use crate::content::{EdgeContent, NodeContent};
use crate::types::{Connection, ConnectionMode, HandleType, Position};

/// Default radius (in world units) for handle hit detection.
pub(crate) const DEFAULT_HANDLE_HIT_RADIUS: f64 = 1.5;

/// Default distance threshold (in world units) for edge hit detection.
pub(crate) const DEFAULT_EDGE_HIT_THRESHOLD: f64 = 1.5;

/// Default radius (in world units) for finding target handles when creating connections.
pub(crate) const DEFAULT_CONNECTION_RADIUS: f64 = 2.0;

/// Default distance threshold (in world units) before node drag is initiated.
pub(crate) const DEFAULT_NODE_DRAG_THRESHOLD: f64 = 2.0;

/// Default distance from the bottom-right grip that starts a resize drag, in world
/// units.
pub(crate) const DEFAULT_RESIZE_HANDLE_RADIUS: f64 = 1.0;

/// What sits at a point on the canvas.
///
/// Returned by [`Flow::pick`](crate::Flow::pick). Borrows the IDs rather than
/// cloning them — this reports what is there, it does not hand over anything you
/// can change. Mutate through the graph operations instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick<'a> {
    /// A handle on a node.
    Handle {
        /// The node the handle belongs to.
        node_id: &'a str,
        /// The handle's ID, or `None` when it is the node's only one of its type.
        handle_id: Option<&'a str>,
        /// Whether it is a source or target handle.
        handle_type: HandleType,
    },
    /// A node body.
    Node {
        /// The node's ID.
        node_id: &'a str,
    },
    /// An edge.
    Edge {
        /// The edge's ID.
        edge_id: &'a str,
    },
    /// Nothing — empty canvas.
    Nothing,
}

/// Current drag operation state.
#[derive(Debug, Clone, Default)]
pub(crate) enum DragState {
    /// No active drag operation.
    #[default]
    None,

    /// Clicked on a node, waiting for mouse up to emit `NodeClicked`.
    /// Used for non-draggable nodes, multi-select clicks, and non-connectable handle clicks
    /// so that `NodeClicked` consistently fires on mouse up (matching draggable node behavior).
    AwaitingNodeClick { node_id: String },

    /// Creating a new connection from a handle.
    /// Preview state lives in [`EdgePreview`](super::edge_preview::EdgePreview).
    CreatingConnection,

    /// Moving a node.
    MovingNode {
        node_id: String,
        offset: Position,
        parent_absolute: Option<Position>,
        /// Starting position for threshold calculation.
        start_pos: Position,
        /// Whether drag threshold was exceeded (distinguishes click from drag).
        drag_started: bool,
        /// Whether selection was already performed during this drag operation.
        /// Used for deferred selection: when `select_nodes_on_drag` is true with
        /// a non-zero drag threshold, selection happens when the threshold is exceeded
        /// rather than on mouse-down.
        selected: bool,
    },

    /// Reconnecting an existing edge to a different handle.
    /// The edge being reconnected.
    ReconnectingEdge {
        /// The edge being reconnected.
        edge_id: String,
    },

    /// Right button pressed; still undecided between a context menu and a box.
    AwaitingContextMenu {
        /// The event to emit if the press turns out to be a click.
        event: FlowEvent,
        /// Where the press landed, in world coordinates.
        anchor: Position,
    },

    /// Dragging a selection box over the canvas.
    SelectingBox {
        /// Where the drag began, in world coordinates.
        anchor: Position,
        /// Where the cursor is now, in world coordinates.
        current: Position,
    },

    /// Resizing a node by its bottom-right corner.
    ResizingNode {
        node_id: String,
        /// The node's world bounds when the drag began. Only the size changes, so
        /// the top-left is the fixed anchor throughout.
        initial: crate::types::Rect,
    },

    /// Panning the viewport.
    Panning {
        anchor_canvas: Position,
        initial_viewport: Position,
    },
}

impl DragState {
    /// Returns true if any drag is active.
    pub fn is_active(&self) -> bool {
        !matches!(self, DragState::None)
    }
}

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    /// Handles mouse down at the given position.
    ///
    /// This performs hit testing and starts the appropriate drag operation:
    /// - Click on source handle: starts connection creation (if connectable)
    /// - Click on target handle: starts connection creation in Loose mode (if connectable), otherwise selects node
    /// - Click on node body: moves node if `allow_node_drag` is true, otherwise just selects
    /// - Click on edge: selects the edge (or toggles with multi-select)
    /// - Click on empty space: starts panning
    ///
    /// When `multi_select` is true, selection is toggled without clearing other
    /// selections, and node dragging is disabled.
    ///
    /// # Arguments
    ///
    /// * `mouse_world_pos` - Mouse position in world coordinates
    /// * `mouse_canvas_pos` - Mouse position in canvas coordinates (needed for panning)
    /// * `allow_node_drag` - Whether clicking on node body should start dragging
    /// * `multi_select` - Whether to toggle selection without clearing
    ///
    pub(crate) fn on_mouse_down(
        &mut self,
        mouse_world_pos: Position,
        mouse_canvas_pos: Position,
        allow_node_drag: bool,
        multi_select: bool,
    ) -> EventResponse {
        let mut events = match self.hit_test(mouse_world_pos) {
            MouseHit::Handle {
                node_id,
                handle_id,
                handle_type,
            } => {
                // Try reconnection first (selected reconnectable edge at this handle)
                if let Some(events) = self.try_start_reconnection(
                    &node_id,
                    handle_id.as_deref(),
                    handle_type,
                    mouse_world_pos,
                ) {
                    events
                } else {
                    self.handle_new_connection(
                        node_id,
                        handle_id,
                        handle_type,
                        mouse_world_pos,
                        multi_select,
                    )
                }
            }
            MouseHit::Node { node_id } => {
                if multi_select {
                    // Multi-select: toggle without drag (only if selectable)
                    let selectable = self
                        .internal_node(&node_id)
                        .is_some_and(|n| n.node.selectable);
                    if selectable {
                        self.toggle_node_selection(&node_id);
                    }
                    self.drag_state = DragState::AwaitingNodeClick { node_id };
                } else if allow_node_drag {
                    // Normal click with drag (if node is draggable)
                    if let Some(node) = self.internal_node(&node_id) {
                        if node.node.draggable {
                            let node_pos = node.position_absolute;
                            let selectable = node.node.selectable;
                            let node_selected = node.node.selected;
                            let parent_absolute = self.parent_absolute_of(&node_id);

                            // Selection timing depends on threshold:
                            // - threshold == 0: immediate on mouse-down
                            // - threshold > 0: deferred to when threshold is exceeded
                            let selected = if self.node_drag_threshold == 0.0 {
                                if self.select_nodes_on_drag && selectable {
                                    self.select_node(&node_id);
                                    true
                                } else {
                                    // Deselect others when dragging non-selectable
                                    // or when select_nodes_on_drag is false
                                    if self.deselect_on_drag && !node_selected {
                                        self.clear_selection();
                                    }
                                    false
                                }
                            } else {
                                // Deferred to threshold exceeded in on_mouse_drag
                                false
                            };

                            self.drag_state = DragState::MovingNode {
                                node_id: node_id.clone(),
                                offset: node_pos - mouse_world_pos,
                                parent_absolute,
                                start_pos: mouse_world_pos,
                                drag_started: false,
                                selected,
                            };
                            // NodeDragStarted deferred until drag threshold is exceeded
                        } else {
                            // Node not draggable — select if selectable, emit click on mouse-up
                            let selectable = self
                                .internal_node(&node_id)
                                .is_some_and(|n| n.node.selectable);
                            if selectable {
                                self.select_node(&node_id);
                            }
                            self.drag_state = DragState::AwaitingNodeClick { node_id };
                        }
                    }
                } else {
                    let selectable = self
                        .internal_node(&node_id)
                        .is_some_and(|n| n.node.selectable);
                    if selectable {
                        self.select_node(&node_id);
                    }
                    self.drag_state = DragState::AwaitingNodeClick { node_id };
                }
                vec![]
            }
            MouseHit::Edge { edge_id } => {
                if multi_select {
                    self.toggle_edge_selection(&edge_id);
                } else {
                    self.select_edge(&edge_id);
                }
                vec![FlowEvent::EdgeClicked { edge_id }]
            }
            MouseHit::Nothing => {
                // Click on empty space: start panning
                if self.deselect_on_pane_click {
                    self.clear_selection();
                }
                self.drag_state = DragState::Panning {
                    anchor_canvas: mouse_canvas_pos,
                    initial_viewport: self.viewport.offset(),
                };
                vec![FlowEvent::PaneClicked {
                    x: mouse_world_pos.x,
                    y: mouse_world_pos.y,
                }]
            }
        };

        if let Some(sel) = self.maybe_selection_changed_event() {
            events.push(sel);
        }

        if events.is_empty() {
            EventResponse::Handled
        } else {
            EventResponse::Event(events)
        }
    }

    /// Handles a handle click that didn't match a reconnection.
    ///
    /// Source handles always try to start a connection. Target handles only start
    /// connections in Loose mode. Falls back to node selection otherwise.
    fn handle_new_connection(
        &mut self,
        node_id: String,
        handle_id: Option<String>,
        handle_type: HandleType,
        mouse_world_pos: Position,
        multi_select: bool,
    ) -> Vec<FlowEvent> {
        // Source handles are always connectable; target handles only in Loose mode
        let can_connect = match handle_type {
            HandleType::Source => true,
            HandleType::Target => self.connection_mode == ConnectionMode::Loose,
        };

        let connectable = can_connect
            && self.internal_node(&node_id).is_some_and(|node| {
                node.handle_bounds
                    .find(handle_id.as_deref(), handle_type)
                    .is_some_and(|h| {
                        node.node.connectable && h.connectable && h.connectable_start && !h.hidden
                    })
            });

        if connectable {
            let selectable = self
                .internal_node(&node_id)
                .is_some_and(|n| n.node.selectable);
            if selectable {
                self.select_node(&node_id);
            }
            self.set_edge_preview_raw(
                node_id.clone(),
                handle_id.clone(),
                handle_type,
                mouse_world_pos,
            );
            self.drag_state = DragState::CreatingConnection;
            vec![FlowEvent::ConnectionStarted { node_id, handle_id }]
        } else {
            let selectable = self
                .internal_node(&node_id)
                .is_some_and(|n| n.node.selectable);
            if selectable {
                if multi_select {
                    self.toggle_node_selection(&node_id);
                } else {
                    self.select_node(&node_id);
                }
            }
            self.drag_state = DragState::AwaitingNodeClick { node_id };
            vec![]
        }
    }

    /// Attempts to start a reconnection from the given handle.
    ///
    /// Returns `Some(events)` if a reconnectable edge was found and the drag was started,
    /// `None` to fall through to normal connection logic.
    fn try_start_reconnection(
        &mut self,
        node_id: &str,
        handle_id: Option<&str>,
        handle_type: HandleType,
        mouse_world_pos: Position,
    ) -> Option<Vec<FlowEvent>> {
        let (edge_id, opposite_node, opposite_handle) =
            self.find_reconnectable_edge_at(node_id, handle_id, handle_type)?;

        let selectable = self
            .internal_node(node_id)
            .is_some_and(|n| n.node.selectable);
        if selectable {
            self.select_node(node_id);
        }
        self.set_edge_preview_raw(
            opposite_node,
            opposite_handle,
            handle_type.opposite(),
            mouse_world_pos,
        );
        self.drag_state = DragState::ReconnectingEdge {
            edge_id: edge_id.clone(),
        };
        Some(vec![FlowEvent::ReconnectionStarted {
            edge_id,
            handle_type,
        }])
    }

    /// Performs a hit test at the given world position (zoom-independent behavior).
    ///
    /// Iterates nodes in reverse z-order (front to back). For each node, checks
    /// handles first (smaller targets get priority within the same node), then
    /// the node body. When a node body is hit, checks if any edge at that position
    /// has a higher implicit z-index (derived from `max(source_z, target_z)`), and
    /// prefers the edge if so. This allows edges between children to be clickable
    /// through a transparent parent.
    ///
    /// Requires [`ensure_z_order`](Self::ensure_z_order) to have been called
    /// before this method (done at the top of `handle_mouse_event`).
    pub(crate) fn hit_test(&self, world_pos: Position) -> MouseHit {
        // Check nodes in reverse z-order (front to back): handles then body per node.
        for &idx in self.z_order_cache.iter().rev() {
            let node = &self.nodes[idx];
            if node.node.hidden {
                continue;
            }

            // Quick bounds check: skip nodes far from click position
            // Expand bounds by handle_hit_radius to account for handles on edges
            let bounds = node.bounds();
            let expanded_bounds = crate::types::Rect::new(
                Position::new(
                    bounds.position.x - self.handle_hit_radius,
                    bounds.position.y - self.handle_hit_radius,
                ),
                crate::types::Dimensions::new(
                    bounds.dimensions.width + self.handle_hit_radius * 2.0,
                    bounds.dimensions.height + self.handle_hit_radius * 2.0,
                ),
            );
            if !expanded_bounds.contains_point(&world_pos) {
                continue;
            }

            // Check handles (source then target)
            for (handles, handle_type) in [
                (&node.handle_bounds.source, HandleType::Source),
                (&node.handle_bounds.target, HandleType::Target),
            ] {
                for handle in handles {
                    if handle.hidden {
                        continue;
                    }
                    if handle.absolute_position.distance_to(&world_pos) <= self.handle_hit_radius {
                        return MouseHit::Handle {
                            node_id: node.node.id.clone(),
                            handle_id: handle.id.clone(),
                            handle_type,
                        };
                    }
                }
            }

            // Check node body (nodes that are neither selectable nor draggable are transparent)
            if (node.node.selectable || node.node.draggable) && bounds.contains_point(&world_pos) {
                let node_z = node.effective_z;

                // Before returning the node hit, check if any edge at this position
                // has a higher implicit z-index (from its endpoints).
                if let Some(edge_hit) = self.edge_hit_above(world_pos, node_z) {
                    return edge_hit;
                }

                return MouseHit::Node {
                    node_id: node.node.id.clone(),
                };
            }
        }

        // Edge hit testing (no node was hit)
        self.edge_hit_test(world_pos)
    }

    /// Checks if any edge at `world_pos` has an implicit z-index above `min_z`.
    ///
    /// Implicit edge z = `max(source_effective_z, target_effective_z)`.
    fn edge_hit_above(&self, world_pos: Position, min_z: i32) -> Option<MouseHit> {
        for edge in self.edges.iter().rev() {
            if edge.hidden || !edge.selectable {
                continue;
            }

            let source_idx = self.node_lookup.get(edge.source.as_str()).copied();
            let target_idx = self.node_lookup.get(edge.target.as_str()).copied();

            if let (Some(s_idx), Some(t_idx)) = (source_idx, target_idx) {
                let source_z = self.nodes[s_idx].effective_z;
                let target_z = self.nodes[t_idx].effective_z;
                let edge_z = source_z.max(target_z);

                if edge_z <= min_z {
                    continue;
                }

                if self.edge_hits_at(edge, world_pos) {
                    return Some(MouseHit::Edge {
                        edge_id: edge.id.clone(),
                    });
                }
            }
        }
        None
    }

    /// Edge hit testing fallback (no node was hit at this position).
    fn edge_hit_test(&self, world_pos: Position) -> MouseHit {
        for edge in self.edges.iter().rev() {
            if edge.hidden || !edge.selectable {
                continue;
            }
            if self.edge_hits_at(edge, world_pos) {
                return MouseHit::Edge {
                    edge_id: edge.id.clone(),
                };
            }
        }
        MouseHit::Nothing
    }

    /// Starts a selection box if [`selection_on_drag`](Self::selection_on_drag) is
    /// on and the press landed on empty canvas.
    ///
    /// Pressing a node still drags the node — the flag redirects the pane gesture,
    /// not every gesture.
    pub(crate) fn try_start_selection_box(&mut self, world_pos: Position) -> Option<EventResponse> {
        if !self.selection_on_drag || !matches!(self.hit_test(world_pos), MouseHit::Nothing) {
            return None;
        }
        self.drag_state = DragState::SelectingBox {
            anchor: world_pos,
            current: world_pos,
        };
        Some(EventResponse::Handled)
    }

    /// Starts a resize if the press landed on a resizable node's bottom-right grip.
    ///
    /// Only that corner resizes. The others would have to move the node's position
    /// as well as its size, and a node dragged by a corner that is itself moving is
    /// far harder to aim than it looks.
    ///
    /// Selection is not required: the grip is drawn on every resizable node, so it
    /// should work wherever it is visible.
    pub(crate) fn try_start_resize(&mut self, world_pos: Position) -> Option<EventResponse> {
        let hit = self.nodes.iter().find_map(|internal| {
            if !internal.node.resizable || internal.node.hidden {
                return None;
            }
            let bounds = internal.bounds();
            let grip = Position::new(bounds.right(), bounds.bottom());
            (grip.distance_to(&world_pos) <= self.resize_handle_radius)
                .then(|| (internal.node.id.clone(), bounds))
        })?;

        let (node_id, initial) = hit;
        self.drag_state = DragState::ResizingNode {
            node_id: node_id.clone(),
            initial,
        };
        Some(EventResponse::Event(vec![FlowEvent::NodeResizeStarted {
            node_id,
        }]))
    }

    /// Applies a resize drag. The top-left is fixed, so only the size changes.
    ///
    /// Because position never moves, there is nothing to accumulate across drag
    /// events — the size is always derived afresh from the initial bounds.
    fn apply_resize(&mut self, node_id: &str, initial: crate::types::Rect, cursor: Position) {
        let width = (cursor.x - initial.x()).max(self.min_node_size.width);
        let height = (cursor.y - initial.y()).max(self.min_node_size.height);
        if let Some(node) = self.internal_node_mut(node_id) {
            node.node.width = width;
            node.node.height = height;
        }
        self.resolve_hierarchy();
    }

    /// Selects every visible node intersecting the dragged box.
    fn select_within_box(&mut self, anchor: Position, current: Position) {
        let area = crate::types::Rect::new(
            Position::new(anchor.x.min(current.x), anchor.y.min(current.y)),
            crate::types::Dimensions::new(
                (current.x - anchor.x).abs(),
                (current.y - anchor.y).abs(),
            ),
        );
        let hits: Vec<String> = self.nodes_in(area).map(str::to_string).collect();
        self.clear_selection();
        for id in hits {
            if self.node(&id).is_some_and(|n| n.selectable) {
                self.toggle_node_selection(&id);
            }
        }
    }

    /// Reports what sits at a world position, using the same pick as a click.
    ///
    /// For reacting to the user's own clicks, prefer the events — `NodeClicked`,
    /// `EdgeClicked`, `NodeContextMenu`. Reach for this when an app has to know
    /// what is under the cursor *before* deciding whether to let the flow act on
    /// it: modal interactions like "click a node to finish this connection", where
    /// the answer changes whether the event should be forwarded at all.
    ///
    /// Takes `&mut self` because it refreshes the z-order cache when the graph has
    /// changed since the last render; the pick itself changes nothing.
    pub fn pick(&mut self, world_pos: Position) -> Pick<'_> {
        self.ensure_z_order();
        match self.hit_test(world_pos) {
            MouseHit::Handle {
                node_id,
                handle_id,
                handle_type,
            } => {
                // Re-borrow from the graph so the result carries no owned copies.
                let node = self
                    .internal_node(&node_id)
                    .expect("hit test returned a live node");
                let handle_id = handle_id.and_then(|wanted| {
                    node.handle_bounds
                        .by_type(handle_type)
                        .iter()
                        .find_map(|h| h.id.as_deref().filter(|id| *id == wanted))
                });
                Pick::Handle {
                    node_id: node.node.id.as_str(),
                    handle_id,
                    handle_type,
                }
            }
            MouseHit::Node { node_id } => Pick::Node {
                node_id: self
                    .internal_node(&node_id)
                    .expect("hit test returned a live node")
                    .node
                    .id
                    .as_str(),
            },
            MouseHit::Edge { edge_id } => Pick::Edge {
                edge_id: self
                    .edge(&edge_id)
                    .expect("hit test returned a live edge")
                    .id
                    .as_str(),
            },
            MouseHit::Nothing => Pick::Nothing,
        }
    }

    /// Right button pressed: decide later.
    ///
    /// The menu event is held rather than emitted, because the same press may turn
    /// into a selection box. That also aligns right-click with left — `NodeClicked`
    /// has always fired on release, not on press.
    pub(crate) fn on_right_down(&mut self, world_pos: Position) -> EventResponse {
        let event = match self.hit_test(world_pos) {
            MouseHit::Node { node_id } | MouseHit::Handle { node_id, .. } => {
                FlowEvent::NodeContextMenu { node_id }
            }
            MouseHit::Edge { edge_id } => FlowEvent::EdgeContextMenu { edge_id },
            MouseHit::Nothing => FlowEvent::PaneContextMenu {
                x: world_pos.x,
                y: world_pos.y,
            },
        };
        self.drag_state = DragState::AwaitingContextMenu {
            event,
            anchor: world_pos,
        };
        EventResponse::Handled
    }

    /// Right button moved: past the drag threshold this becomes a selection box.
    pub(crate) fn on_right_drag(&mut self, world_pos: Position) -> EventResponse {
        if let DragState::AwaitingContextMenu { anchor, .. } = self.drag_state
            && anchor.distance_to(&world_pos) > self.node_drag_threshold
        {
            self.drag_state = DragState::SelectingBox {
                anchor,
                current: world_pos,
            };
        }
        if let DragState::SelectingBox { current, .. } = &mut self.drag_state {
            *current = world_pos;
        }
        EventResponse::Handled
    }

    /// Right button released: emit the held menu, or commit the box.
    pub(crate) fn on_right_up(&mut self) -> EventResponse {
        match std::mem::take(&mut self.drag_state) {
            DragState::AwaitingContextMenu { event, .. } => EventResponse::Event(vec![event]),
            DragState::SelectingBox { anchor, current } => {
                self.select_within_box(anchor, current);
                let mut events = Vec::new();
                if let Some(sel) = self.maybe_selection_changed_event() {
                    events.push(sel);
                }
                EventResponse::Event(events)
            }
            other => {
                self.drag_state = other;
                EventResponse::NotHandled
            }
        }
    }

    /// Tests whether a single edge is hit at the given world position.
    fn edge_hits_at(&self, edge: &crate::types::Edge<E>, world_pos: Position) -> bool {
        if let Some((source_handle, target_handle)) = self.resolve_edge_handles(edge) {
            edge.content.hit_test(
                world_pos,
                &crate::content::EdgePathContext {
                    from: source_handle.absolute_position,
                    to: target_handle.absolute_position,
                    source_position: source_handle.position,
                    target_position: target_handle.position,
                    source_bounds: self.node_bounds(&edge.source).unwrap_or_default(),
                    target_bounds: self.node_bounds(&edge.target),
                },
                self.edge_hit_threshold,
            )
        } else {
            false
        }
    }

    /// Updates the current drag operation with a new mouse position.
    ///
    /// For connection creation: updates the preview line endpoint and validates hover target.
    /// For node movement: moves the node (after threshold is exceeded).
    /// For panning: updates the viewport.
    ///
    /// Returns [`EventResponse::Event(FlowEvent::NodeDragStarted)`] when node drag threshold
    /// is first exceeded, otherwise [`EventResponse::Handled`].
    ///
    /// # Arguments
    ///
    /// * `mouse_world_pos` - Current mouse position in world coordinates
    /// * `mouse_canvas_pos` - Current mouse position in canvas coordinates (needed for panning)
    pub(crate) fn on_mouse_drag(
        &mut self,
        mouse_world_pos: Position,
        mouse_canvas_pos: Position,
    ) -> EventResponse {
        self.last_mouse_canvas_pos = Some(mouse_canvas_pos);
        let mut state = std::mem::take(&mut self.drag_state);
        let response = match &mut state {
            DragState::CreatingConnection | DragState::ReconnectingEdge { .. } => {
                self.update_edge_preview_to_position(mouse_world_pos);
                EventResponse::Handled
            }
            DragState::MovingNode {
                node_id,
                offset,
                parent_absolute,
                start_pos,
                drag_started,
                selected,
            } => {
                // Check if we've exceeded the drag threshold
                let distance = mouse_world_pos.distance_to(start_pos);
                let threshold_exceeded = distance > self.node_drag_threshold;

                if !*drag_started && !threshold_exceeded {
                    // Still within threshold - don't move yet
                    EventResponse::Handled
                } else {
                    let just_started = !*drag_started && threshold_exceeded;

                    // Deferred selection (threshold > 0 case).
                    // Select on drag start, not mouse-down.
                    if just_started && !*selected {
                        let selectable = self
                            .internal_node(node_id)
                            .is_some_and(|n| n.node.selectable);
                        if self.select_nodes_on_drag && selectable {
                            self.select_node(node_id);
                            *selected = true;
                        } else {
                            // Deselect others when dragging non-selectable
                            // or when select_nodes_on_drag is false
                            if self.deselect_on_drag {
                                let node_selected =
                                    self.internal_node(node_id).is_some_and(|n| n.node.selected);
                                if !node_selected {
                                    self.clear_selection();
                                }
                            }
                        }
                    }

                    let new_absolute_pos = mouse_world_pos + *offset;

                    // Convert absolute position to relative (for child nodes)
                    let new_relative_pos = if let Some(parent_pos) = parent_absolute {
                        new_absolute_pos - *parent_pos
                    } else {
                        new_absolute_pos
                    };

                    // Update node position by ID
                    if let Some(node) = self.internal_node_mut(node_id) {
                        node.node.position = new_relative_pos;
                    }

                    // Defer hierarchy resolution until render or mouse_up.
                    // Multiple drag events between frames only need one resolve.
                    self.drag_hierarchy_pending = true;

                    *drag_started = *drag_started || just_started;

                    let mut events = vec![if just_started {
                        FlowEvent::NodeDragStarted {
                            node_id: node_id.clone(),
                        }
                    } else {
                        FlowEvent::NodeDragged {
                            node_id: node_id.clone(),
                        }
                    }];
                    if let Some(sel) = self.maybe_selection_changed_event() {
                        events.push(sel);
                    }
                    EventResponse::Event(events)
                }
            }
            DragState::SelectingBox { current, .. } => {
                *current = mouse_world_pos;
                EventResponse::Handled
            }
            DragState::ResizingNode { node_id, initial } => {
                let (node_id, initial) = (node_id.clone(), *initial);
                self.apply_resize(&node_id, initial, mouse_world_pos);
                EventResponse::Event(vec![FlowEvent::NodeResized { node_id }])
            }
            DragState::Panning {
                anchor_canvas,
                initial_viewport,
            } => {
                let delta = mouse_canvas_pos - *anchor_canvas;
                self.viewport.x = initial_viewport.x + delta.x;
                self.viewport.y = initial_viewport.y + delta.y;
                self.viewport_changed_response()
            }
            _ => EventResponse::Handled,
        };
        self.drag_state = state;
        response
    }

    /// Completes the current drag operation.
    ///
    /// Returns an [`EventResponse`] indicating what happened:
    /// - [`FlowEvent::NodeClicked`] if a node was clicked without dragging
    /// - [`FlowEvent::ConnectionCompleted`] if a valid connection was made
    /// - [`FlowEvent::ConnectionCancelled`] if connection was started but cancelled
    /// - [`FlowEvent::NodeDragEnded`] if node drag ended
    /// - [`EventResponse::Handled`] for panning completion or no-op
    ///
    /// Connections are normalized: starting from a target handle swaps source/target.
    pub(crate) fn on_mouse_up(&mut self, _mouse_world_pos: Position) -> EventResponse {
        self.last_mouse_canvas_pos = None;
        match std::mem::take(&mut self.drag_state) {
            DragState::AwaitingNodeClick { node_id } => {
                EventResponse::Event(vec![FlowEvent::NodeClicked { node_id }])
            }
            DragState::CreatingConnection => {
                let event = match self.take_edge_preview_connection() {
                    Some(connection) => FlowEvent::ConnectionCompleted(connection),
                    None => FlowEvent::ConnectionCancelled,
                };
                EventResponse::Event(vec![event])
            }
            DragState::ReconnectingEdge { edge_id } => {
                let event = match self.take_edge_preview_connection() {
                    Some(new_connection) => {
                        // Build old connection from the edge (if it still exists)
                        if let Some(e) = self.edge(&edge_id) {
                            FlowEvent::ReconnectionCompleted {
                                edge_id,
                                old_connection: Connection::new(
                                    &e.source,
                                    e.source_handle.clone(),
                                    &e.target,
                                    e.target_handle.clone(),
                                ),
                                new_connection,
                            }
                        } else {
                            // Edge removed during drag
                            FlowEvent::ReconnectionCancelled { edge_id }
                        }
                    }
                    None => FlowEvent::ReconnectionCancelled { edge_id },
                };
                EventResponse::Event(vec![event])
            }
            // A left release cannot end a right-button gesture; put it back.
            DragState::AwaitingContextMenu { event, anchor } => {
                self.drag_state = DragState::AwaitingContextMenu { event, anchor };
                EventResponse::Handled
            }
            DragState::SelectingBox { anchor, current } => {
                self.select_within_box(anchor, current);
                let mut events = Vec::new();
                if let Some(sel) = self.maybe_selection_changed_event() {
                    events.push(sel);
                }
                EventResponse::Event(events)
            }
            DragState::ResizingNode { node_id, .. } => {
                self.resolve_hierarchy();
                EventResponse::Event(vec![FlowEvent::NodeResizeEnded { node_id }])
            }
            DragState::MovingNode {
                node_id,
                drag_started,
                selected,
                ..
            } => {
                self.resolve_drag_hierarchy_if_pending();
                if drag_started {
                    EventResponse::Event(vec![FlowEvent::NodeDragEnded { node_id }])
                } else {
                    // Click (threshold not exceeded). Select if not already selected
                    // on mouse-down — skipped when select_nodes_on_drag=true with threshold=0
                    // since that case selects immediately.
                    if !selected {
                        let selectable = self
                            .internal_node(&node_id)
                            .is_some_and(|n| n.node.selectable);
                        if selectable {
                            self.select_node(&node_id);
                        }
                    }
                    let mut events = vec![FlowEvent::NodeClicked { node_id }];
                    if let Some(sel) = self.maybe_selection_changed_event() {
                        events.push(sel);
                    }
                    EventResponse::Event(events)
                }
            }
            DragState::Panning { .. } | DragState::None => EventResponse::Handled,
        }
    }

    /// Finds a single selected reconnectable edge at the given endpoint.
    ///
    /// Returns `Some` only when exactly one selected edge matches — multiple
    /// matches are ambiguous and return `None` (falls through to new connection).
    fn find_reconnectable_edge_at(
        &self,
        node_id: &str,
        handle_id: Option<&str>,
        handle_type: HandleType,
    ) -> Option<(String, String, Option<String>)> {
        let global_default = self.edges_reconnectable;
        let mut found: Option<(String, String, Option<String>)> = None;

        for edge in &self.edges {
            if !edge.selected {
                continue;
            }

            if !edge.reconnectable.allows(handle_type, global_default) {
                continue;
            }

            let matches = match handle_type {
                HandleType::Source => {
                    edge.source == node_id && edge.source_handle.as_deref() == handle_id
                }
                HandleType::Target => {
                    edge.target == node_id && edge.target_handle.as_deref() == handle_id
                }
            };

            if matches {
                if found.is_some() {
                    // Ambiguous — multiple selected edges at same handle
                    return None;
                }
                let (opposite_node, opposite_handle) = match handle_type {
                    HandleType::Source => (edge.target.clone(), edge.target_handle.clone()),
                    HandleType::Target => (edge.source.clone(), edge.source_handle.clone()),
                };
                found = Some((edge.id.clone(), opposite_node, opposite_handle));
            }
        }

        found
    }
}

/// Result of a hit test operation.
#[derive(Debug, Clone)]
pub(crate) enum MouseHit {
    /// Hit a handle on a node (source or target).
    Handle {
        node_id: String,
        handle_id: Option<String>,
        handle_type: HandleType,
    },
    /// Hit a node body (but not a handle).
    Node { node_id: String },
    /// Hit an edge.
    Edge { edge_id: String },
    /// Hit nothing.
    Nothing,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Edge, Node, Reconnectable};
    use crate::ui::{StepEdge, TextContent};

    fn make_test_state(nodes: Vec<Node<TextContent>>) -> Flow {
        let mut state = Flow::with_graph(nodes, vec![]).unwrap();
        state.ensure_z_order();
        state
    }

    // --- pick ----------------------------------------------------------------

    #[test]
    fn pick_reports_without_changing_anything() {
        let node = Node::new(
            "a",
            Position::new(0.0, 0.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);

        assert_eq!(
            state.pick(Position::new(5.0, 5.0)),
            Pick::Node { node_id: "a" }
        );
        assert_eq!(state.pick(Position::new(900.0, 900.0)), Pick::Nothing);

        // A pick is a question, not an interaction.
        assert!(!state.has_selected_nodes());
        assert!(matches!(state.drag_state, DragState::None));
    }

    // --- selection box -------------------------------------------------------

    fn box_flow() -> Flow {
        make_test_state(vec![
            Node::new(
                "a",
                Position::new(0.0, 0.0),
                (10.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 0.0),
                (10.0, 10.0),
                TextContent::from("b"),
            ),
            Node::new(
                "far",
                Position::new(900.0, 900.0),
                (10.0, 10.0),
                TextContent::from("f"),
            ),
        ])
    }

    #[test]
    fn selection_box_selects_what_it_covers() {
        let mut state = box_flow();
        state.on_right_down(Position::new(-5.0, -5.0));
        state.on_right_drag(Position::new(65.0, 15.0));
        state.on_right_up();

        let mut selected: Vec<String> = state.selected_nodes().map(|n| n.id.clone()).collect();
        selected.sort();
        assert_eq!(selected, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn selection_box_replaces_the_previous_selection() {
        let mut state = box_flow();
        state.select_node("far");

        state.on_right_down(Position::new(-5.0, -5.0));
        state.on_right_drag(Position::new(15.0, 15.0));
        state.on_right_up();

        let selected: Vec<String> = state.selected_nodes().map(|n| n.id.clone()).collect();
        assert_eq!(
            selected,
            vec!["a".to_string()],
            "previous selection should clear"
        );
    }

    #[test]
    fn selection_on_drag_redirects_the_left_pane_gesture() {
        use crate::input::{Modifiers, MouseButton as B, MouseEvent, MouseEventKind as K};

        let press = |column, row| MouseEvent {
            kind: K::Down(B::Left),
            column,
            row,
            modifiers: Modifiers::NONE,
        };

        // Off by default: pressing empty canvas pans.
        let mut state = box_flow();
        state
            .render_context
            .set_canvas_area(ratatui::layout::Rect::new(0, 0, 80, 24));
        state.handle_mouse_event(press(70, 20));
        assert!(matches!(state.drag_state, DragState::Panning { .. }));

        // On: the same press selects instead.
        let mut state = box_flow();
        state
            .render_context
            .set_canvas_area(ratatui::layout::Rect::new(0, 0, 80, 24));
        state.selection_on_drag = true;
        state.handle_mouse_event(press(70, 20));
        assert!(
            matches!(state.drag_state, DragState::SelectingBox { .. }),
            "got {:?}",
            state.drag_state
        );
    }

    #[test]
    fn selection_on_drag_leaves_node_dragging_alone() {
        use crate::input::{Modifiers, MouseButton as B, MouseEvent, MouseEventKind as K};

        let mut state = box_flow();
        state
            .render_context
            .set_canvas_area(ratatui::layout::Rect::new(0, 0, 80, 24));
        state.selection_on_drag = true;
        // Node "a" sits at world (0,0); with an identity viewport that is cell (0,0).
        state.handle_mouse_event(MouseEvent {
            kind: K::Down(B::Left),
            column: 2,
            row: 2,
            modifiers: Modifiers::NONE,
        });
        assert!(
            matches!(state.drag_state, DragState::MovingNode { .. }),
            "pressing a node must still drag it, got {:?}",
            state.drag_state
        );
    }

    // --- resize --------------------------------------------------------------

    fn resizable_flow() -> Flow {
        let node = Node::new(
            "a",
            Position::new(0.0, 0.0),
            (20.0, 10.0),
            TextContent::from("a"),
        )
        .with_resizable(true);
        make_test_state(vec![node])
    }

    #[test]
    fn resizing_grows_from_a_fixed_top_left() {
        let mut state = resizable_flow();
        // Grab the grip at (20, 10) and drag out.
        assert!(state.try_start_resize(Position::new(20.0, 10.0)).is_some());
        state.on_mouse_drag(Position::new(30.0, 25.0), Position::new(30.0, 25.0));

        let node = state.node("a").unwrap();
        assert_eq!((node.width, node.height), (30.0, 25.0));
        assert_eq!(
            node.position,
            Position::new(0.0, 0.0),
            "top-left must not move"
        );
    }

    #[test]
    fn repeated_drag_events_do_not_accumulate() {
        // Size is derived from the initial bounds every time, so replaying the same
        // cursor position must be idempotent rather than compounding.
        let mut state = resizable_flow();
        state.try_start_resize(Position::new(20.0, 10.0));
        for _ in 0..5 {
            state.on_mouse_drag(Position::new(30.0, 25.0), Position::new(30.0, 25.0));
        }
        let node = state.node("a").unwrap();
        assert_eq!((node.width, node.height), (30.0, 25.0));
        assert_eq!(node.position, Position::new(0.0, 0.0));
    }

    #[test]
    fn resizing_clamps_to_the_minimum() {
        let mut state = resizable_flow();
        state.min_node_size = crate::types::Dimensions::new(4.0, 3.0);
        state.try_start_resize(Position::new(20.0, 10.0));
        state.on_mouse_drag(Position::new(-50.0, -50.0), Position::new(-50.0, -50.0));

        let node = state.node("a").unwrap();
        assert_eq!((node.width, node.height), (4.0, 3.0));
    }

    #[test]
    fn only_the_bottom_right_grip_resizes() {
        let mut state = resizable_flow();
        // Other corners are ordinary node body.
        for corner in [
            Position::new(0.0, 0.0),
            Position::new(20.0, 0.0),
            Position::new(0.0, 10.0),
        ] {
            assert!(
                state.try_start_resize(corner).is_none(),
                "only the bottom-right corner should grab, {corner:?} did"
            );
        }
        assert!(state.try_start_resize(Position::new(20.0, 10.0)).is_some());
    }

    #[test]
    fn resizing_does_not_require_selection() {
        let node = Node::new(
            "a",
            Position::new(0.0, 0.0),
            (20.0, 10.0),
            TextContent::from("a"),
        )
        .with_resizable(true);
        let mut state = make_test_state(vec![node]);
        assert!(!state.has_selected_nodes());
        assert!(
            state.try_start_resize(Position::new(20.0, 10.0)).is_some(),
            "the grip is visible on unselected nodes, so it must work there"
        );
    }

    #[test]
    fn non_resizable_nodes_have_no_grip() {
        let node = Node::new(
            "b",
            Position::new(0.0, 0.0),
            (20.0, 10.0),
            TextContent::from("b"),
        )
        .with_selected(true);
        let mut state = make_test_state(vec![node]);
        assert!(state.try_start_resize(Position::new(20.0, 10.0)).is_none());
    }

    // --- context menus -------------------------------------------------------

    #[test]
    fn context_menu_reports_what_was_under_the_cursor() {
        let node = Node::new(
            "a",
            Position::new(0.0, 0.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);

        assert_eq!(
            {
                state.on_right_down(Position::new(5.0, 5.0));
                state.on_right_up().into_events().collect::<Vec<_>>()
            },
            vec![FlowEvent::NodeContextMenu {
                node_id: "a".to_string()
            }]
        );

        assert_eq!(
            {
                state.on_right_down(Position::new(500.0, 500.0));
                state.on_right_up().into_events().collect::<Vec<_>>()
            },
            vec![FlowEvent::PaneContextMenu { x: 500.0, y: 500.0 }]
        );
    }

    #[test]
    fn context_menu_leaves_selection_alone() {
        // Opening a menu on one node of a multi-selection must not collapse it.
        let a = Node::new(
            "a",
            Position::new(0.0, 0.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let b = Node::new(
            "b",
            Position::new(100.0, 0.0),
            (20.0, 10.0),
            TextContent::from("b"),
        );
        let mut state = make_test_state(vec![a, b]);
        state.select_node("a");
        state.toggle_node_selection("b");

        let before: Vec<String> = state.selected_nodes().map(|n| n.id.clone()).collect();
        state.on_right_down(Position::new(105.0, 5.0));
        state.on_right_up();
        let after: Vec<String> = state.selected_nodes().map(|n| n.id.clone()).collect();

        assert_eq!(before, after, "context menu must not change selection");
    }

    #[test]
    fn test_hit_test_node() {
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let state = make_test_state(vec![node]);

        // Hit inside node
        let hit = state.hit_test(Position::new(15.0, 15.0));
        assert!(matches!(hit, MouseHit::Node { node_id } if node_id == "a"));

        // Miss outside node
        let hit = state.hit_test(Position::new(100.0, 100.0));
        assert!(matches!(hit, MouseHit::Nothing));
    }

    #[test]
    fn test_hit_test_source_handle() {
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let state = make_test_state(vec![node]);

        // Default source handle is on right edge at (30, 15)
        let hit = state.hit_test(Position::new(30.0, 15.0));
        assert!(
            matches!(hit, MouseHit::Handle { node_id, handle_type, .. } if node_id == "a" && handle_type == HandleType::Source)
        );
    }

    #[test]
    fn test_hit_test_target_handle() {
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let state = make_test_state(vec![node]);

        // Default target handle is on left edge at (10, 15)
        let hit = state.hit_test(Position::new(10.0, 15.0));
        assert!(
            matches!(hit, MouseHit::Handle { node_id, handle_type, .. } if node_id == "a" && handle_type == HandleType::Target)
        );
    }

    #[test]
    fn test_find_connectable_handle_excludes_self() {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
        ];
        let state = make_test_state(nodes);

        // Position near node "a"'s target handle, but searching from "a" - should find nothing
        let result = state.find_connectable_handle_by_position(
            Position::new(10.0, 15.0),
            "a",
            HandleType::Source,
        );
        assert!(result.is_none());

        // Position near node "b"'s target handle, searching from "a" - should find it
        let result = state.find_connectable_handle_by_position(
            Position::new(50.0, 15.0),
            "a",
            HandleType::Source,
        );
        assert!(result.is_some());
        let handle = result.unwrap();
        assert_eq!(handle.node_id, "b");
    }

    #[test]
    fn test_strict_mode_rejects_source_to_source() {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
        ];
        let state = make_test_state(nodes);

        // In Strict mode, searching for source→source should find nothing
        // Node b's source handle is at (50 + 20, 15) = (70, 15)
        let result = state.find_connectable_handle_by_position(
            Position::new(70.0, 15.0),
            "a",
            HandleType::Source,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_node_mouse_down_emits_selection_changed_immediate() {
        // With threshold == 0, selection happens on mouse-down
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);
        state.node_drag_threshold = 0.0;

        state.snapshot_selection();
        state.ensure_z_order();
        let response = state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );

        let events: Vec<_> = response.into_events().collect();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            FlowEvent::SelectionChanged { node_ids, .. } if *node_ids == vec!["a".to_string()]
        ));
    }

    #[test]
    fn test_node_click_emits_selection_changed_deferred() {
        // With threshold > 0, selection is deferred to mouse-up (click)
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);
        // default threshold is 2.0

        state.snapshot_selection();
        state.ensure_z_order();
        let response = state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );
        // No selection on mouse-down (deferred)
        let events: Vec<_> = response.into_events().collect();
        assert!(events.is_empty());
        assert!(!state.node("a").unwrap().selected);

        // Mouse-up without exceeding threshold → click → deferred selection
        state.snapshot_selection();
        let response = state.on_mouse_up(Position::new(20.0, 15.0));
        let events: Vec<_> = response.into_events().collect();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, FlowEvent::NodeClicked { .. }))
        );
        assert!(events.iter().any(
            |e| matches!(e, FlowEvent::SelectionChanged { node_ids, .. } if *node_ids == vec!["a".to_string()])
        ));
        assert!(state.node("a").unwrap().selected);
    }

    #[test]
    fn test_node_mouse_down_suppresses_selection_changed_when_unchanged() {
        // With threshold == 0, selection happens on mouse-down.
        // If already selected, SelectionChanged should be suppressed.
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);
        state.node_drag_threshold = 0.0;
        state.select_node("a");

        // Snapshot with "a" already selected, then click same node
        state.snapshot_selection();
        state.ensure_z_order();
        let response = state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );

        // No SelectionChanged — "a" was already selected
        let events: Vec<_> = response.into_events().collect();
        assert!(events.is_empty());
    }

    #[test]
    fn test_loose_mode_allows_source_to_source() {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
        ];
        let mut state = make_test_state(nodes);
        state.connection_mode = ConnectionMode::Loose;

        // In Loose mode, source→source should find node b's source handle
        // Node b's source handle is at (50 + 20, 15) = (70, 15)
        let result = state.find_connectable_handle_by_position(
            Position::new(70.0, 15.0),
            "a",
            HandleType::Source,
        );
        assert!(result.is_some());
        let handle = result.unwrap();
        assert_eq!(handle.node_id, "b");
        assert_eq!(handle.handle_type, HandleType::Source);
    }

    #[test]
    fn test_hit_test_z_index_ordering() {
        // Two overlapping nodes at same position, different z-index
        let nodes = vec![
            Node::new(
                "back",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("back"),
            )
            .with_z_index(0),
            Node::new(
                "front",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("front"),
            )
            .with_z_index(1),
        ];
        let state = make_test_state(nodes);

        // Hit in overlap area should return the higher z-index node
        let hit = state.hit_test(Position::new(20.0, 15.0));
        assert!(matches!(hit, MouseHit::Node { node_id } if node_id == "front"));
    }

    #[test]
    fn test_hit_test_z_index_body_occludes_handle() {
        // Node A (z=0) has a handle at a position overlapping node B's body (z=1).
        // Node A at x=10..30, source handle at (30, 15).
        // Node B overlaps that handle position: x=25..45.
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            )
            .with_z_index(0),
            Node::new(
                "b",
                Position::new(25.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            )
            .with_z_index(1),
        ];
        let state = make_test_state(nodes);

        // Position (30, 15) is node A's source handle AND inside node B's body.
        // Node B is in front (z=1), so its body should win.
        let hit = state.hit_test(Position::new(30.0, 15.0));
        assert!(matches!(hit, MouseHit::Node { node_id } if node_id == "b"));
    }

    #[test]
    fn test_hit_test_z_index_selected_elevation() {
        // Two overlapping nodes, back one is selected (elevated).
        let nodes = vec![
            Node::new(
                "back",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("back"),
            )
            .with_z_index(0)
            .with_selected(true),
            Node::new(
                "front",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("front"),
            )
            .with_z_index(0),
        ];
        let state = make_test_state(nodes);

        // "back" is selected so it gets +1000 z elevation, making it in front
        let hit = state.hit_test(Position::new(20.0, 15.0));
        assert!(matches!(hit, MouseHit::Node { node_id } if node_id == "back"));
    }

    #[test]
    fn test_hit_test_child_above_selected_parent() {
        // Parent node selected (+1000 elevation), child should still be above it.
        let parent = Node::new(
            "parent",
            Position::new(10.0, 10.0),
            (40.0, 30.0),
            TextContent::from("parent"),
        )
        .with_selected(true);
        let child = Node::new(
            "child",
            Position::new(5.0, 5.0),
            (15.0, 10.0),
            TextContent::from("child"),
        )
        .with_parent("parent");
        let nodes = vec![parent, child];
        let state = make_test_state(nodes);

        // Child's absolute position: parent(10,10) + child(5,5) = (15,15)
        // Click inside child body: (20, 18) is within child bounds (15..30, 15..25)
        let hit = state.hit_test(Position::new(20.0, 18.0));
        assert!(matches!(hit, MouseHit::Node { node_id } if node_id == "child"));
    }

    #[test]
    fn test_hit_test_edge_through_parent() {
        // Edge between two children should be clickable through the parent body.
        //
        // Layout:
        //   parent (0,0) 60x30
        //     child_a (5,5) 10x10   -> absolute (5,5)..(15,15)
        //     child_b (40,5) 10x10  -> absolute (40,5)..(50,15)
        //   edge: child_a -> child_b (crosses parent body around x=25)
        let parent = Node::new(
            "parent",
            Position::new(0.0, 0.0),
            (60.0, 30.0),
            TextContent::from("parent"),
        );
        let child_a = Node::new(
            "child_a",
            Position::new(5.0, 5.0),
            (10.0, 10.0),
            TextContent::from("a"),
        )
        .with_parent("parent");
        let child_b = Node::new(
            "child_b",
            Position::new(40.0, 5.0),
            (10.0, 10.0),
            TextContent::from("b"),
        )
        .with_parent("parent");
        let edge: Edge<StepEdge> = Edge::new("e1", "child_a", "child_b");

        let mut state = Flow::with_graph(vec![parent, child_a, child_b], vec![edge]).unwrap();
        state.ensure_z_order();

        // Children have effective_z = 1 (parent=0, child=parent+1=1).
        // Edge implicit z = max(child_a_z, child_b_z) = 1.
        // Parent effective_z = 0, so edge_z (1) > parent_z (0).
        //
        // The edge path goes from child_a's source handle (15, 10) to child_b's
        // target handle (40, 10). The midpoint ~(27.5, 10) is inside the parent body
        // but not inside either child. Hit testing should return the edge, not the parent.
        let hit = state.hit_test(Position::new(27.5, 10.0));
        assert!(
            matches!(hit, MouseHit::Edge { edge_id } if edge_id == "e1"),
            "Edge between children should be clickable through parent body, got: {:?}",
            state.hit_test(Position::new(27.5, 10.0))
        );
    }

    // ========== Reconnection Tests ==========

    /// Creates a two-node state with an edge from "a" to "b".
    /// Node "a" at (10,10) 20x10, Node "b" at (50,10) 20x10.
    /// Edge e1: a→b. Source handle a at (30, 15), target handle b at (50, 15).
    fn make_reconnect_state() -> Flow {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
            Node::new(
                "c",
                Position::new(90.0, 10.0),
                (20.0, 10.0),
                TextContent::from("c"),
            ),
        ];
        let edges: Vec<Edge<StepEdge>> = vec![Edge::new("e1", "a", "b")];
        let mut state = Flow::with_graph(nodes, edges).unwrap();
        state.ensure_z_order();
        state
    }

    #[test]
    fn test_reconnect_detection_selected_edge() {
        let mut state = make_reconnect_state();
        // Select edge e1, then click on its source handle (node a's source at ~(30, 15))
        state.select_edge("e1");
        state.snapshot_selection();
        state.ensure_z_order();

        let response = state.on_mouse_down(
            Position::new(30.0, 15.0),
            Position::new(30.0, 15.0),
            true,
            false,
        );

        let events: Vec<_> = response.into_events().collect();
        assert!(
            events.iter().any(|e| matches!(
                e,
                FlowEvent::ReconnectionStarted { edge_id, handle_type }
                if edge_id == "e1" && handle_type == &HandleType::Source
            )),
            "Expected ReconnectionStarted, got: {:?}",
            events
        );
        assert!(matches!(
            state.drag_state,
            DragState::ReconnectingEdge { .. }
        ));
    }

    #[test]
    fn test_no_reconnect_without_selection() {
        let mut state = make_reconnect_state();
        // Edge NOT selected, click on source handle → ConnectionStarted (not reconnect)
        state.snapshot_selection();
        state.ensure_z_order();

        let response = state.on_mouse_down(
            Position::new(30.0, 15.0),
            Position::new(30.0, 15.0),
            true,
            false,
        );

        let events: Vec<_> = response.into_events().collect();
        assert!(
            events.iter().any(|e| matches!(
                e,
                FlowEvent::ConnectionStarted { node_id, .. } if node_id == "a"
            )),
            "Expected ConnectionStarted, got: {:?}",
            events
        );
        assert!(matches!(state.drag_state, DragState::CreatingConnection));
    }

    #[test]
    fn test_reconnect_per_edge_flag_target_only() {
        // Edge with Reconnectable::Target — clicking source handle should NOT reconnect
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
        ];
        let edges: Vec<Edge<StepEdge>> =
            vec![Edge::new("e1", "a", "b").with_reconnectable(Reconnectable::Target)];
        let mut state = Flow::with_graph(nodes, edges).unwrap();
        state.ensure_z_order();
        state.select_edge("e1");
        state.snapshot_selection();
        state.ensure_z_order();

        // Click source handle — should NOT reconnect (only Target allowed)
        let response = state.on_mouse_down(
            Position::new(30.0, 15.0),
            Position::new(30.0, 15.0),
            true,
            false,
        );

        let events: Vec<_> = response.into_events().collect();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, FlowEvent::ConnectionStarted { .. })),
            "Expected ConnectionStarted (not reconnect), got: {:?}",
            events
        );
    }

    #[test]
    fn test_reconnect_global_default_with_inherit() {
        let mut state = make_reconnect_state();
        // edges_reconnectable = true (default) + Inherit → reconnection works
        state.select_edge("e1");
        state.snapshot_selection();
        state.ensure_z_order();

        let response = state.on_mouse_down(
            Position::new(30.0, 15.0),
            Position::new(30.0, 15.0),
            true,
            false,
        );
        let events: Vec<_> = response.into_events().collect();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, FlowEvent::ReconnectionStarted { .. }))
        );

        // Now set global to false
        state.drag_state = DragState::None;
        let mut state2 = make_reconnect_state();
        state2.edges_reconnectable = false;
        state2.select_edge("e1");
        state2.snapshot_selection();
        state2.ensure_z_order();

        let response = state2.on_mouse_down(
            Position::new(30.0, 15.0),
            Position::new(30.0, 15.0),
            true,
            false,
        );
        let events: Vec<_> = response.into_events().collect();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, FlowEvent::ConnectionStarted { .. })),
            "Expected ConnectionStarted with global false + Inherit, got: {:?}",
            events
        );
    }

    #[test]
    fn test_reconnect_explicit_none_overrides_global() {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
        ];
        let edges: Vec<Edge<StepEdge>> =
            vec![Edge::new("e1", "a", "b").with_reconnectable(Reconnectable::None)];
        let mut state = Flow::with_graph(nodes, edges).unwrap();
        state.edges_reconnectable = true; // Global true, but edge says None
        state.ensure_z_order();
        state.select_edge("e1");
        state.snapshot_selection();
        state.ensure_z_order();

        let response = state.on_mouse_down(
            Position::new(30.0, 15.0),
            Position::new(30.0, 15.0),
            true,
            false,
        );

        let events: Vec<_> = response.into_events().collect();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, FlowEvent::ConnectionStarted { .. })),
            "Expected ConnectionStarted (None overrides global true), got: {:?}",
            events
        );
    }

    #[test]
    fn test_reconnect_full_cycle() {
        let mut state = make_reconnect_state();
        // Edge e1: a→b. Reconnect the target end (b's target handle at 50, 15)
        // to node c's target handle (at 90, 15). Fixed end is source (node a).
        state.select_edge("e1");
        state.snapshot_selection();
        state.ensure_z_order();

        // Mouse down on target handle of edge e1 (b's target at 50, 15)
        let response = state.on_mouse_down(
            Position::new(50.0, 15.0),
            Position::new(50.0, 15.0),
            true,
            false,
        );
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::ReconnectionStarted { .. }))
        );

        // The fixed end is Source (node a). find_connectable_handle searches for
        // Target handles (opposite of Source). Node c's target handle is at (90, 15).
        let _ = state.on_mouse_drag(Position::new(90.0, 15.0), Position::new(90.0, 15.0));

        // Mouse up with valid target
        let response = state.on_mouse_up(Position::new(90.0, 15.0));
        let events: Vec<_> = response.into_events().collect();
        assert!(
            events.iter().any(|e| matches!(
                e,
                FlowEvent::ReconnectionCompleted { edge_id, old_connection, new_connection }
                if edge_id == "e1"
                    && old_connection.source == "a"
                    && old_connection.target == "b"
                    && new_connection.source == "a"
                    && new_connection.target == "c"
            )),
            "Expected ReconnectionCompleted with correct connections, got: {:?}",
            events
        );
    }

    #[test]
    fn test_reconnect_cancelled_no_target() {
        let mut state = make_reconnect_state();
        state.select_edge("e1");
        state.snapshot_selection();
        state.ensure_z_order();

        // Mouse down on source handle
        state.on_mouse_down(
            Position::new(30.0, 15.0),
            Position::new(30.0, 15.0),
            true,
            false,
        );

        // Drag to empty space (far from any handle)
        let _ = state.on_mouse_drag(Position::new(200.0, 200.0), Position::new(200.0, 200.0));

        // Mouse up with no valid target
        let response = state.on_mouse_up(Position::new(200.0, 200.0));
        let events: Vec<_> = response.into_events().collect();
        assert!(
            events.iter().any(|e| matches!(
                e,
                FlowEvent::ReconnectionCancelled { edge_id } if edge_id == "e1"
            )),
            "Expected ReconnectionCancelled, got: {:?}",
            events
        );
    }

    #[test]
    fn test_reconnect_cancel_via_esc() {
        let mut state = make_reconnect_state();
        state.select_edge("e1");
        state.snapshot_selection();
        state.ensure_z_order();

        // Mouse down on source handle
        state.on_mouse_down(
            Position::new(30.0, 15.0),
            Position::new(30.0, 15.0),
            true,
            false,
        );
        assert!(matches!(
            state.drag_state,
            DragState::ReconnectingEdge { .. }
        ));

        // Cancel via action
        let response = state.apply(crate::FlowAction::CancelConnection);
        let events: Vec<_> = response.into_events().collect();
        assert!(
            events.iter().any(|e| matches!(
                e,
                FlowEvent::ReconnectionCancelled { edge_id } if edge_id == "e1"
            )),
            "Expected ReconnectionCancelled from ESC, got: {:?}",
            events
        );
        assert!(matches!(state.drag_state, DragState::None));
    }

    // ========== DragState Transition Tests ==========

    #[test]
    fn test_panning_flow() {
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);

        // Click on empty space → Panning
        state.snapshot_selection();
        let response = state.on_mouse_down(
            Position::new(100.0, 100.0),
            Position::new(100.0, 100.0),
            true,
            false,
        );
        assert!(matches!(state.drag_state, DragState::Panning { .. }));
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::PaneClicked { .. }))
        );

        // Drag → ViewportChanged
        let response =
            state.on_mouse_drag(Position::new(110.0, 105.0), Position::new(110.0, 105.0));
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::ViewportChanged { .. }))
        );

        // Release → None
        let response = state.on_mouse_up(Position::new(110.0, 105.0));
        assert_eq!(response, EventResponse::Handled);
        assert!(matches!(state.drag_state, DragState::None));
    }

    #[test]
    fn test_node_drag_threshold_then_move() {
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);

        // Click on node body → MovingNode(drag_started=false)
        state.snapshot_selection();
        state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );
        assert!(matches!(
            state.drag_state,
            DragState::MovingNode {
                drag_started: false,
                ..
            }
        ));

        // Small drag within threshold → no NodeDragStarted
        let response = state.on_mouse_drag(Position::new(20.5, 15.0), Position::new(20.5, 15.0));
        assert_eq!(response, EventResponse::Handled);
        assert!(matches!(
            state.drag_state,
            DragState::MovingNode {
                drag_started: false,
                ..
            }
        ));

        // Drag past threshold → NodeDragStarted
        let response = state.on_mouse_drag(Position::new(25.0, 15.0), Position::new(25.0, 15.0));
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::NodeDragStarted { .. }))
        );
        assert!(matches!(
            state.drag_state,
            DragState::MovingNode {
                drag_started: true,
                ..
            }
        ));

        // Subsequent drag → NodeDragged (not NodeDragStarted again)
        let response = state.on_mouse_drag(Position::new(30.0, 15.0), Position::new(30.0, 15.0));
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::NodeDragged { .. }))
        );

        // Release → NodeDragEnded
        let response = state.on_mouse_up(Position::new(30.0, 15.0));
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::NodeDragEnded { .. }))
        );
        assert!(matches!(state.drag_state, DragState::None));
    }

    #[test]
    fn test_node_click_without_drag() {
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);

        // Click on node body then release without exceeding threshold → NodeClicked
        state.snapshot_selection();
        state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );
        let response = state.on_mouse_up(Position::new(20.0, 15.0));
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::NodeClicked { ref node_id } if node_id == "a"))
        );
        assert!(matches!(state.drag_state, DragState::None));
    }

    #[test]
    fn test_non_draggable_node_click() {
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        )
        .with_draggable(false);
        let mut state = make_test_state(vec![node]);

        // Click non-draggable → AwaitingNodeClick
        state.snapshot_selection();
        state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );
        assert!(matches!(
            state.drag_state,
            DragState::AwaitingNodeClick { .. }
        ));

        // Release → NodeClicked
        let response = state.on_mouse_up(Position::new(20.0, 15.0));
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::NodeClicked { ref node_id } if node_id == "a"))
        );
        assert!(matches!(state.drag_state, DragState::None));
    }

    #[test]
    fn test_reconnect_ambiguous_multi_select() {
        // Two selected edges at same source handle → falls through to ConnectionStarted
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
            Node::new(
                "c",
                Position::new(90.0, 10.0),
                (20.0, 10.0),
                TextContent::from("c"),
            ),
        ];
        let edges: Vec<Edge<StepEdge>> = vec![Edge::new("e1", "a", "b"), Edge::new("e2", "a", "c")];
        let mut state = Flow::with_graph(nodes, edges).unwrap();
        state.ensure_z_order();

        // Select both edges — they share node "a"'s default source handle
        state.toggle_edge_selection("e1");
        state.toggle_edge_selection("e2");
        state.snapshot_selection();
        state.ensure_z_order();

        // Click on node a's source handle
        let response = state.on_mouse_down(
            Position::new(30.0, 15.0),
            Position::new(30.0, 15.0),
            true,
            false,
        );

        let events: Vec<_> = response.into_events().collect();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, FlowEvent::ConnectionStarted { .. })),
            "Ambiguous multi-select should fall through to ConnectionStarted, got: {:?}",
            events
        );
    }

    #[test]
    fn test_select_nodes_on_drag_false_selects_on_click_only() {
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);
        state.select_nodes_on_drag = false;
        state.node_drag_threshold = 0.0;

        // Mouse-down: no selection (select_nodes_on_drag=false)
        state.snapshot_selection();
        state.ensure_z_order();
        state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );
        assert!(!state.node("a").unwrap().selected);

        // Drag past threshold → node moves but stays unselected
        let response = state.on_mouse_drag(Position::new(25.0, 15.0), Position::new(25.0, 15.0));
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::NodeDragStarted { .. }))
        );
        assert!(!state.node("a").unwrap().selected);

        // Release → NodeDragEnded, still unselected (was a drag, not click)
        let response = state.on_mouse_up(Position::new(25.0, 15.0));
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::NodeDragEnded { .. }))
        );
        assert!(!state.node("a").unwrap().selected);
    }

    #[test]
    fn test_select_nodes_on_drag_false_click_selects() {
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);
        state.select_nodes_on_drag = false;

        // Mouse-down + up without drag → click → selects
        state.snapshot_selection();
        state.ensure_z_order();
        state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );
        assert!(!state.node("a").unwrap().selected);

        state.snapshot_selection();
        let response = state.on_mouse_up(Position::new(20.0, 15.0));
        let events: Vec<_> = response.into_events().collect();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, FlowEvent::NodeClicked { .. }))
        );
        assert!(state.node("a").unwrap().selected);
    }

    #[test]
    fn test_deferred_selection_on_drag_threshold_exceeded() {
        // With threshold > 0, selection deferred to when threshold exceeded
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        );
        let mut state = make_test_state(vec![node]);
        // default: select_nodes_on_drag=true, node_drag_threshold=2.0

        state.snapshot_selection();
        state.ensure_z_order();
        state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );
        // Not selected yet
        assert!(!state.node("a").unwrap().selected);

        // Drag past threshold → selects
        state.snapshot_selection();
        let response = state.on_mouse_drag(Position::new(25.0, 15.0), Position::new(25.0, 15.0));
        assert!(state.node("a").unwrap().selected);
        let events: Vec<_> = response.into_events().collect();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, FlowEvent::NodeDragStarted { .. }))
        );
        assert!(events.iter().any(
            |e| matches!(e, FlowEvent::SelectionChanged { node_ids, .. } if *node_ids == vec!["a".to_string()])
        ));
    }

    #[test]
    fn test_non_selectable_draggable_node_can_drag() {
        // Non-selectable but draggable nodes should be draggable
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        )
        .with_selectable(false);
        let mut state = make_test_state(vec![node]);
        state.node_drag_threshold = 0.0;

        // Hit test should find non-selectable draggable node
        state.ensure_z_order();
        let hit = state.hit_test(Position::new(20.0, 15.0));
        assert!(matches!(hit, MouseHit::Node { ref node_id } if node_id == "a"));

        // Mouse-down starts MovingNode
        state.snapshot_selection();
        state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );
        assert!(matches!(state.drag_state, DragState::MovingNode { .. }));
        // Not selected (non-selectable)
        assert!(!state.node("a").unwrap().selected);
    }

    #[test]
    fn test_non_selectable_draggable_clears_others() {
        // Dragging a non-selectable node should deselect others
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            )
            .with_selectable(false),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
        ];
        let mut state = make_test_state(nodes);
        state.node_drag_threshold = 0.0;
        state.select_node("b");
        assert!(state.node("b").unwrap().selected);

        // Mouse-down on non-selectable "a" → clears "b"'s selection
        state.snapshot_selection();
        state.ensure_z_order();
        state.on_mouse_down(
            Position::new(20.0, 15.0),
            Position::new(20.0, 15.0),
            true,
            false,
        );
        assert!(!state.node("a").unwrap().selected);
        assert!(!state.node("b").unwrap().selected);
    }

    #[test]
    fn test_non_selectable_non_draggable_transparent() {
        // Nodes that are neither selectable nor draggable should be click-through
        let node = Node::new(
            "a",
            Position::new(10.0, 10.0),
            (20.0, 10.0),
            TextContent::from("a"),
        )
        .with_selectable(false)
        .with_draggable(false);
        let mut state = make_test_state(vec![node]);
        state.ensure_z_order();

        let hit = state.hit_test(Position::new(20.0, 15.0));
        assert!(matches!(hit, MouseHit::Nothing));
    }

    #[test]
    fn test_deselect_on_drag_false_preserves_selection() {
        // With deselect_on_drag=false, dragging an unselected node
        // should NOT clear the existing selection
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
        ];
        let mut state = make_test_state(nodes);
        state.node_drag_threshold = 0.0;
        state.select_nodes_on_drag = false;
        state.deselect_on_drag = false;
        state.select_node("a");
        assert!(state.node("a").unwrap().selected);

        // Mouse-down on "b" → "a" stays selected
        state.snapshot_selection();
        state.ensure_z_order();
        state.on_mouse_down(
            Position::new(60.0, 15.0),
            Position::new(60.0, 15.0),
            true,
            false,
        );
        assert!(state.node("a").unwrap().selected);
        assert!(!state.node("b").unwrap().selected);

        // Drag "b" past threshold → "a" still selected
        state.snapshot_selection();
        let response = state.on_mouse_drag(Position::new(65.0, 15.0), Position::new(65.0, 15.0));
        assert!(
            response
                .into_events()
                .any(|e| matches!(e, FlowEvent::NodeDragStarted { .. }))
        );
        assert!(state.node("a").unwrap().selected);
        assert!(!state.node("b").unwrap().selected);
    }

    #[test]
    fn test_deselect_on_drag_true_clears_selection() {
        // With deselect_on_drag=true (default), dragging an unselected node
        // clears the existing selection
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
        ];
        let mut state = make_test_state(nodes);
        state.node_drag_threshold = 0.0;
        state.select_nodes_on_drag = false;
        // deselect_on_drag defaults to true
        state.select_node("a");

        // Mouse-down on "b" → "a" gets deselected
        state.snapshot_selection();
        state.ensure_z_order();
        state.on_mouse_down(
            Position::new(60.0, 15.0),
            Position::new(60.0, 15.0),
            true,
            false,
        );
        assert!(!state.node("a").unwrap().selected);
    }

    #[test]
    fn test_railway_pattern_no_selection_change_on_drag() {
        // Exact Railway-style config: select_nodes_on_drag=false, deselect_on_drag=false,
        // default threshold (2.0). Select A, then drag B past threshold.
        // No selection should change at any point during the drag.
        let nodes = vec![
            Node::new(
                "a",
                Position::new(10.0, 10.0),
                (20.0, 10.0),
                TextContent::from("a"),
            ),
            Node::new(
                "b",
                Position::new(50.0, 10.0),
                (20.0, 10.0),
                TextContent::from("b"),
            ),
        ];
        let mut state = make_test_state(nodes);
        // default threshold = 2.0
        state.select_nodes_on_drag = false;
        state.deselect_on_drag = false;
        state.select_node("a");
        assert!(state.node("a").unwrap().selected);
        assert!(!state.node("b").unwrap().selected);

        // Mouse-down on B
        state.snapshot_selection();
        state.ensure_z_order();
        let response = state.on_mouse_down(
            Position::new(60.0, 15.0),
            Position::new(60.0, 15.0),
            true,
            false,
        );
        // No SelectionChanged on mouse-down
        let events: Vec<_> = response.into_events().collect();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, FlowEvent::SelectionChanged { .. })),
            "mouse-down should not emit SelectionChanged, got: {:?}",
            events
        );
        assert!(state.node("a").unwrap().selected);
        assert!(!state.node("b").unwrap().selected);

        // Drag B past threshold
        state.snapshot_selection();
        let response = state.on_mouse_drag(Position::new(65.0, 15.0), Position::new(65.0, 15.0));
        let events: Vec<_> = response.into_events().collect();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, FlowEvent::SelectionChanged { .. })),
            "drag should not emit SelectionChanged, got: {:?}",
            events
        );
        assert!(
            state.node("a").unwrap().selected,
            "A should stay selected during drag"
        );
        assert!(
            !state.node("b").unwrap().selected,
            "B should not be selected during drag"
        );

        // Continue dragging
        state.snapshot_selection();
        let response = state.on_mouse_drag(Position::new(70.0, 15.0), Position::new(70.0, 15.0));
        let events: Vec<_> = response.into_events().collect();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, FlowEvent::SelectionChanged { .. })),
        );

        // Mouse-up (drag end)
        state.snapshot_selection();
        let response = state.on_mouse_up(Position::new(70.0, 15.0));
        let events: Vec<_> = response.into_events().collect();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, FlowEvent::SelectionChanged { .. })),
            "mouse-up after drag should not emit SelectionChanged, got: {:?}",
            events
        );
        assert!(
            state.node("a").unwrap().selected,
            "A should stay selected after drag"
        );
        assert!(
            !state.node("b").unwrap().selected,
            "B should not be selected after drag"
        );
    }
}
