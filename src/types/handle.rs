//! Handle types for node connection points.
//!
//! Handles are connection points where edges attach to nodes. They support:
//! - Multiple connection points per node
//! - Directed connections (source/target types)
//! - Custom positioning and offset along node edges
//! - Interactive edge creation (dragging from handles)

use super::connection::ConnectionMode;
use super::geometry::{Dimensions, Position, Rect};
use crate::ui::HandleStyle;

/// The side of a node where a handle is positioned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HandlePosition {
    /// Handle on the top edge of the node.
    Top,
    /// Handle on the right edge of the node.
    Right,
    /// Handle on the bottom edge of the node.
    Bottom,
    /// Handle on the left edge of the node.
    Left,
}

impl HandlePosition {
    /// The side of `from` that a ray toward `toward` exits through.
    ///
    /// The offset is weighed against each half-extent rather than against itself,
    /// so the answer respects the node's shape — the same way xyflow picks sides
    /// for floating edges. A wide, short node has a long top edge lying close to
    /// its center, so a point up and to the right exits through `Top`; a tall,
    /// narrow node at the very same offset exits through `Right`. Comparing `|dx|`
    /// against `|dy|` alone would give both the same answer.
    pub fn facing(from: &Rect, toward: Position) -> Self {
        let center = from.center();
        let (dx, dy) = (toward.x - center.x, toward.y - center.y);
        let (half_w, half_h) = (from.width() / 2.0, from.height() / 2.0);

        if (dx.abs() * half_h) > (dy.abs() * half_w) {
            if dx > 0.0 { Self::Right } else { Self::Left }
        } else if dy > 0.0 {
            Self::Bottom
        } else {
            Self::Top
        }
    }

    /// Every side, in clockwise order from the top.
    pub const ALL: [HandlePosition; 4] = [
        HandlePosition::Top,
        HandlePosition::Right,
        HandlePosition::Bottom,
        HandlePosition::Left,
    ];

    /// Terminal-space offset for an edge endpoint arriving on this side.
    ///
    /// Applied after the zoom transform, so it does not scale. Right and bottom
    /// endpoints already sit one cell outside the border and need no nudge; top and
    /// left sit on it and are pushed outward by one.
    ///
    /// Keyed on the side rather than on a handle because the side is what the
    /// geometry depends on, and an edge may leave from a side its handle does not
    /// sit on — [`Path`](crate::Path) carries the sides actually used.
    pub fn edge_endpoint_render_offset(&self) -> (i32, i32) {
        match self {
            Self::Bottom | Self::Right => (0, 0),
            Self::Top => (0, -1),
            Self::Left => (-1, 0),
        }
    }

    /// Lowercase name of the side, for use as a handle ID.
    ///
    /// Pairs with [`Edge::with_source_side`](crate::Edge::with_source_side) so an
    /// edge can name a side rather than invent an ID convention.
    pub fn side_name(&self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
        }
    }

    /// Returns the position on the opposite side.
    pub fn opposite(&self) -> Self {
        match self {
            HandlePosition::Top => HandlePosition::Bottom,
            HandlePosition::Right => HandlePosition::Left,
            HandlePosition::Bottom => HandlePosition::Top,
            HandlePosition::Left => HandlePosition::Right,
        }
    }

    /// Returns true if this is a horizontal position (left or right).
    pub fn is_horizontal(&self) -> bool {
        matches!(self, HandlePosition::Left | HandlePosition::Right)
    }

    /// Returns true if this is a vertical position (top or bottom).
    pub fn is_vertical(&self) -> bool {
        matches!(self, HandlePosition::Top | HandlePosition::Bottom)
    }

    /// Returns the default handle ID for this position.
    pub fn default_id(&self) -> &'static str {
        match self {
            HandlePosition::Top => "top",
            HandlePosition::Right => "right",
            HandlePosition::Bottom => "bottom",
            HandlePosition::Left => "left",
        }
    }
}

/// The type of handle (source or target for edges).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HandleType {
    /// Source handle — edges originate from here.
    Source,
    /// Target handle — edges terminate here.
    Target,
}

