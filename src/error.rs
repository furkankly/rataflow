//! Error types for rataflow.

use thiserror::Error as ThisError;

/// Errors that can occur when working with flow graphs.
#[derive(Debug, ThisError)]
#[non_exhaustive]
pub enum Error {
    /// Edge references a non-existent node.
    #[error("edge `{edge_id}` references non-existent node `{node_id}`")]
    InvalidEdgeReference {
        /// The ID of the edge with the invalid reference.
        edge_id: String,
        /// The node ID that was not found.
        node_id: String,
    },

    /// Node references a non-existent parent.
    #[error("node `{node_id}` references non-existent parent `{parent_id}`")]
    InvalidParentReference {
        /// The ID of the node with the invalid parent reference.
        node_id: String,
        /// The parent ID that was not found.
        parent_id: String,
    },

    /// A parent assignment would make a node its own ancestor.
    #[error("node `{node_id}` cannot be parented to `{parent_id}`: that would form a cycle")]
    CyclicParent {
        /// The node being re-parented.
        node_id: String,
        /// The proposed parent, which sits below `node_id` in the tree.
        parent_id: String,
    },

    /// Duplicate node ID.
    #[error("duplicate node ID `{node_id}`")]
    DuplicateNodeId {
        /// The duplicate node ID.
        node_id: String,
    },

    /// Duplicate edge ID.
    #[error("duplicate edge ID `{edge_id}`")]
    DuplicateEdgeId {
        /// The duplicate edge ID.
        edge_id: String,
    },

    /// Self-referential edge (source equals target).
    #[error("edge `{edge_id}` has same source and target `{node_id}`")]
    SelfReferentialEdge {
        /// The ID of the self-referential edge.
        edge_id: String,
        /// The node ID that is both source and target.
        node_id: String,
    },

    /// Node not found.
    #[error("node `{node_id}` not found")]
    NodeNotFound {
        /// The ID of the node that was not found.
        node_id: String,
    },

    /// Edge not found.
    #[error("edge `{edge_id}` not found")]
    EdgeNotFound {
        /// The ID of the edge that was not found.
        edge_id: String,
    },

    /// Multiple handles of the same type without IDs.
    #[error("node `{node_id}` has {count} {handle_type} handles without IDs")]
    AmbiguousHandles {
        /// The ID of the node with ambiguous handles.
        node_id: String,
        /// The handle type with ambiguity.
        handle_type: &'static str,
        /// Number of handles without IDs.
        count: usize,
    },

    /// Duplicate handle ID within the same type.
    #[error("node `{node_id}` has duplicate {handle_type} handle ID `{handle_id}`")]
    DuplicateHandleId {
        /// The ID of the node with duplicate handle IDs.
        node_id: String,
        /// The handle type with the duplicate.
        handle_type: &'static str,
        /// The duplicate handle ID.
        handle_id: String,
    },
}
