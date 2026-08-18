//! Node types for flow graphs.
//!
//! Nodes are the primary elements in a flow graph. They support:
//! - Generic content types for custom rendering and payloads
//! - Custom node rendering via the [`NodeContent`](crate::NodeContent) trait
//! - Selection state for keyboard navigation
//! - Hierarchical relationships (parent/child)
//! - Handle definitions for edge connections

use std::fmt::Debug;

use super::geometry::{CoordinateExtent, Dimensions, NodeOrigin, Position, Rect};
use super::handle::{Handle, HandleBounds, HandlePosition};

/// A node in the flow graph.
///
/// Generic over:
/// - `N`: The content type stored in the node (implements [`NodeContent`](crate::content::NodeContent))
///
/// # Example
///
/// ```no_run
/// use rataflow::{Node, Position, Dimensions};
///
/// #[derive(Clone, Debug)]
/// struct MyContent {
///     label: String,
///     value: i32,
/// }
///
/// let node = Node::new("node1", Position::new(10.0, 20.0), (100.0, 50.0), MyContent {
///     label: "Hello".to_string(),
///     value: 42,
/// });
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(bound(
        serialize = "N: serde::Serialize",
        deserialize = "N: serde::de::DeserializeOwned",
    ))
)]
pub struct Node<N = ()> {
    /// Unique identifier for this node.
    pub id: String,
    /// Position of the node (relative to parent if has one, otherwise absolute).
    pub position: Position,
    /// Content associated with this node (implements [`NodeContent`](crate::NodeContent) for rendering).
    pub content: N,
    /// Width of the node.
    pub width: f64,
    /// Height of the node.
    pub height: f64,
    /// Whether the node is hidden.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hidden: bool,
    /// Z-index for layering. Higher values render on top. Default: 0.
    ///
    /// When `elevate_nodes_on_select` is enabled (default), selected nodes
    /// are rendered as if their z-index were 1000 higher.
    #[cfg_attr(feature = "serde", serde(default))]
    pub z_index: i32,
    /// Whether the node is currently selected.
    #[cfg_attr(feature = "serde", serde(default))]
    pub selected: bool,
    /// Whether the node can be selected.
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::bool_true"))]
    pub selectable: bool,
    /// Whether the node can be deleted.
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::bool_true"))]
    pub deletable: bool,
    /// Whether the node can be dragged.
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::bool_true"))]
    pub draggable: bool,
    /// Whether the node can be resized by dragging its bottom-right grip.
    #[cfg_attr(feature = "serde", serde(default))]
    pub resizable: bool,
    /// Whether this node's handles can participate in connections.
    /// Acts as a master switch — when false, ALL handles on this node are non-connectable.
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::bool_true"))]
    pub connectable: bool,
    /// Optional parent node ID for hierarchical graphs.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parent_id: Option<String>,
    /// Extent constraint for this node's position.
    #[cfg_attr(feature = "serde", serde(default))]
    pub extent: Option<NodeExtent>,
    /// Whether to expand parent to fit this node.
    #[cfg_attr(feature = "serde", serde(default))]
    pub expand_parent: bool,
    /// Whether the node blocks content behind it.
    ///
    /// When `true` (default), the entire node area is solid — edges and other
    /// nodes behind it are hidden. When `false`, only the parts you explicitly
    /// render are visible; everything else shows through (useful for parent
    /// nodes in hierarchical graphs where edges should be visible inside).
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::bool_true"))]
    pub opaque: bool,
    /// Origin point for position calculations.
    #[cfg_attr(feature = "serde", serde(default))]
    pub origin: NodeOrigin,
    /// Handle definitions for this node.
    #[cfg_attr(feature = "serde", serde(default))]
    pub handles: Vec<Handle>,
    /// Default position for source handles if not explicitly defined.
    #[cfg_attr(
        feature = "serde",
        serde(default = "super::serde_defaults::handle_position_right")
    )]
    pub source_position: HandlePosition,
    /// Default position for target handles if not explicitly defined.
    #[cfg_attr(
        feature = "serde",
        serde(default = "super::serde_defaults::handle_position_left")
    )]
    pub target_position: HandlePosition,
}

impl<N> Node<N> {
    /// Creates a new node with the given ID, position, dimensions, and content.
    pub fn new(
        id: impl Into<String>,
        position: impl Into<Position>,
        dimensions: impl Into<Dimensions>,
        content: N,
    ) -> Self {
        let dims = dimensions.into();
        Self {
            id: id.into(),
            position: position.into(),
            content,
            width: dims.width,
            height: dims.height,
            hidden: false,
            z_index: 0,
            selected: false,
            selectable: true,
            deletable: true,
            draggable: true,
            resizable: false,
            connectable: true,
            parent_id: None,
            extent: None,
            expand_parent: false,
            opaque: true,
            origin: NodeOrigin::default(),
            handles: Vec::new(),
            source_position: HandlePosition::Right,
            target_position: HandlePosition::Left,
        }
    }