impl HandleType {
    /// Returns the opposite handle type.
    pub fn opposite(&self) -> Self {
        match self {
            HandleType::Source => HandleType::Target,
            HandleType::Target => HandleType::Source,
        }
    }
}

/// A handle definition on a node.
///
/// Handles are connection points where edges attach to nodes.
/// Each handle has a position on the node and a type (source or target).
///
/// Handle IDs are optional — `None` means "the only handle of this type on this node".
/// When a node has multiple handles of the same type, each must have a unique ID.
///
/// # Per-Handle Styling
///
/// Each handle can have its own style via the `style` field. The rendering
/// uses a fallback chain:
/// 1. Non-connectable node: `handle.disabled_style` → [`HandleStyle::disabled()`] → theme muted color
/// 2. Connectable node: `handle.style` → [`HandleStyle::default()`] → theme accent color
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Handle {
    /// Unique identifier for this handle within the node (per handle type).
    /// `None` means this is the only handle of its type on the node.
    #[cfg_attr(feature = "serde", serde(default))]
    pub id: Option<String>,
    /// The side of the node where the handle is positioned.
    pub position: HandlePosition,
    /// Whether this is a source or target handle.
    pub handle_type: HandleType,
    /// Offset along the edge (0.0 = start, 1.0 = end, 0.5 = center).
    /// For example, on a Right handle, 0.0 is top, 1.0 is bottom.
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::f64_half"))]
    pub offset: f64,
    /// Optional per-handle style for the connectable state. If `None`, falls back to
    /// the theme's default for the handle type (source or target).
    #[cfg_attr(feature = "serde", serde(skip))]
    pub style: Option<HandleStyle>,
    /// Optional per-handle style for the disabled (non-connectable) state. If `None`,
    /// falls back to the theme's disabled style.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub disabled_style: Option<HandleStyle>,
    /// Whether this handle can participate in connections at all.
    /// When false, the handle is completely non-interactive for connections.
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::bool_true"))]
    pub connectable: bool,
    /// Whether connections can start from this handle (drag begins here).
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::bool_true"))]
    pub connectable_start: bool,
    /// Whether connections can end on this handle (drop target).
    #[cfg_attr(feature = "serde", serde(default = "super::serde_defaults::bool_true"))]
    pub connectable_end: bool,
    /// Whether this handle is hidden.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hidden: bool,
}

impl Handle {
    /// Creates a new handle with default settings.
    ///
    /// The ID defaults to `None`, meaning "the only handle of this type".
    pub fn new(position: HandlePosition, handle_type: HandleType) -> Self {
        Self {
            id: None,
            position,
            handle_type,
            offset: 0.5, // Center by default
            style: None,
            disabled_style: None,
            connectable: true,
            connectable_start: true,
            connectable_end: true,
            hidden: false,
        }
    }

    /// Creates a source handle at the given position.
    pub fn source(position: HandlePosition) -> Self {
        Self::new(position, HandleType::Source)
    }

    /// Creates a target handle at the given position.
    pub fn target(position: HandlePosition) -> Self {
        Self::new(position, HandleType::Target)
    }

    /// Sets the handle ID.
    ///
    /// Required when a node has multiple handles of the same type.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the offset along the edge.
    pub fn with_offset(mut self, offset: f64) -> Self {
        self.offset = offset.clamp(0.0, 1.0);
        self
    }

