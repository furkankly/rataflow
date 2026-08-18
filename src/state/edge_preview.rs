//! Edge preview state and connection resolution.
//!
//! Centralizes all edge preview logic — state, handle resolution, and validation —
//! used by both mouse drag handlers and the keyboard navigation API.

use super::Flow;
use crate::content::{EdgeContent, NodeContent};
use crate::types::{
    ComputedHandle, Connection, ConnectionMode, HandleBounds, HandleType, Position,
};

/// Returns compatible target handles from the given bounds.
///
/// Filters by connection mode (strict searches opposite type only, loose searches
/// both) and connectability flags (`connectable` and `connectable_end`).
fn compatible_targets<'a>(
    bounds: &'a HandleBounds,
    from_handle_type: HandleType,
    connection_mode: ConnectionMode,
) -> impl Iterator<Item = &'a ComputedHandle> + 'a {
    let opposite = from_handle_type.opposite();
    let loose = matches!(connection_mode, ConnectionMode::Loose);

    bounds
        .source
        .iter()
        .filter(move |_| loose || opposite == HandleType::Source)
        .chain(
            bounds
                .target
                .iter()
                .filter(move |_| loose || opposite == HandleType::Target),
        )
        .filter(|h| h.connectable && h.connectable_end && !h.hidden)
}

/// State for the edge preview.
///
/// Returned by [`Flow::edge_preview()`] when an edge preview is active.
/// Tracks the source handle, target handle, and connection validation.
#[derive(Debug, Clone)]
pub struct EdgePreview {
    /// The source node ID.
    pub from_node_id: String,
    /// The source handle ID, if any.
    pub from_handle_id: Option<String>,
    /// The source handle type.
    pub from_handle_type: HandleType,
    /// Target world position for rendering. `None` means connection mode is
    /// active but nothing is rendered yet.
    pub(crate) to_world: Option<Position>,
    /// The target node ID, if a target has been set.
    pub to_node_id: Option<String>,
    /// The target handle ID, if a target handle has been resolved.
    pub to_handle_id: Option<String>,
    /// Whether the current connection is valid (`Some(true)`), invalid
    /// (`Some(false)`), or has no target to validate (`None`).
    pub is_valid: Option<bool>,
}

