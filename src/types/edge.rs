//! Edge types for flow graphs.
//!
//! Edges connect nodes in the flow graph. They support:
//! - Directed connections (source → target)
//! - Multiple handle connections per node
//! - Generic content types for custom rendering and payloads
//! - Custom edge rendering via the [`EdgeContent`](crate::EdgeContent) trait
//! - Edge markers (arrows, etc.)

use std::fmt::Debug;

use super::{HandlePosition, HandleType};

/// Controls which ends of an edge can be reconnected by dragging.
///
/// Per-edge setting that can override the global
/// [`Flow::edges_reconnectable`](crate::Flow) default.
/// Reconnection requires the edge to be selected first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Reconnectable {
    /// Use the global `edges_reconnectable` setting.
    #[default]
    Inherit,
    /// Both source and target ends can be reconnected.
    Both,
    /// Only the source end can be reconnected.
    Source,
    /// Only the target end can be reconnected.
    Target,
    /// Cannot be reconnected (overrides global setting).
    None,
}

impl Reconnectable {
    /// Whether this setting allows reconnecting the given handle type,
    /// falling back to the global default when `Inherit`.
    pub fn allows(&self, handle_type: HandleType, global_default: bool) -> bool {
        match self {
            Reconnectable::Inherit => global_default,
            Reconnectable::Both => true,
            Reconnectable::Source => handle_type == HandleType::Source,
            Reconnectable::Target => handle_type == HandleType::Target,
            Reconnectable::None => false,
        }
    }
}

/// An edge connecting two nodes in the flow graph.
///
/// Generic over:
/// - `E`: The content type stored in the edge (implements [`EdgeContent`](crate::EdgeContent))
///
/// # Example
///
/// ```no_run
/// use rataflow::{Edge, StepEdge};
///
/// // Edge with default StepEdge content
/// let edge: Edge<StepEdge> = Edge::new("e1", "node1", "node2");
///
/// // Edge with custom content
/// #[derive(Clone, Debug, Default)]
/// struct MyEdge { weight: f64 }
///
/// let edge = Edge::new("e2", "node1", "node2").with_content(MyEdge { weight: 1.5 });
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(bound(
        serialize = "E: serde::Serialize",
        deserialize = "E: serde::de::DeserializeOwned",
    ))
)]
pub struct Edge<E = ()> {
    /// Unique identifier for this edge.
    pub id: String,
    /// ID of the source node.
    pub source: String,
    /// ID of the target node.
    pub target: String,
    /// ID of the source handle to connect from.
    ///
    /// If `None`, uses the node's default source handle (the first source handle,
    /// typically based on the node's `source_position`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_handle: Option<String>,
    /// ID of the target handle to connect to.
    ///
    /// If `None`, uses the node's default target handle (the first target handle,
    /// typically based on the node's `target_position`).
    #[cfg_attr(feature = "serde", serde(default))]
    pub target_handle: Option<String>,
    /// Content associated with this edge (implements [`EdgeContent`](crate::EdgeContent) for rendering).
    pub content: E,
    /// Whether the edge is hidden.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hidden: bool,
    /// Whether the edge can be deleted.
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::bool_true"))]
    pub deletable: bool,
    /// Whether the edge can be selected.
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::bool_true"))]
    pub selectable: bool,
    /// Whether the edge is currently selected.
    #[cfg_attr(feature = "serde", serde(default))]
    pub selected: bool,
    /// Whether the edge is animated (marching ants pattern).
    ///
    /// Requires calling [`Flow::tick_animation`](crate::Flow::tick_animation)
    /// in your event loop with elapsed time.
    #[cfg_attr(feature = "serde", serde(default))]
    pub animated: bool,
    /// Optional label to display on the edge.
    #[cfg_attr(feature = "serde", serde(default))]
    pub label: Option<String>,
    /// Whether this edge's endpoints can be reconnected by dragging.
    ///
    /// `Inherit` defers to [`Flow::edges_reconnectable`](crate::Flow).
    /// Other variants override the global setting. The edge must be selected
    /// for reconnection to activate.
    #[cfg_attr(feature = "serde", serde(default))]
    pub reconnectable: Reconnectable,
}

impl<E: Default> Edge<E> {
    /// Creates a new edge with default content.
    pub fn new(
        id: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            target: target.into(),
            source_handle: None,
            target_handle: None,
            content: E::default(),
            hidden: false,
            deletable: true,
            selectable: true,
            selected: false,
            animated: false,
            label: None,
            reconnectable: Reconnectable::Inherit,
        }
    }
}

impl<E> Edge<E> {
    /// Sets the edge content.
    pub fn with_content(mut self, content: E) -> Self {
        self.content = content;
        self
    }

    /// Sets the source handle ID.
    pub fn with_source_handle(mut self, handle: Option<String>) -> Self {
        self.source_handle = handle;
        self
    }

    /// Sets the target handle ID.
    pub fn with_target_handle(mut self, handle: Option<String>) -> Self {
        self.target_handle = handle;
        self
    }

    /// Attaches the edge's source to a given side of its source node.
    ///
    /// Shorthand for naming a handle by its side, for nodes that carry one per side.
    /// The node needs a source handle whose ID is [`HandlePosition::side_name`];
    /// without one the edge falls back to the node's first source handle.
    pub fn with_source_side(mut self, side: HandlePosition) -> Self {
        self.source_handle = Some(side.side_name().to_string());
        self
    }