    /// Sets the handle style for the connectable state.
    ///
    /// This enables per-handle styling. When set, this style takes precedence
    /// over the theme's default for the handle type.
    pub fn with_style(mut self, style: HandleStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the handle style for the disabled (non-connectable) state.
    ///
    /// When a node is non-connectable, this style takes precedence over
    /// the theme's disabled style.
    pub fn with_disabled_style(mut self, style: HandleStyle) -> Self {
        self.disabled_style = Some(style);
        self
    }

    /// Sets whether this handle can participate in connections.
    pub fn with_connectable(mut self, connectable: bool) -> Self {
        self.connectable = connectable;
        self
    }

    /// Sets whether connections can start from this handle.
    pub fn with_connectable_start(mut self, connectable_start: bool) -> Self {
        self.connectable_start = connectable_start;
        self
    }

    /// Sets whether connections can end on this handle.
    pub fn with_connectable_end(mut self, connectable_end: bool) -> Self {
        self.connectable_end = connectable_end;
        self
    }

    /// Sets whether this handle is hidden.
    pub fn with_hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Calculates the absolute position of this handle given node position and dimensions.
    ///
    /// Returns the position at the node's edge in world coordinates.
    pub fn absolute_position(&self, node_pos: &Position, node_dims: &Dimensions) -> Position {
        match self.position {
            HandlePosition::Top => {
                Position::new(node_pos.x + node_dims.width * self.offset, node_pos.y)
            }
            HandlePosition::Right => Position::new(
                node_pos.x + node_dims.width,
                node_pos.y + node_dims.height * self.offset,
            ),
            HandlePosition::Bottom => Position::new(
                node_pos.x + node_dims.width * self.offset,
                node_pos.y + node_dims.height,
            ),
            HandlePosition::Left => {
                Position::new(node_pos.x, node_pos.y + node_dims.height * self.offset)
            }
        }
    }
}

/// Computed handle bounds for a node, cached for performance.
///
/// Stores the absolute positions of all handles on a node,
/// computed after the node's dimensions are known.
#[derive(Debug, Clone, Default)]
pub(crate) struct HandleBounds {
    /// Source handles with their absolute positions.
    pub source: Vec<ComputedHandle>,
    /// Target handles with their absolute positions.
    pub target: Vec<ComputedHandle>,
}

impl HandleBounds {
    /// Creates empty handle bounds.
    pub fn new() -> Self {
        Self {
            source: Vec::new(),
            target: Vec::new(),
        }
    }

    /// Computes handle bounds from handle definitions.
    ///
    /// If no explicit handles are provided, creates default handles using
    /// the node's source_position and target_position with `id: None`.
    pub fn compute(
        handles: &[Handle],
        node_pos: &Position,
        node_dims: &Dimensions,
        node_id: &str,
        default_source_position: HandlePosition,
        default_target_position: HandlePosition,
    ) -> Self {
        let mut source = Vec::new();
        let mut target = Vec::new();

        if handles.is_empty() {
            // Create default handles from source_position and target_position
            let default_source = Handle::source(default_source_position);
            let default_target = Handle::target(default_target_position);

            source.push(ComputedHandle {
                id: None,
                node_id: node_id.to_string(),
                position: default_source.position,
                handle_type: HandleType::Source,
                absolute_position: default_source.absolute_position(node_pos, node_dims),
                connectable: default_source.connectable,
                connectable_start: default_source.connectable_start,
                connectable_end: default_source.connectable_end,
                hidden: false,
                style: None,
                disabled_style: None,
            });

            target.push(ComputedHandle {
                id: None,
                node_id: node_id.to_string(),
                position: default_target.position,
                handle_type: HandleType::Target,
                absolute_position: default_target.absolute_position(node_pos, node_dims),
                connectable: default_target.connectable,
                connectable_start: default_target.connectable_start,
                connectable_end: default_target.connectable_end,
                hidden: false,
                style: None,
                disabled_style: None,
            });
        } else {
            // Use explicit handles
            for handle in handles {
                let computed = ComputedHandle {
                    id: handle.id.clone(),
                    node_id: node_id.to_string(),
                    position: handle.position,
                    handle_type: handle.handle_type,
                    absolute_position: handle.absolute_position(node_pos, node_dims),
                    connectable: handle.connectable,
                    connectable_start: handle.connectable_start,
                    connectable_end: handle.connectable_end,
                    hidden: handle.hidden,
                    style: handle.style,
                    disabled_style: handle.disabled_style,
                };

                match handle.handle_type {
                    HandleType::Source => source.push(computed),
                    HandleType::Target => target.push(computed),
                }
            }
        }

        Self { source, target }
    }