    /// Sets the node dimensions.
    pub fn with_dimensions(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Adds padding around the content.
    ///
    /// Increases dimensions by the specified amounts:
    /// - `horizontal`: added to both left and right (total width += 2 * horizontal)
    /// - `vertical`: added to both top and bottom (total height += 2 * vertical)
    pub fn with_padding(mut self, horizontal: f64, vertical: f64) -> Self {
        self.width += horizontal * 2.0;
        self.height += vertical * 2.0;
        self
    }

    /// Sets whether the node is hidden.
    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Sets the z-index for layering.
    pub fn with_z_index(mut self, z_index: i32) -> Self {
        self.z_index = z_index;
        self
    }

    /// Sets whether the node is selected.
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sets whether the node is selectable.
    pub fn with_selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Sets whether the node is deletable.
    pub fn with_deletable(mut self, deletable: bool) -> Self {
        self.deletable = deletable;
        self
    }

    /// Sets whether the node is draggable.
    pub fn with_draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Sets whether the node can be resized by dragging its bottom-right grip.
    ///
    /// Resizing starts within
    /// [`Flow::resize_handle_radius`](crate::Flow::resize_handle_radius) of a
    /// corner and is clamped to
    /// [`Flow::min_node_size`](crate::Flow::min_node_size).
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Sets whether this node's handles can participate in connections.
    pub fn with_connectable(mut self, connectable: bool) -> Self {
        self.connectable = connectable;
        self
    }

    /// Sets the parent node ID.
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Sets the extent constraint.
    pub fn with_extent(mut self, extent: impl Into<Option<NodeExtent>>) -> Self {
        self.extent = extent.into();
        self
    }

    /// Sets whether to expand the parent node to fit this child.
    pub fn with_expand_parent(mut self, expand_parent: bool) -> Self {
        self.expand_parent = expand_parent;
        self
    }

    /// Sets whether the node blocks content behind it.
    pub fn with_opaque(mut self, opaque: bool) -> Self {
        self.opaque = opaque;
        self
    }

    /// Sets the origin point.
    pub fn with_origin(mut self, origin: NodeOrigin) -> Self {
        self.origin = origin;
        self
    }

    /// Sets the handles for this node.
    pub fn with_handles(mut self, handles: Vec<Handle>) -> Self {
        self.handles = handles;
        self
    }

    /// Adds a handle to this node.
    pub fn add_handle(mut self, handle: Handle) -> Self {
        self.handles.push(handle);
        self
    }

    /// Sets the default source handle position.
    pub fn with_source_position(mut self, position: HandlePosition) -> Self {
        self.source_position = position;
        self
    }

    /// Sets the default target handle position.
    pub fn with_target_position(mut self, position: HandlePosition) -> Self {
        self.target_position = position;
        self
    }

    /// Materializes default handles into the `handles` vec if empty.
    ///
    /// When no explicit handles are defined, `HandleBounds::compute()` creates
    /// defaults internally. This method promotes those defaults into `handles`
    /// so that per-handle mutations (hidden, style, etc.) have something to act on.
    pub(crate) fn ensure_explicit_handles(&mut self) {
        if self.handles.is_empty() {
            self.handles = vec![
                Handle::source(self.source_position),
                Handle::target(self.target_position),
            ];
        }
    }

    /// Returns the node dimensions.
    pub fn dimensions(&self) -> Dimensions {
        Dimensions::new(self.width, self.height)
    }

    /// Returns the bounding rectangle.
    pub fn bounds(&self) -> Rect {
        let dims = self.dimensions();
        let offset = self.origin.offset(&dims);
        Rect::new(self.position + offset, dims)
    }
}

impl<N> Node<N> {
    /// Maps the content to a new type.
    pub fn map_content<N2>(self, f: impl FnOnce(N) -> N2) -> Node<N2> {
        Node {
            id: self.id,
            position: self.position,
            content: f(self.content),
            width: self.width,
            height: self.height,
            hidden: self.hidden,
            z_index: self.z_index,
            selected: self.selected,
            selectable: self.selectable,
            deletable: self.deletable,
            draggable: self.draggable,
            resizable: self.resizable,
            connectable: self.connectable,
            parent_id: self.parent_id,
            extent: self.extent,
            expand_parent: self.expand_parent,
            opaque: self.opaque,
            origin: self.origin,
            handles: self.handles,
            source_position: self.source_position,
            target_position: self.target_position,
        }
    }
}

/// Extent constraint for a node's position.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeExtent {
    /// Constrain the node within its parent's bounds.
    Parent,
    /// Constrain the node within specific coordinates.
    Coordinates(CoordinateExtent),
}

/// Converts a node's position (which refers to the origin point) to top-left coordinates.
pub(crate) fn get_position_with_origin<D>(node: &Node<D>) -> Position {
    let dims = node.dimensions();
    node.position + node.origin.offset(&dims)
}