    /// Attaches the edge's target to a given side of its target node.
    ///
    /// Shorthand for naming a handle by its side, for nodes that carry one per side.
    /// The node needs a target handle whose ID is [`HandlePosition::side_name`];
    /// without one the edge falls back to the node's first target handle.
    pub fn with_target_side(mut self, side: HandlePosition) -> Self {
        self.target_handle = Some(side.side_name().to_string());
        self
    }

    /// Sets whether the edge is hidden.
    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Sets whether the edge is deletable.
    pub fn with_deletable(mut self, deletable: bool) -> Self {
        self.deletable = deletable;
        self
    }

    /// Sets whether the edge is selectable.
    pub fn with_selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Sets whether the edge is selected.
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sets whether the edge is animated (marching ants pattern).
    pub fn with_animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Sets the edge label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets whether this edge's endpoints can be reconnected by dragging.
    pub fn with_reconnectable(mut self, reconnectable: Reconnectable) -> Self {
        self.reconnectable = reconnectable;
        self
    }

    /// Returns true if this edge connects the given nodes (in either direction).
    pub fn connects(&self, node_a: &str, node_b: &str) -> bool {
        (self.source == node_a && self.target == node_b)
            || (self.source == node_b && self.target == node_a)
    }

    /// Returns true if this edge connects the given nodes in the specified direction.
    pub fn connects_directed(&self, source: &str, target: &str) -> bool {
        self.source == source && self.target == target
    }

    /// Returns true if this edge originates from the given node.
    pub fn is_from(&self, node_id: &str) -> bool {
        self.source == node_id
    }

    /// Returns true if this edge terminates at the given node.
    pub fn is_to(&self, node_id: &str) -> bool {
        self.target == node_id
    }

    /// Maps the content to a new type.
    pub fn map_content<E2>(self, f: impl FnOnce(E) -> E2) -> Edge<E2> {
        Edge {
            id: self.id,
            source: self.source,
            target: self.target,
            source_handle: self.source_handle,
            target_handle: self.target_handle,
            content: f(self.content),
            hidden: self.hidden,
            deletable: self.deletable,
            selectable: self.selectable,
            selected: self.selected,
            animated: self.animated,
            label: self.label,
            reconnectable: self.reconnectable,
        }
    }
}

/// Edge marker types for arrows and other decorations.
///
/// Markers are rendered at the start and/or end of edges. Arrow markers
/// are direction-aware (they rotate based on handle position), while
/// other markers use fixed characters.
///
/// Markers are configured via [`EdgeStyle`](crate::EdgeStyle).
///
/// # Example
///
/// ```no_run
/// use rataflow::{EdgeStyle, EdgeMarker, StepEdge};
///
/// // Default style has arrow at target
/// let style = EdgeStyle::default();
///
/// // Circle markers at both ends
/// let style = EdgeStyle::default()
///     .with_marker_start(EdgeMarker::Circle)
///     .with_marker_end(EdgeMarker::Circle);
///
/// // Custom character marker
/// let style = EdgeStyle::default()
///     .with_marker_end(EdgeMarker::Custom('★'));
///
/// // Edge content with custom markers
/// let edge_content = StepEdge::default()
///     .with_style(style);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EdgeMarker {
    /// Outline arrow marker using Unicode triangles (▷◁△▽).
    /// Direction-aware: rotates based on handle position.
    Arrow,
    /// Filled/closed arrow marker using Unicode triangles (▶◀▲▼).
    /// Direction-aware: rotates based on handle position.
    #[default]
    ArrowClosed,
    /// Circle marker (●). Not direction-aware.
    Circle,
    /// Diamond marker (◆). Not direction-aware.
    Diamond,
    /// Custom single-character marker. Not direction-aware.
    Custom(char),
    /// Custom direction-aware marker with different chars for each direction.
    CustomDirectional {
        left: char,
        right: char,
        top: char,
        bottom: char,
    },
    /// No marker.
    None,
}

impl EdgeMarker {
    /// Returns the character for this marker given the handle position.
    ///
    /// For direction-aware markers (Arrow, ArrowClosed, CustomDirectional), the character
    /// rotates based on handle position. Start markers point away from the source;
    /// end markers point into the target.
    ///
    /// Returns `None` for `EdgeMarker::None`.
    pub(crate) fn char_for_position(
        self,
        position: HandlePosition,
        is_start: bool,
    ) -> Option<char> {
        match self {
            EdgeMarker::None => None,
            EdgeMarker::Arrow => Some(select_directional(position, is_start, '◁', '▷', '△', '▽')),
            EdgeMarker::ArrowClosed => {
                Some(select_directional(position, is_start, '◀', '▶', '▲', '▼'))
            }
            EdgeMarker::Circle => Some('●'),
            EdgeMarker::Diamond => Some('◆'),
            EdgeMarker::Custom(ch) => Some(ch),
            EdgeMarker::CustomDirectional {
                left,
                right,
                top,
                bottom,
            } => Some(select_directional(
                position, is_start, left, right, top, bottom,
            )),
        }
    }
}

/// Selects a direction-aware character based on handle position.
///
/// Start markers use the position directly; end markers use the opposite
/// (pointing into the handle rather than away from it).
fn select_directional(
    position: HandlePosition,
    is_start: bool,
    left: char,
    right: char,
    top: char,
    bottom: char,
) -> char {
    let pos = if is_start {
        position
    } else {
        position.opposite()
    };
    match pos {
        HandlePosition::Left => left,
        HandlePosition::Right => right,
        HandlePosition::Top => top,
        HandlePosition::Bottom => bottom,
    }
}