    /// Gets a handle by ID with mode-aware searching.
    ///
    /// - Strict: searches only handles of the specified type
    /// - Loose: searches all handles
    ///
    /// Falls back to first handle of the specified type if not found.
    pub fn get(
        &self,
        id: Option<&str>,
        handle_type: HandleType,
        connection_mode: ConnectionMode,
    ) -> &ComputedHandle {
        if let Some(id) = id {
            match connection_mode {
                ConnectionMode::Strict => {
                    if let Some(handle) = self
                        .by_type(handle_type)
                        .iter()
                        .find(|h| h.id.as_deref() == Some(id))
                    {
                        return handle;
                    }
                }
                ConnectionMode::Loose => {
                    if let Some(handle) = self
                        .source
                        .iter()
                        .chain(self.target.iter())
                        .find(|h| h.id.as_deref() == Some(id))
                    {
                        return handle;
                    }
                }
            }
        }
        self.by_type(handle_type)
            .first()
            .expect("HandleBounds should always have at least one handle of each type")
    }

    /// Finds a handle by ID, searching both source and target handles.
    ///
    /// Returns None if handle is not found. Unlike `get()`, this doesn't fall back.
    pub fn find(&self, id: Option<&str>, fallback_type: HandleType) -> Option<&ComputedHandle> {
        match id {
            Some(id) => self
                .source
                .iter()
                .chain(self.target.iter())
                .find(|h| h.id.as_deref() == Some(id)),
            None => match fallback_type {
                HandleType::Source => self.source.first(),
                HandleType::Target => self.target.first(),
            },
        }
    }

    /// Returns all handles of a given type.
    pub fn by_type(&self, handle_type: HandleType) -> &[ComputedHandle] {
        match handle_type {
            HandleType::Source => &self.source,
            HandleType::Target => &self.target,
        }
    }
}

/// A handle with its computed absolute position.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ComputedHandle {
    /// The handle's ID. `None` means "the only handle of this type".
    pub id: Option<String>,
    /// The node this handle belongs to.
    pub node_id: String,
    /// The position on the node edge.
    pub position: HandlePosition,
    /// Whether this is a source or target handle.
    pub handle_type: HandleType,
    /// The absolute position in world coordinates.
    pub absolute_position: Position,
    /// Whether this handle can participate in connections.
    pub connectable: bool,
    /// Whether connections can start from this handle.
    pub connectable_start: bool,
    /// Whether connections can end on this handle.
    pub connectable_end: bool,
    /// Whether this handle is hidden.
    pub hidden: bool,
    /// Optional per-handle style for the connectable state.
    pub style: Option<HandleStyle>,
    /// Optional per-handle style for the disabled (non-connectable) state.
    pub disabled_style: Option<HandleStyle>,
}