/// Internal node representation with computed values.
///
/// Wraps a user-provided [`Node`] with computed fields needed for rendering
/// and interaction (absolute position, handle bounds).
#[derive(Debug, Clone)]
pub(crate) struct InternalNode<N = ()> {
    /// The original user node.
    pub node: Node<N>,
    /// Computed absolute position of the node's TOP-LEFT corner in world coordinates.
    pub position_absolute: Position,
    /// Cached handle bounds.
    pub handle_bounds: HandleBounds,
    /// Effective z-index (base z_index + selection elevation + child-above-parent).
    /// Computed by [`Flow::ensure_z_order`].
    pub effective_z: i32,
}

impl<N> InternalNode<N> {
    /// Creates an internal node from a user node.
    pub fn from_node(node: Node<N>) -> Self {
        let position_absolute = get_position_with_origin(&node);
        Self {
            handle_bounds: HandleBounds::new(),
            position_absolute,
            node,
            effective_z: 0,
        }
    }

    /// Returns the node ID.
    pub fn id(&self) -> &str {
        &self.node.id
    }

    /// Returns the node dimensions.
    pub fn dimensions(&self) -> Dimensions {
        self.node.dimensions()
    }

    /// Returns the bounding rectangle using absolute position.
    pub fn bounds(&self) -> Rect {
        Rect::new(self.position_absolute, self.dimensions())
    }

    /// Updates the handle bounds based on current position and dimensions.
    pub fn update_handle_bounds(&mut self) {
        self.handle_bounds = HandleBounds::compute(
            &self.node.handles,
            &self.position_absolute,
            &self.node.dimensions(),
            &self.node.id,
            self.node.source_position,
            self.node.target_position,
        );
    }

    /// Updates position_absolute for a child node given the parent's position_absolute.
    pub fn update_position_from_parent(&mut self, parent_position_absolute: Position) {
        let relative_top_left = get_position_with_origin(&self.node);
        self.position_absolute = parent_position_absolute + relative_top_left;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_position_with_origin_top_left() {
        let node =
            Node::new("test", (50.0, 50.0), (100.0, 50.0), ()).with_origin(NodeOrigin::TOP_LEFT);

        let top_left = get_position_with_origin(&node);
        // TOP_LEFT origin: position IS the top-left
        assert_eq!(top_left.x, 50.0);
        assert_eq!(top_left.y, 50.0);
    }

    #[test]
    fn test_get_position_with_origin_center() {
        let node =
            Node::new("test", (50.0, 50.0), (100.0, 50.0), ()).with_origin(NodeOrigin::CENTER);

        let top_left = get_position_with_origin(&node);
        // CENTER origin: top_left = position - (width/2, height/2)
        assert_eq!(top_left.x, 0.0); // 50 - 100/2
        assert_eq!(top_left.y, 25.0); // 50 - 50/2
    }

    #[test]
    fn test_get_position_with_origin_bottom_right() {
        let node = Node::new("test", (100.0, 50.0), (100.0, 50.0), ())
            .with_origin(NodeOrigin::BOTTOM_RIGHT);

        let top_left = get_position_with_origin(&node);
        // BOTTOM_RIGHT origin: top_left = position - (width, height)
        assert_eq!(top_left.x, 0.0); // 100 - 100
        assert_eq!(top_left.y, 0.0); // 50 - 50
    }

    #[test]
    fn test_internal_node_with_center_origin() {
        let node =
            Node::new("test", (50.0, 50.0), (100.0, 50.0), ()).with_origin(NodeOrigin::CENTER);

        let internal = InternalNode::from_node(node);

        // position_absolute should be the top-left corner
        assert_eq!(internal.position_absolute.x, 0.0); // 50 - 100/2
        assert_eq!(internal.position_absolute.y, 25.0); // 50 - 50/2

        // bounds should use position_absolute directly
        let bounds = internal.bounds();
        assert_eq!(bounds.x(), 0.0);
        assert_eq!(bounds.y(), 25.0);
        assert_eq!(bounds.width(), 100.0);
        assert_eq!(bounds.height(), 50.0);
    }

    #[test]
    fn test_internal_node_hierarchy() {
        // Parent at (100, 100) with dimensions 200x100
        let parent = Node::new("parent", (100.0, 100.0), (200.0, 100.0), ());
        let parent_internal = InternalNode::from_node(parent);

        // Child at (50, 25) relative to parent, with CENTER origin
        let child = Node::new("child", (50.0, 25.0), (40.0, 20.0), ())
            .with_origin(NodeOrigin::CENTER)
            .with_parent("parent");
        let mut child_internal = InternalNode::from_node(child);

        // Update child position based on parent
        child_internal.update_position_from_parent(parent_internal.position_absolute);

        // Child's relative top-left is (50 - 20, 25 - 10) = (30, 15)
        // Child's absolute top-left is parent's top-left + relative = (100 + 30, 100 + 15) = (130, 115)
        assert_eq!(child_internal.position_absolute.x, 130.0);
        assert_eq!(child_internal.position_absolute.y, 115.0);
    }
}
