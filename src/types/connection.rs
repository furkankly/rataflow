//! Connection types for edge creation.
//!
//! - [`Connection`] — a potential edge used for validation during interactive creation
//! - [`ConnectionMode`] — controls which handle type combinations are allowed

/// A potential edge between two handles.
///
/// Represents an edge that could be created, used for validation during
/// interactive edge creation. Unlike [`Edge`](crate::Edge), a Connection
/// has no ID (the edge doesn't exist yet) and no content.
///
/// Handle IDs are optional — `None` means "the only handle of this type on the node".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Connection {
    /// Source node ID.
    pub source: String,
    /// Source handle ID. `None` means the default (only) source handle.
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_handle: Option<String>,
    /// Target node ID.
    pub target: String,
    /// Target handle ID. `None` means the default (only) target handle.
    #[cfg_attr(feature = "serde", serde(default))]
    pub target_handle: Option<String>,
}

impl Connection {
    /// Creates a new connection.
    pub fn new(
        source: impl Into<String>,
        source_handle: Option<String>,
        target: impl Into<String>,
        target_handle: Option<String>,
    ) -> Self {
        Self {
            source: source.into(),
            source_handle,
            target: target.into(),
            target_handle,
        }
    }

    /// Returns the deterministic edge ID for this connection.
    ///
    /// Format: `{source}:{source_handle}<>{target}:{target_handle}`
    /// Handle IDs default to empty string when `None`.
    pub fn edge_id(&self) -> String {
        format!(
            "{}:{}<>{}:{}",
            self.source,
            self.source_handle.as_deref().unwrap_or(""),
            self.target,
            self.target_handle.as_deref().unwrap_or("")
        )
    }
}

/// Controls which handle type combinations are allowed for connections.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConnectionMode {
    /// Only source handles can connect to target handles (default).
    #[default]
    Strict,
    /// Any handle type combination except self-loops.
    Loose,
}