impl ComputedHandle {
    /// Returns the terminal-space offset for edge endpoints.
    ///
    /// Returns the terminal-space offset for rendering handles on the node border.
    ///
    /// Right/Bottom handles need -1 offset to sit on the last pixel of the node,
    /// not one pixel outside. Applied AFTER zoom transform.
    pub fn handle_render_offset(&self) -> (i32, i32) {
        match self.position {
            HandlePosition::Top | HandlePosition::Left => (0, 0),
            HandlePosition::Right => (-1, 0),
            HandlePosition::Bottom => (0, -1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect::new(Position::new(x, y), Dimensions::new(w, h))
    }

    #[test]
    fn facing_respects_the_node_shape_not_just_the_offset() {
        // Identical diagonal offset, opposite answers — the near edge wins.
        // A wide, short node has its top edge close by, so the ray exits there
        // even though the target is equally far to the right.
        let wide = rect(0.0, 0.0, 100.0, 10.0);
        let center = wide.center();
        assert_eq!(
            HandlePosition::facing(&wide, Position::new(center.x + 30.0, center.y - 30.0)),
            HandlePosition::Top
        );

        // A tall, narrow node has its side edges close by, so the same offset
        // exits right. Comparing |dx| against |dy| alone would give one answer
        // for both.
        let tall = rect(0.0, 0.0, 10.0, 100.0);
        let center = tall.center();
        assert_eq!(
            HandlePosition::facing(&tall, Position::new(center.x + 30.0, center.y - 30.0)),
            HandlePosition::Right
        );
    }

    #[test]
    fn facing_covers_all_four_sides() {
        let square = rect(0.0, 0.0, 10.0, 10.0);
        let center = square.center();
        for (dx, dy, expected) in [
            (100.0, 0.0, HandlePosition::Right),
            (-100.0, 0.0, HandlePosition::Left),
            (0.0, 100.0, HandlePosition::Bottom),
            (0.0, -100.0, HandlePosition::Top),
        ] {
            assert_eq!(
                HandlePosition::facing(&square, Position::new(center.x + dx, center.y + dy)),
                expected,
                "offset ({dx}, {dy})"
            );
        }
    }

    #[test]
    fn test_handle_absolute_position() {
        let node_pos = Position::new(10.0, 20.0);
        let node_dims = Dimensions::new(100.0, 50.0);

        // Right handle at center - returns edge position in world space
        // The -1 offset to place ON border is applied in terminal space
        let handle = Handle::source(HandlePosition::Right);
        let pos = handle.absolute_position(&node_pos, &node_dims);
        assert_eq!(pos.x, 110.0); // 10 + 100 (edge of node)
        assert_eq!(pos.y, 45.0); // 20 + 50 * 0.5

        // Top handle at start
        let handle = Handle::target(HandlePosition::Top).with_offset(0.0);
        let pos = handle.absolute_position(&node_pos, &node_dims);
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);

        // Bottom handle at end - returns edge position in world space
        let handle = Handle::source(HandlePosition::Bottom).with_offset(1.0);
        let pos = handle.absolute_position(&node_pos, &node_dims);
        assert_eq!(pos.x, 110.0); // 10 + 100 * 1.0
        assert_eq!(pos.y, 70.0); // 20 + 50 (edge of node)
    }

    #[test]
    fn test_handle_bounds_compute_explicit() {
        let handles = vec![
            Handle::source(HandlePosition::Right).with_id("out"),
            Handle::target(HandlePosition::Left).with_id("in"),
        ];

        let bounds = HandleBounds::compute(
            &handles,
            &Position::new(0.0, 0.0),
            &Dimensions::new(10.0, 10.0),
            "node1",
            HandlePosition::Right,
            HandlePosition::Left,
        );

        assert_eq!(bounds.source.len(), 1);
        assert_eq!(bounds.target.len(), 1);
        assert_eq!(bounds.source[0].id, Some("out".to_string()));
        assert_eq!(bounds.target[0].id, Some("in".to_string()));
    }

    #[test]
    fn test_handle_bounds_compute_defaults() {
        // Empty handles array should create default handles with None IDs
        let handles: Vec<Handle> = vec![];

        let bounds = HandleBounds::compute(
            &handles,
            &Position::new(0.0, 0.0),
            &Dimensions::new(10.0, 10.0),
            "node1",
            HandlePosition::Right,
            HandlePosition::Left,
        );

        // Should have default source and target handles with None IDs
        assert_eq!(bounds.source.len(), 1);
        assert_eq!(bounds.target.len(), 1);
        assert_eq!(bounds.source[0].id, None);
        assert_eq!(bounds.target[0].id, None);
        assert_eq!(bounds.source[0].position, HandlePosition::Right);
        assert_eq!(bounds.target[0].position, HandlePosition::Left);
    }

    #[test]
    fn test_get_mode_aware_lookup() {
        let handles = vec![
            Handle::source(HandlePosition::Right).with_id("src"),
            Handle::target(HandlePosition::Left).with_id("tgt"),
        ];

        let bounds = HandleBounds::compute(
            &handles,
            &Position::new(0.0, 0.0),
            &Dimensions::new(10.0, 10.0),
            "node1",
            HandlePosition::Right,
            HandlePosition::Left,
        );

        // Strict mode: only searches specified type
        let handle = bounds.get(Some("src"), HandleType::Source, ConnectionMode::Strict);
        assert_eq!(handle.id, Some("src".to_string()));

        // Strict mode: searching for target ID in source type fails, falls back
        let handle = bounds.get(Some("tgt"), HandleType::Source, ConnectionMode::Strict);
        assert_eq!(handle.id, Some("src".to_string())); // Falls back to first source

        // Loose mode: searches all handles regardless of type
        let handle = bounds.get(Some("tgt"), HandleType::Source, ConnectionMode::Loose);
        assert_eq!(handle.id, Some("tgt".to_string())); // Finds target handle even when asking for Source
    }
}