impl EdgePreview {
    /// Builds a normalized [`Connection`] from this preview state.
    ///
    /// Normalizes direction: if the preview started from a target handle,
    /// source and target are swapped. Returns `None` if no target node is set.
    fn to_connection(&self) -> Option<Connection> {
        let to_node_id = self.to_node_id.as_ref()?;
        Some(if self.from_handle_type == HandleType::Target {
            Connection::new(
                to_node_id,
                self.to_handle_id.clone(),
                &self.from_node_id,
                self.from_handle_id.clone(),
            )
        } else {
            Connection::new(
                &self.from_node_id,
                self.from_handle_id.clone(),
                to_node_id,
                self.to_handle_id.clone(),
            )
        })
    }
}

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    // ========== Public API ==========

    /// Starts an edge preview from a source handle.
    ///
    /// Validates the source handle and enters connection mode without rendering
    /// a preview line. Call [`preview_to_handle`](Self::preview_to_handle) or
    /// [`preview_to_node`](Self::preview_to_node) to set a target,
    /// [`complete_edge_preview`](Self::complete_edge_preview) to finalize
    /// (then [`add_edge_from_connection`](Self::add_edge_from_connection) to
    /// add the edge), or [`clear_edge_preview`](Self::clear_edge_preview) to cancel.
    ///
    /// Returns `false` if the node or handle doesn't exist, or the handle
    /// isn't connectable as a start point.
    pub fn start_edge_preview(
        &mut self,
        from_node_id: &str,
        from_handle_id: Option<&str>,
        from_handle_type: HandleType,
    ) -> bool {
        let Some(node) = self.internal_node(from_node_id) else {
            return false;
        };
        let Some(handle) = node.handle_bounds.find(from_handle_id, from_handle_type) else {
            return false;
        };
        if !node.node.connectable
            || !handle.connectable
            || !handle.connectable_start
            || handle.hidden
        {
            return false;
        }

        self.edge_preview = Some(EdgePreview {
            from_node_id: from_node_id.to_string(),
            from_handle_id: from_handle_id.map(|s| s.to_string()),
            from_handle_type,
            to_world: None,
            to_node_id: None,
            to_handle_id: None,
            is_valid: None,
        });
        true
    }

    /// Points the edge preview at a specific handle on a target node.
    ///
    /// Validates that the handle exists, is compatible with the source handle
    /// (connection mode, connectability flags), and sets the preview line endpoint
    /// and validation color.
    ///
    /// Pass `handle_id: None` to target the first compatible handle.
    ///
    /// Returns `false` if no edge preview is active, the target is the source node,
    /// the node or handle doesn't exist, or the handle isn't compatible.
    pub fn preview_to_handle(&mut self, to_node_id: &str, to_handle_id: Option<&str>) -> bool {
        let Some(mut ep) = self.edge_preview.take() else {
            return false;
        };

        let node = self.validated_preview_target(to_node_id, &ep.from_node_id);

        let handle = node.and_then(|n| {
            compatible_targets(&n.handle_bounds, ep.from_handle_type, self.connection_mode).find(
                |h| match to_handle_id {
                    Some(id) => h.id.as_deref() == Some(id),
                    None => true,
                },
            )
        });

        let Some(handle) = handle else {
            self.edge_preview = Some(ep);
            return false;
        };

        let handle_pos = handle.absolute_position;
        let to_node = handle.node_id.clone();
        let to_handle = handle.id.clone();

        ep.to_world = Some(handle_pos);
        ep.is_valid = Some(self.is_valid_preview_connection(
            &ep.from_node_id,
            ep.from_handle_id.as_deref(),
            &to_node,
            to_handle.as_deref(),
        ));
        ep.to_node_id = Some(to_node);
        ep.to_handle_id = to_handle;
        self.edge_preview = Some(ep);
        true
    }

    /// Points the edge preview at a target node.
    ///
    /// Finds the best compatible handle on the node (closest to the source handle)
    /// using the current connection mode (strict/loose), validates connectability
    /// and duplicates, and sets the preview line endpoint and validation color.
    ///
    /// Returns `false` if no edge preview is active, the node doesn't exist,
    /// or no compatible handle exists on the node.
    pub fn preview_to_node(&mut self, to_node_id: &str) -> bool {
        let Some(ep) = self.edge_preview.as_ref() else {
            return false;
        };
        let from_node_id = ep.from_node_id.clone();
        let from_handle_id = ep.from_handle_id.clone();
        let from_handle_type = ep.from_handle_type;

        let from_pos = self.internal_node(&from_node_id).and_then(|n| {
            n.handle_bounds
                .find(from_handle_id.as_deref(), from_handle_type)
                .map(|h| h.absolute_position)
        });

        let best =
            self.validated_preview_target(to_node_id, &from_node_id)
                .and_then(|n| {
                    compatible_targets(&n.handle_bounds, from_handle_type, self.connection_mode)
                        .min_by(|a, b| {
                            let da = from_pos
                                .map(|p| a.absolute_position.distance_to(&p))
                                .unwrap_or(0.0);
                            let db = from_pos
                                .map(|p| b.absolute_position.distance_to(&p))
                                .unwrap_or(0.0);
                            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                        })
                });

        let Some(best) = best else {
            return false;
        };
        let handle_id = best.id.clone();

        self.preview_to_handle(to_node_id, handle_id.as_deref())
    }

    /// Cycles the edge preview to the next or previous compatible handle on the
    /// current target node.
    ///
    /// Enumerates all compatible handles on the to-node and steps forward or
    /// backward from the currently selected handle. Wraps around at both ends.
    ///
    /// Returns `false` if no edge preview is active, no to-node is set,
    /// or the to-node has only one compatible handle.
    pub fn cycle_to_handle(&mut self, forward: bool) -> bool {
        let Some(ep) = self.edge_preview.as_ref() else {
            return false;
        };
        let Some(to_node_id) = ep.to_node_id.clone() else {
            return false;
        };
        let from_node_id = ep.from_node_id.clone();
        let from_handle_type = ep.from_handle_type;
        let current_handle_id = ep.to_handle_id.clone();

        let Some(node) = self.validated_preview_target(&to_node_id, &from_node_id) else {
            return false;
        };

        let compatible: Vec<_> =
            compatible_targets(&node.handle_bounds, from_handle_type, self.connection_mode)
                .map(|h| h.id.clone())
                .collect();

        if compatible.len() <= 1 {
            return false;
        }

        let current_idx = compatible
            .iter()
            .position(|id| *id == current_handle_id)
            .unwrap_or(0);

        let next_idx = if forward {
            (current_idx + 1) % compatible.len()
        } else {
            (current_idx + compatible.len() - 1) % compatible.len()
        };

        self.preview_to_handle(&to_node_id, compatible[next_idx].as_deref())
    }

    /// Sets the from-handle without touching the to-side. Re-validates if a to-node is set.
    fn set_preview_source_handle(&mut self, handle_id: Option<&str>) -> bool {
        let Some(mut ep) = self.edge_preview.take() else {
            return false;
        };

        let Some(node) = self.internal_node(&ep.from_node_id) else {
            self.edge_preview = Some(ep);
            return false;
        };

        let Some(handle) = node
            .handle_bounds
            .by_type(ep.from_handle_type)
            .iter()
            .find(|h| {
                h.connectable
                    && h.connectable_start
                    && !h.hidden
                    && match handle_id {
                        Some(id) => h.id.as_deref() == Some(id),
                        None => true,
                    }
            })
        else {
            self.edge_preview = Some(ep);
            return false;
        };

        ep.from_handle_id = handle.id.clone();

        if let Some(ref to_node_id) = ep.to_node_id {
            ep.is_valid = Some(self.is_valid_preview_connection(
                &ep.from_node_id,
                ep.from_handle_id.as_deref(),
                to_node_id,
                ep.to_handle_id.as_deref(),
            ));
        }

        self.edge_preview = Some(ep);
        true
    }

    /// Cycles the from-handle of the edge preview to the next or previous
    /// connectable handle on the from-node.
    ///
    /// Enumerates all connectable-start handles of the same type on the from-node
    /// and steps forward or backward. Wraps around at both ends.
    /// Does not affect the to-side selection.
    ///
    /// Returns `false` if no edge preview is active or the from-node has
    /// only one connectable-start handle.
    pub fn cycle_from_handle(&mut self, forward: bool) -> bool {
        let Some(ep) = self.edge_preview.as_ref() else {
            return false;
        };
        let from_node_id = ep.from_node_id.clone();
        let from_handle_type = ep.from_handle_type;
        let current_handle_id = ep.from_handle_id.clone();

        let Some(node) = self.internal_node(&from_node_id) else {
            return false;
        };

        let compatible: Vec<_> = node
            .handle_bounds
            .by_type(from_handle_type)
            .iter()
            .filter(|h| h.connectable && h.connectable_start && !h.hidden)
            .map(|h| h.id.clone())
            .collect();

        if compatible.len() <= 1 {
            return false;
        }

        let current_idx = compatible
            .iter()
            .position(|id| *id == current_handle_id)
            .unwrap_or(0);

        let next_idx = if forward {
            (current_idx + 1) % compatible.len()
        } else {
            (current_idx + compatible.len() - 1) % compatible.len()
        };

        self.set_preview_source_handle(compatible[next_idx].as_deref())
    }

    /// Completes the edge preview and returns a normalized [`Connection`] if valid.
    ///
    /// The connection is not added to the graph — call
    /// [`add_edge_from_connection`](Self::add_edge_from_connection) to create
    /// the edge. Returns `None` if no preview is active or the current target
    /// is not valid. Clears the edge preview on success.
    pub fn complete_edge_preview(&mut self) -> Option<Connection> {
        let ep = self.edge_preview.as_ref()?;
        if ep.is_valid != Some(true) {
            return None;
        }
        let connection = ep.to_connection()?;
        self.edge_preview = None;
        Some(connection)
    }

    /// Returns the edge preview state, or `None` if no preview is active.
    pub fn edge_preview(&self) -> Option<&EdgePreview> {
        self.edge_preview.as_ref()
    }

    /// Clears the edge preview.
    pub fn clear_edge_preview(&mut self) {
        self.edge_preview = None;
    }

    // ========== Internal API (used by mouse handlers) ==========

    /// Creates an edge preview from raw fields. Used by mouse-down handlers.
    pub(crate) fn set_edge_preview_raw(
        &mut self,
        from_node_id: String,
        from_handle_id: Option<String>,
        from_handle_type: HandleType,
        to_world: Position,
    ) {
        self.edge_preview = Some(EdgePreview {
            from_node_id,
            from_handle_id,
            from_handle_type,
            to_world: Some(to_world),
            to_node_id: None,
            to_handle_id: None,
            is_valid: None,
        });
    }

    /// Updates the edge preview to a world position, resolving the nearest
    /// compatible handle. Used by mouse-drag handlers.
    pub(crate) fn update_edge_preview_to_position(&mut self, world_pos: Position) {
        let Some(mut ep) = self.edge_preview.take() else {
            return;
        };

        let resolved = self
            .find_connectable_handle_by_position(world_pos, &ep.from_node_id, ep.from_handle_type)
            .map(|h| (h.node_id.clone(), h.id.clone()));

        ep.to_world = Some(world_pos);

        if let Some((to_node, to_handle)) = resolved {
            ep.is_valid = Some(self.is_valid_preview_connection(
                &ep.from_node_id,
                ep.from_handle_id.as_deref(),
                &to_node,
                to_handle.as_deref(),
            ));
            ep.to_node_id = Some(to_node);
            ep.to_handle_id = to_handle;
        } else {
            ep.to_node_id = None;
            ep.to_handle_id = None;
            ep.is_valid = None;
        }

        self.edge_preview = Some(ep);
    }

    /// Takes the edge preview and returns a normalized [`Connection`] if valid.
    /// Clears the preview unconditionally. Used by mouse-up handlers.
    pub(crate) fn take_edge_preview_connection(&mut self) -> Option<Connection> {
        let ep = self.edge_preview.take()?;
        (ep.is_valid == Some(true))
            .then(|| ep.to_connection())
            .flatten()
    }

    // ========== Handle Resolution ==========

    /// Checks whether a preview connection is valid (not a duplicate and passes the validator).
    fn is_valid_preview_connection(
        &self,
        from_node_id: &str,
        from_handle_id: Option<&str>,
        to_node_id: &str,
        to_handle_id: Option<&str>,
    ) -> bool {
        !self.connection_exists(from_node_id, from_handle_id, to_node_id, to_handle_id)
            && self.is_connection_valid(&Connection::new(
                from_node_id,
                from_handle_id.map(str::to_string),
                to_node_id,
                to_handle_id.map(str::to_string),
            ))
    }

    /// Validates a target node for edge preview compatibility.
    ///
    /// Returns the node if it exists, is visible, connectable, and not the source node.
    fn validated_preview_target<'a>(
        &'a self,
        to_node_id: &str,
        from_node_id: &str,
    ) -> Option<&'a crate::types::InternalNode<N>> {
        if to_node_id == from_node_id {
            return None;
        }
        let node = self.internal_node(to_node_id)?;
        if node.node.hidden || !node.node.connectable {
            return None;
        }
        Some(node)
    }

    /// Finds the closest connectable handle within `connection_radius` of a world position.
    ///
    /// Scans all nodes, respecting connection mode (strict/loose), connectability flags,
    /// and self-loop prevention.
    pub(crate) fn find_connectable_handle_by_position(
        &self,
        world_pos: Position,
        from_node_id: &str,
        from_handle_type: HandleType,
    ) -> Option<&crate::types::ComputedHandle> {
        let mut best: Option<(&crate::types::ComputedHandle, f64)> = None;

        for node in &self.nodes {
            if node.node.hidden || node.id() == from_node_id || !node.node.connectable {
                continue;
            }

            for handle in
                compatible_targets(&node.handle_bounds, from_handle_type, self.connection_mode)
            {
                let dist = handle.absolute_position.distance_to(&world_pos);
                if dist <= self.connection_radius && best.as_ref().is_none_or(|(_, d)| dist < *d) {
                    best = Some((handle, dist));
                }
            }
        }

        best.map(|(handle, _)| handle)
    }
}
