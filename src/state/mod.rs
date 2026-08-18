//! Flow graph state management.
//!
//! [`Flow`] is the core type — it holds graph topology (nodes, edges), viewport,
//! selection, interaction state, and renders directly via `impl Widget for &mut Flow`.
//! Selection lives on each entity (`node.selected` / `edge.selected`), with convenience
//! methods on `Flow` for querying and mutating it.
//!
//! # Method Overview
//!
//! ## Construction & Configuration
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`Flow::new`] | Create an empty flow |
//! | [`Flow::with_graph`] | Create flow from nodes and edges (validates graph) |
//! | [`Flow::with_uniform_width`] | Set all nodes to the widest node's width |
//! | [`Flow::with_uniform_height`] | Set all nodes to the tallest node's height |
//! | [`Flow::with_uniform_padding`] | Add uniform padding to all nodes |
//! | [`Flow::with_min_zoom`] | Set the minimum zoom level |
//! | [`Flow::with_max_zoom`] | Set the maximum zoom level |
//! | [`Flow::with_viewport`] | Set the initial viewport (pan/zoom) |
//! | [`Flow::with_connection_mode`] | Set the connection mode |
//! | [`Flow::with_locked`] | Set whether interaction mutations are locked |
//! | [`Flow::with_edges_reconnectable`] | Set whether edges are reconnectable by default |
//! | [`Flow::with_multi_select_mode`] | Set whether multi-select mode is active |
//! | [`Flow::with_theme`] | Set the color theme |
//! | [`Flow::with_elevate_nodes_on_select`] | Set whether selected nodes are z-elevated |
//! | [`Flow::with_animation_speed`] | Set the animation speed in milliseconds |
//! | [`Flow::with_handle_hit_radius`] | Set the handle hit detection radius |
//! | [`Flow::with_edge_hit_threshold`] | Set the edge hit detection threshold |
//! | [`Flow::with_connection_radius`] | Set the connection snap radius |
//! | [`Flow::with_node_drag_threshold`] | Set the node drag threshold |
//! | [`Flow::with_select_nodes_on_drag`] | Set whether nodes are selected when drag starts |
//! | [`Flow::with_deselect_on_drag`] | Set whether dragging an unselected node clears selection |
//! | [`Flow::with_deselect_on_pane_click`] | Set whether clicking empty space clears selection |
//! | [`Flow::with_selection_on_drag`] | Set whether left-drag draws a selection box |
//! | [`Flow::with_selection_reveal`] | Set the viewport response when keyboard nav changes the selection |
//! | [`Flow::with_auto_pan_on_node_drag`] | Set whether auto-pan is active during node drag |
//! | [`Flow::with_auto_pan_on_connect`] | Set whether auto-pan is active during connection drag |
//! | [`Flow::with_auto_pan_speed`] | Set the auto-pan speed |
//! | [`Flow::block`] | Set a block to wrap the canvas |
//! | [`Flow::set_block`] | Set the block at runtime |
//! | [`Flow::preview_style`] | Set the edge preview style |
//! | [`Flow::set_preview_style`] | Set the edge preview style at runtime |
//!
//! ## Graph — Queries
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`Flow::nodes`] | Iterate over all nodes |
//! | [`Flow::edges`] | Slice of all edges |
//! | [`Flow::node`] | Get node by ID |
//! | [`Flow::edge`] | Get edge by ID |
//! | [`Flow::node_bounds`] | Node's world-space bounds, parent offsets resolved |
//! | [`Flow::nodes_in`] | IDs of nodes intersecting a world-space area |
//! | [`Flow::pick`] | What sits at a world position, without acting on it |
//! | [`Flow::connection_exists`] | Check if an edge exists with the given source, target, and handles |
//!
//! ## Graph — Mutations
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`Flow::add_node`] | Add a node (validates ID and parent refs) |
//! | [`Flow::add_edge`] | Add an edge (validates refs, prevents self-loops) |
//! | [`Flow::add_edge_from_connection`] | Add edge from a completed connection |
//! | [`Flow::remove_node`] | Remove a node and its connected edges |
//! | [`Flow::remove_edge`] | Remove an edge by ID |
//! | [`Flow::clear`] | Remove all nodes and edges |
//! | [`Flow::set_node_position`] | Set a node's position |
//! | [`Flow::move_node`] | Move a node by a relative delta |
//! | [`Flow::set_node_dimensions`] | Set a node's width and height |
//! | [`Flow::set_node_z_index`] | Set a node's z-index for layering |
//! | [`Flow::set_node_hidden`] | Show or hide a node |
//! | [`Flow::set_node_selectable`] | Set whether a node can be selected |
//! | [`Flow::set_node_deletable`] | Set whether a node can be deleted |
//! | [`Flow::set_node_draggable`] | Set whether a node can be dragged |
//! | [`Flow::set_node_resizable`] | Set whether a node can be resized |
//! | [`Flow::set_node_connectable`] | Set whether a node's handles can connect |
//! | [`Flow::set_node_opaque`] | Set whether a node blocks content behind it |
//! | [`Flow::set_handle_styles`] | Set handle style for all handles on a node |
//! | [`Flow::set_handle_style`] | Set handle style for a single handle by ID |
//! | [`Flow::set_handle_disabled_styles`] | Set disabled style for all handles on a node |
//! | [`Flow::set_handle_disabled_style`] | Set disabled style for a single handle by ID |
//! | [`Flow::node_content_mut`] | Mutable access to a node's content |
//! | [`Flow::nodes_content_mut`] | Mutable access to every node's content, with its ID |
//! | [`Flow::set_edge_hidden`] | Show or hide an edge |
//! | [`Flow::set_edge_label`] | Set an edge's label |
//! | [`Flow::set_edge_selectable`] | Set whether an edge can be selected |
//! | [`Flow::set_edge_deletable`] | Set whether an edge can be deleted |
//! | [`Flow::set_edge_animated`] | Set whether an edge is animated |
//! | [`Flow::set_edge_reconnectable`] | Set an edge's reconnectable mode |
//! | [`Flow::edge_content_mut`] | Mutable access to an edge's content |
//! | [`Flow::edges_content_mut`] | Mutable access to every edge's content, with its ID |
//! | [`Flow::reconnect_edge`] | Reconnect an existing edge to new endpoints |
//! | [`Flow::set_nodes`] | Replace all nodes (validates IDs, parent refs, handles) |
//! | [`Flow::set_edges`] | Replace all edges (validates IDs, node refs, self-loops) |
//! | [`Flow::retain_nodes`] | Keep only nodes matching a predicate (drops their edges) |
//! | [`Flow::retain_edges`] | Keep only edges matching a predicate |
//! | [`Flow::set_node_parent`] | Set or clear a node's parent |
//! | [`Flow::set_handles_hidden`] | Show or hide all handles on a node |
//! | [`Flow::set_handle_hidden`] | Show or hide a single handle by ID |
//! | [`Flow::set_node_positions`] | Set many node positions at once, resolving the hierarchy once |
//!
//! ## Selection
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`Flow::has_selected_nodes`] | True if any node is selected |
//! | [`Flow::has_selected_edges`] | True if any edge is selected |
//! | [`Flow::first_selected_node_id`] | ID of the first selected node, if any |
//! | [`Flow::first_selected_edge_id`] | ID of the first selected edge, if any |
//! | [`Flow::clear_selection`] | Deselect all nodes and edges |
//! | [`Flow::select_all_nodes`] | Select every node |
//! | [`Flow::select_all_edges`] | Select every edge |
//! | [`Flow::select_node`] | Select a node (clears other selection) |
//! | [`Flow::toggle_node_selection`] | Toggle a node's selection (multi-select) |
//! | [`Flow::selected_nodes`] | Iterate over selected nodes |
//! | [`Flow::select_next_node`] | Select the next node in insertion order |
//! | [`Flow::select_prev_node`] | Select the previous node in insertion order |
//! | [`Flow::select_node_in_direction`] | Select nearest node in a spatial direction |
//! | [`Flow::remove_selected_nodes`] | Remove all selected nodes (respects `deletable`) |
//! | [`Flow::select_edge`] | Select an edge (clears other selection) |
//! | [`Flow::toggle_edge_selection`] | Toggle an edge's selection (multi-select) |
//! | [`Flow::selected_edges`] | Iterate over selected edges |
//! | [`Flow::select_next_edge`] | Select the next edge in order |
//! | [`Flow::select_prev_edge`] | Select the previous edge in order |
//! | [`Flow::remove_selected_edges`] | Remove all selected edges (respects `deletable`) |
//!
//! ## Viewport
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`Flow::pan`] | Pan the viewport by a delta |
//! | [`Flow::zoom_in`] | Zoom in by the default factor |
//! | [`Flow::zoom_out`] | Zoom out by the default factor |
//! | [`Flow::zoom_to`] | Zoom to a specific level |
//! | [`Flow::zoom_around`] | Zoom around a canvas position |
//! | [`Flow::reset_zoom`] | Reset zoom to 1.0 |
//! | [`Flow::center_on_selected`] | Center viewport on selected nodes |
//! | [`Flow::center_on`] | Center viewport on a world position |
//! | [`Flow::ensure_node_visible`] | Pan minimum amount to make a node visible |
//! | [`Flow::request_fit_view`] | Fit all nodes into view (applied during next render) |
//! | [`Flow::request_fit_view_with_options`] | Fit nodes with options (applied during next render) |
//! | [`Flow::canvas_size`] | Canvas size from the last render |
//! | [`Flow::node_terminal_rect`] | Node's terminal-space rect, for app-drawn overlays |
//! | [`Flow::is_in_bounds`] | True if terminal coordinates fall inside the canvas |
//!
//! ## Coordinate Transforms (on [`Viewport`] via `flow.viewport`)
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`Viewport::canvas_to_world`] | Convert a canvas position to world coordinates |
//! | [`Viewport::world_to_canvas`] | Convert a world position to canvas coordinates |
//! | [`Viewport::world_to_canvas_rect`] | Convert a world rect to canvas coordinates |
//!
//! ## Event Handling
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`Flow::apply`] | Apply a [`FlowAction`](crate::FlowAction) (returns [`EventResponse`](crate::EventResponse)) |
//! | [`Flow::handle_key_event`] | Handle a keyboard event with default bindings |
//! | [`Flow::handle_mouse_event`] | Handle a mouse event with default behavior |
//! | [`Flow::apply_controls_action`] | Apply a [`ControlsAction`](crate::ControlsAction) |
//! | [`Flow::handle_controls_key_event`] | Handle a key event with default controls bindings |
//! | `Flow::handle_wheel` | Zoom from a browser wheel event (`ratzilla` only) |
//!
//! ## Connection Validation
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`Flow::set_connection_validator`] | Set a custom connection validator |
//! | [`Flow::clear_connection_validator`] | Clear the connection validator |
//!
//! ## Animation & Interaction State
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`Flow::tick_animation`] | Advance animation clock |
//! | [`Flow::tick_auto_pan`] | Advance auto-pan during drag operations |
//! | [`Flow::start_edge_preview`] | Start an edge preview from a source handle |
//! | [`Flow::preview_to_handle`] | Point the edge preview at a specific handle |
//! | [`Flow::preview_to_node`] | Point the edge preview at a target node (closest handle) |
//! | [`Flow::cycle_to_handle`] | Cycle the target handle of the edge preview |
//! | [`Flow::cycle_from_handle`] | Cycle the source handle of the edge preview |
//! | [`Flow::complete_edge_preview`] | Complete the edge preview and return a Connection |
//! | [`Flow::edge_preview`] | Edge preview state, or `None` if inactive |
//! | [`Flow::clear_edge_preview`] | Clear the edge preview |
//! | [`Flow::is_dragging`] | True if a drag is in progress |
//! | [`Flow::toggle_lock`] | Toggle the interaction lock |
//! | [`Flow::canvas_area`] | Canvas area from the last render |
//!
//! ## Snapshots
//!
//! | Method | Description |
//! |--------|-------------|
//! | [`Flow::to_snapshot`] | Extract a snapshot of nodes, edges, and viewport |
//! | [`Flow::from_snapshot`] | Restore a flow from a snapshot |

mod animation;
mod auto_pan;
pub(crate) mod edge_preview;
pub use edge_preview::EdgePreview;
mod event_handlers;
pub mod flow_ops;
mod graph;
mod hierarchy;
mod mouse;
mod render_context;
pub(crate) mod selection;
pub use selection::Direction;
mod validation;
mod viewport;

use crate::content::{EdgeContent, NodeContent};
use crate::error::Error;
use crate::theme::Theme;
use crate::types::{
    ConnectionMode, Dimensions, Edge, FitViewOptions, HandleType, InternalNode, Node, Position,
    SelectionReveal, Viewport,
};
use crate::ui::{EdgePreviewStyle, StepEdge, TextContent};

use ratatui::layout::Rect;
use ratatui::widgets::Block;
use std::collections::HashMap;
use std::collections::HashSet;

use validation::ConnectionValidator;

pub(crate) use animation::DEFAULT_ANIMATION_SPEED_MS;
pub(crate) use auto_pan::DEFAULT_AUTO_PAN_SPEED;
pub(crate) use mouse::DragState;
pub use mouse::Pick;
pub(crate) use mouse::{
    DEFAULT_CONNECTION_RADIUS, DEFAULT_EDGE_HIT_THRESHOLD, DEFAULT_HANDLE_HIT_RADIUS,
    DEFAULT_NODE_DRAG_THRESHOLD, DEFAULT_RESIZE_HANDLE_RADIUS,
};
pub(crate) use render_context::RenderContext;
pub(crate) use viewport::{DEFAULT_MAX_ZOOM, DEFAULT_MIN_ZOOM};

/// Z-index boost applied to selected nodes when `elevate_nodes_on_select` is enabled.
pub(crate) const DEFAULT_SELECTED_NODE_Z: i32 = 1000;

/// Snapshot of a flow graph's nodes, edges, and viewport.
///
/// Use [`Flow::to_snapshot`] to extract a snapshot and
/// [`Flow::from_snapshot`] to restore one. Useful for undo/redo,
/// and with the `serde` feature, for save/restore to disk.
///
/// # Example
///
/// ```
/// # use rataflow::Flow;
/// # let mut flow: Flow = Flow::new();
/// // Save current flow for undo
/// let snapshot = flow.to_snapshot();
///
/// // ... user makes changes ...
///
/// // Restore previous flow
/// flow = Flow::from_snapshot(snapshot).unwrap();
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(bound(
        serialize = "N: serde::Serialize, E: serde::Serialize",
        deserialize = "N: serde::de::DeserializeOwned, E: serde::de::DeserializeOwned",
    ))
)]
pub struct FlowSnapshot<N: NodeContent = TextContent, E: EdgeContent = StepEdge> {
    /// The nodes in the graph.
    #[cfg_attr(feature = "serde", serde(default))]
    pub nodes: Vec<Node<N>>,
    /// The edges connecting nodes.
    #[cfg_attr(feature = "serde", serde(default))]
    pub edges: Vec<Edge<E>>,
    /// The viewport (pan/zoom).
    #[cfg_attr(feature = "serde", serde(default))]
    pub viewport: Viewport,
}

/// The core flow graph type — holds graph topology, viewport, selection,
/// interaction state, and renders directly via `impl Widget for &mut Flow`.
///
/// Generic over:
/// - `N`: The node content type (must implement [`NodeContent`])
/// - `E`: The edge content type (must implement [`EdgeContent`])
#[derive(Clone)]
pub struct Flow<N: NodeContent = TextContent, E: EdgeContent = StepEdge> {
    /// The nodes in the graph.
    ///
    /// Use [`nodes()`](Self::nodes), [`node()`](Self::node), [`add_node()`](Self::add_node),
    /// and [`remove_node()`](Self::remove_node) for access.
    pub(crate) nodes: Vec<InternalNode<N>>,
    /// The edges connecting nodes.
    ///
    /// Use [`edges()`](Self::edges), [`edge()`](Self::edge), [`add_edge()`](Self::add_edge),
    /// and [`remove_edge()`](Self::remove_edge) for access.
    pub(crate) edges: Vec<Edge<E>>,
    /// Node lookup for O(1) access by ID.
    pub(crate) node_lookup: HashMap<String, usize>,
    /// Edge lookup for O(1) access by ID.
    pub(crate) edge_lookup: HashMap<String, usize>,
    /// Current viewport (pan/zoom).
    pub viewport: Viewport,
    /// Minimum zoom level (default: 0.5).
    pub min_zoom: f64,
    /// Maximum zoom level (default: 2.0).
    pub max_zoom: f64,
    /// Radius for handle hit detection in world units (default: 1.5).
    pub handle_hit_radius: f64,
    /// Threshold for edge hit detection in world units (default: 1.5).
    pub edge_hit_threshold: f64,
    /// Radius for finding target handles when creating connections (default: 2.0).
    pub connection_radius: f64,
    /// Distance threshold before node drag is initiated (default: 2.0 world units).
    /// Movements smaller than this are treated as clicks, not drags.
    pub node_drag_threshold: f64,
    /// Distance from a corner within which a drag starts a resize (default: 1.0
    /// world unit). Only applies to nodes with [`Node::resizable`] set.
    pub resize_handle_radius: f64,
    /// Smallest a node may be resized to (default: 1x1 world units).
    pub min_node_size: Dimensions,
    /// Whether nodes are selected when drag starts (default: true).
    ///
    /// When `true`, a dragged node is selected at drag start (immediately if
    /// [`node_drag_threshold`](Self::node_drag_threshold) is 0, or when the threshold is
    /// exceeded). When `false`, dragging never selects — selection only happens on click
    /// (mouse-up without exceeding the drag threshold).
    ///
    pub select_nodes_on_drag: bool,
    /// Whether dragging an unselected node clears the existing selection (default: true).
    ///
    /// When `true`, starting a drag on an unselected node deselects all other nodes.
    /// When `false`, the existing selection is preserved — only the dragged node moves,
    /// selected nodes stay highlighted but stationary. Useful for apps with detail
    /// panels or inspectors that should persist across drag interactions.
    pub deselect_on_drag: bool,
    /// Whether a left-drag on empty canvas draws a selection box (default: false).
    ///
    /// The right button already carries both gestures — click for a context menu,
    /// drag for a box — but some terminals reserve right-click for themselves
    /// (Warp) and others swallow modifier-drags (xterm and derivatives use Shift to
    /// bypass mouse reporting). This flag is the way out: set it and the left
    /// button selects, which every terminal delivers.
    ///
    /// It is also how an app binds its own trigger. Nothing says the flag has to be
    /// static — flip it while a modifier or a mode is active and the gesture
    /// becomes whatever the app wants it to be.
    ///
    /// While `true`, left-drag on the pane selects rather than pans; pan with the
    /// keyboard, or turn it off again.
    pub selection_on_drag: bool,
    /// Whether clicking empty space clears the existing selection (default: true).
    ///
    /// When `true`, clicking on the canvas background deselects all nodes and edges.
    /// When `false`, selection is preserved — only explicit clicks on other nodes
    /// or programmatic calls change selection.
    pub deselect_on_pane_click: bool,
    /// Current drag operation state.
    ///
    /// Managed internally by mouse event handlers.
    pub(crate) drag_state: DragState,
    /// Edge preview state. Set via [`start_edge_preview`](Self::start_edge_preview).
    pub(crate) edge_preview: Option<EdgePreview>,
    /// Render context for coordinate transforms.
    /// Automatically updated during each render.
    pub(crate) render_context: RenderContext,
    /// Whether multi-select mode is active.
    ///
    /// When active, mouse clicks toggle selection without clearing others.
    pub multi_select_mode: bool,
    /// Whether a drag operation has deferred hierarchy recomputation.
    ///
    /// During node dragging, each mouse move updates the node position but skips
    /// `resolve_hierarchy()` to avoid redundant work when multiple drag events
    /// arrive between renders. Resolved at render time or on mouse_up.
    pub(crate) drag_hierarchy_pending: bool,
    /// Accumulated animation time in milliseconds.
    pub(crate) animation_elapsed_ms: u64,
    /// Animation speed in milliseconds per phase step (default: 120).
    pub animation_speed_ms: u64,
    /// Connection mode controlling which handle type combinations are allowed.
    pub connection_mode: ConnectionMode,
    /// Optional callback for validating connections.
    pub(crate) connection_validator: Option<ConnectionValidator>,
    /// Whether interaction mutations (select, drag, connect, delete) are locked.
    ///
    /// When locked, viewport operations (pan, zoom, fit view) still work.
    pub locked: bool,
    /// Whether edges are reconnectable by default.
    ///
    /// Individual edges can override this via [`Edge::reconnectable`](crate::Edge::reconnectable).
    /// When `true`, edges with [`Reconnectable::Inherit`](crate::Reconnectable) are reconnectable at both ends.
    /// When `false`, only edges with explicit [`Reconnectable::Both`](crate::Reconnectable),
    /// [`Reconnectable::Source`](crate::Reconnectable), or
    /// [`Reconnectable::Target`](crate::Reconnectable) are reconnectable.
    pub edges_reconnectable: bool,
    /// Whether the viewport auto-pans when dragging a node near the canvas edge.
    pub auto_pan_on_node_drag: bool,
    /// Whether the viewport auto-pans when dragging a connection near the canvas edge.
    pub auto_pan_on_connect: bool,
    /// Auto-pan speed in canvas cells per second (default: 30.0).
    pub auto_pan_speed: f64,
    /// What the viewport does when keyboard navigation changes the selection
    /// (default: [`SelectionReveal::EnsureVisible`]). See [`SelectionReveal`].
    pub selection_reveal: SelectionReveal,
    /// Last known mouse position in canvas space, updated during drag events.
    /// Used by [`tick_auto_pan`](Self::tick_auto_pan) to compute edge proximity.
    pub(crate) last_mouse_canvas_pos: Option<Position>,

    /// Color theme for all widget defaults.
    pub theme: Theme,
    /// Whether selected nodes are visually elevated above non-selected nodes.
    ///
    /// When enabled (default), selected nodes are rendered as if their
    /// z-index were 1000 higher.
    pub elevate_nodes_on_select: bool,
    /// Cached z-order indices for rendering and hit testing.
    ///
    /// Sorted by `(effective_z, insertion_index)` where effective_z includes
    /// selection elevation. Recomputed lazily via [`ensure_z_order`](Self::ensure_z_order).
    pub(crate) z_order_cache: Vec<usize>,
    /// Whether the z-order cache needs recomputation.
    pub(crate) z_order_dirty: bool,
    /// Selection snapshot taken at handler entry for diff check.
    /// `SelectionChanged` is only emitted when post-mutation state differs from this.
    pub(crate) prev_selection_node_ids: Vec<String>,
    pub(crate) prev_selection_edge_ids: Vec<String>,
    /// Pending fit-view request to be applied during the next render.
    /// Set via `request_fit_view()` / `request_fit_view_with_options()`.
    pub(crate) pending_fit: Option<FitViewOptions>,
    /// Canvas size when the pending fit was last applied.
    /// Used to detect canvas size stabilization — the fit is re-applied each
    /// render until the canvas size matches, then cleared.
    pub(crate) pending_fit_canvas: Option<(u16, u16)>,
    /// Optional block to wrap the canvas.
    pub(crate) block: Option<Block<'static>>,
    /// Optional override for edge preview style during connection creation.
    /// When `None`, derived from `theme` at render time.
    pub(crate) preview_style: Option<EdgePreviewStyle>,
}

impl<N: NodeContent, E: EdgeContent> std::fmt::Debug for Flow<N, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flow")
            .field("nodes", &self.nodes)
            .field("edges", &self.edges)
            .field("viewport", &self.viewport)
            .field("min_zoom", &self.min_zoom)
            .field("max_zoom", &self.max_zoom)
            .field("handle_hit_radius", &self.handle_hit_radius)
            .field("edge_hit_threshold", &self.edge_hit_threshold)
            .field("connection_radius", &self.connection_radius)
            .field("node_drag_threshold", &self.node_drag_threshold)
            .field("select_nodes_on_drag", &self.select_nodes_on_drag)
            .field("deselect_on_drag", &self.deselect_on_drag)
            .field("deselect_on_pane_click", &self.deselect_on_pane_click)
            .field("drag_state", &self.drag_state)
            .field("edge_preview", &self.edge_preview)
            .field("multi_select_mode", &self.multi_select_mode)
            .field("animation_speed_ms", &self.animation_speed_ms)
            .field("connection_mode", &self.connection_mode)
            .field("locked", &self.locked)
            .field("edges_reconnectable", &self.edges_reconnectable)
            .field("auto_pan_on_node_drag", &self.auto_pan_on_node_drag)
            .field("auto_pan_on_connect", &self.auto_pan_on_connect)
            .field("auto_pan_speed", &self.auto_pan_speed)
            .field("theme", &self.theme)
            .field(
                "connection_validator",
                &self.connection_validator.as_ref().map(|_| "<fn>"),
            )
            .field("elevate_nodes_on_select", &self.elevate_nodes_on_select)
            .field("prev_selection_node_ids", &self.prev_selection_node_ids)
            .field("prev_selection_edge_ids", &self.prev_selection_edge_ids)
            .field("pending_fit", &self.pending_fit.is_some())
            .finish_non_exhaustive()
    }
}

impl<N: NodeContent, E: EdgeContent> Default for Flow<N, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    /// Creates a new empty flow.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            node_lookup: HashMap::new(),
            edge_lookup: HashMap::new(),
            viewport: Viewport::default(),
            min_zoom: DEFAULT_MIN_ZOOM,
            max_zoom: DEFAULT_MAX_ZOOM,
            handle_hit_radius: DEFAULT_HANDLE_HIT_RADIUS,
            edge_hit_threshold: DEFAULT_EDGE_HIT_THRESHOLD,
            connection_radius: DEFAULT_CONNECTION_RADIUS,
            node_drag_threshold: DEFAULT_NODE_DRAG_THRESHOLD,
            resize_handle_radius: DEFAULT_RESIZE_HANDLE_RADIUS,
            min_node_size: Dimensions::new(1.0, 1.0),
            select_nodes_on_drag: true,
            deselect_on_drag: true,
            deselect_on_pane_click: true,
            selection_on_drag: false,
            multi_select_mode: false,
            drag_state: DragState::None,
            edge_preview: None,
            render_context: RenderContext::default(),
            drag_hierarchy_pending: false,
            animation_elapsed_ms: 0,
            animation_speed_ms: DEFAULT_ANIMATION_SPEED_MS,
            connection_mode: ConnectionMode::default(),
            connection_validator: None,
            locked: false,
            edges_reconnectable: true,
            auto_pan_on_node_drag: true,
            auto_pan_on_connect: true,
            auto_pan_speed: DEFAULT_AUTO_PAN_SPEED,
            selection_reveal: SelectionReveal::default(),
            last_mouse_canvas_pos: None,
            theme: Theme::default(),
            elevate_nodes_on_select: true,
            z_order_cache: Vec::new(),
            z_order_dirty: true,
            prev_selection_node_ids: Vec::new(),
            prev_selection_edge_ids: Vec::new(),
            pending_fit: None,
            pending_fit_canvas: None,
            block: None,
            preview_style: None,
        }
    }

    /// Sets a block to wrap the canvas.
    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the visual style for the edge preview during connection creation.
    ///
    /// When not set, derived from [`theme`](Self::theme) at render time.
    pub fn preview_style(mut self, style: EdgePreviewStyle) -> Self {
        self.preview_style = Some(style);
        self
    }

    /// Sets the block at runtime.
    pub fn set_block(&mut self, block: Option<Block<'static>>) {
        self.block = block;
    }

    /// Sets the edge preview style at runtime.
    pub fn set_preview_style(&mut self, style: Option<EdgePreviewStyle>) {
        self.preview_style = style;
    }

    /// Creates a flow with the given nodes and edges.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - An edge references a non-existent node
    /// - A node references a non-existent parent
    /// - A node ID is duplicated
    /// - An edge ID is duplicated
    /// - An edge has the same source and target node
    pub fn with_graph(nodes: Vec<Node<N>>, edges: Vec<Edge<E>>) -> Result<Self, Error> {
        let internal_nodes: Vec<InternalNode<N>> =
            nodes.into_iter().map(InternalNode::from_node).collect();

        Self::validate_graph(&internal_nodes, &edges)?;
        let node_lookup = Self::build_node_lookup(&internal_nodes);
        let edge_lookup = Self::build_edge_lookup(&edges);

        let mut state = Self {
            node_lookup,
            edge_lookup,
            nodes: internal_nodes,
            edges,
            ..Self::new()
        };

        state.snapshot_selection();
        state.resolve_hierarchy();
        Ok(state)
    }

    // ========== Builder Methods (sugar for pub fields) ==========

    /// Sets the minimum zoom level.
    pub fn with_min_zoom(mut self, min_zoom: f64) -> Self {
        self.min_zoom = min_zoom;
        self
    }

    /// Sets the maximum zoom level.
    pub fn with_max_zoom(mut self, max_zoom: f64) -> Self {
        self.max_zoom = max_zoom;
        self
    }

    /// Sets the handle hit detection radius in world units.
    pub fn with_handle_hit_radius(mut self, radius: f64) -> Self {
        self.handle_hit_radius = radius;
        self
    }

    /// Sets the edge hit detection threshold in world units.
    pub fn with_edge_hit_threshold(mut self, threshold: f64) -> Self {
        self.edge_hit_threshold = threshold;
        self
    }

    /// Sets the connection snap radius in world units.
    pub fn with_connection_radius(mut self, radius: f64) -> Self {
        self.connection_radius = radius;
        self
    }

    /// Sets the node drag threshold in world units.
    pub fn with_node_drag_threshold(mut self, threshold: f64) -> Self {
        self.node_drag_threshold = threshold;
        self
    }

    /// Sets whether nodes are selected when drag starts.
    ///
    /// When `true` (default), dragging a node selects it. When `false`, only clicking
    /// (releasing without exceeding the drag threshold) selects.
    pub fn with_select_nodes_on_drag(mut self, enabled: bool) -> Self {
        self.select_nodes_on_drag = enabled;
        self
    }

    /// Sets whether dragging an unselected node clears the existing selection.
    ///
    /// When `true` (default), other nodes are deselected when a drag starts on
    /// an unselected node. When `false`, the existing selection is preserved.
    pub fn with_deselect_on_drag(mut self, enabled: bool) -> Self {
        self.deselect_on_drag = enabled;
        self
    }

    /// Sets whether clicking empty space clears the existing selection.
    ///
    /// When `true` (default), clicking the canvas background deselects all.
    /// When `false`, selection is preserved until an explicit node/edge click.
    pub fn with_deselect_on_pane_click(mut self, enabled: bool) -> Self {
        self.deselect_on_pane_click = enabled;
        self
    }

    /// Sets whether a left-drag on empty canvas draws a selection box.
    ///
    /// See [`selection_on_drag`](Self::selection_on_drag).
    pub fn with_selection_on_drag(mut self, selection_on_drag: bool) -> Self {
        self.selection_on_drag = selection_on_drag;
        self
    }

    /// Sets the animation speed in milliseconds per phase step.
    pub fn with_animation_speed(mut self, speed_ms: u64) -> Self {
        self.animation_speed_ms = speed_ms;
        self
    }

    /// Sets the connection mode.
    pub fn with_connection_mode(mut self, mode: ConnectionMode) -> Self {
        self.connection_mode = mode;
        self
    }

    /// Sets the initial viewport (pan/zoom).
    pub fn with_viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = viewport;
        self
    }

    /// Sets whether multi-select mode is active.
    pub fn with_multi_select_mode(mut self, enabled: bool) -> Self {
        self.multi_select_mode = enabled;
        self
    }

    /// Sets whether interaction mutations are locked.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Sets whether edges are reconnectable by default.
    ///
    /// When `true` (default), edges with [`Reconnectable::Inherit`](crate::Reconnectable)
    /// are reconnectable at both ends. Individual edges can override via
    /// [`Edge::reconnectable`](crate::Edge::reconnectable).
    pub fn with_edges_reconnectable(mut self, reconnectable: bool) -> Self {
        self.edges_reconnectable = reconnectable;
        self
    }

    /// Sets whether the viewport auto-pans when dragging a node near the canvas edge.
    pub fn with_auto_pan_on_node_drag(mut self, enabled: bool) -> Self {
        self.auto_pan_on_node_drag = enabled;
        self
    }

    /// Sets what the viewport does when keyboard navigation changes the
    /// selection (the `Select*` actions).
    ///
    /// Default is [`SelectionReveal::EnsureVisible`]. Use [`SelectionReveal::None`]
    /// to drive the camera yourself (the selection still changes and the
    /// `SelectionChanged` event still fires — only the built-in camera move is
    /// suppressed), or [`SelectionReveal::Center`] to center the newly-selected
    /// node in one move. Mouse-click and programmatic selection are unaffected.
    pub fn with_selection_reveal(mut self, reveal: SelectionReveal) -> Self {
        self.selection_reveal = reveal;
        self
    }

    /// Sets whether the viewport auto-pans when dragging a connection near the canvas edge.
    pub fn with_auto_pan_on_connect(mut self, enabled: bool) -> Self {
        self.auto_pan_on_connect = enabled;
        self
    }

    /// Sets the auto-pan speed in canvas cells per second.
    pub fn with_auto_pan_speed(mut self, speed: f64) -> Self {
        self.auto_pan_speed = speed;
        self
    }

    /// Sets the color theme.
    ///
    /// All elements — built-in content types and library-rendered elements —
    /// resolve from `flow.theme` at render time.
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets whether selected nodes are elevated above non-selected nodes.
    ///
    /// When enabled (default), selected nodes are rendered as if their z-index
    /// were 1000 higher. Disable for full manual z-index control.
    pub fn with_elevate_nodes_on_select(mut self, elevate: bool) -> Self {
        self.elevate_nodes_on_select = elevate;
        self.z_order_dirty = true;
        self
    }

    /// Marks the z-order cache as needing recomputation.
    ///
    /// Called by any operation that changes z-ordering factors:
    /// node addition/removal, z-index changes, selection changes.
    pub(crate) fn invalidate_z_order(&mut self) {
        self.z_order_dirty = true;
    }

    /// Recomputes the z-order cache if dirty.
    ///
    /// Two-phase computation:
    /// 1. Compute base effective z for each node (z_index + selection elevation)
    /// 2. Propagate parent z to children — children always stay above their parent
    ///    (`parentZ >= childZ ? parentZ + 1 : childZ`)
    ///
    /// Sort key: `(effective_z, insertion_index)`.
    /// Uses `sort_unstable_by` — safe because insertion index makes ordering unique.
    pub(crate) fn ensure_z_order(&mut self) {
        if !self.z_order_dirty {
            return;
        }

        let elevate = self.elevate_nodes_on_select;
        let n = self.nodes.len();

        // Phase 1: compute base effective z for each node
        for node in &mut self.nodes {
            let base = node.node.z_index;
            let elevation = if elevate && node.node.selected {
                DEFAULT_SELECTED_NODE_Z
            } else {
                0
            };
            node.effective_z = base + elevation;
        }

        // Phase 2: ensure children are always above their parent.
        // Iterate until stable — handles arbitrary nesting depth.
        loop {
            let mut changed = false;
            for i in 0..n {
                if let Some(parent_id) = &self.nodes[i].node.parent_id
                    && let Some(&parent_idx) = self.node_lookup.get(parent_id.as_str())
                {
                    let parent_z = self.nodes[parent_idx].effective_z;
                    if parent_z >= self.nodes[i].effective_z {
                        self.nodes[i].effective_z = parent_z + 1;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }

        // Phase 3: sort by (effective_z, insertion_index)
        self.z_order_cache.clear();
        self.z_order_cache.extend(0..n);
        let nodes = &self.nodes;
        self.z_order_cache.sort_unstable_by(|&a, &b| {
            nodes[a]
                .effective_z
                .cmp(&nodes[b].effective_z)
                .then_with(|| a.cmp(&b))
        });
        self.z_order_dirty = false;
    }

    /// Returns the cached z-order indices.
    ///
    /// Must call [`ensure_z_order`](Self::ensure_z_order) before using this.
    /// Indices are sorted back-to-front (lowest z first).
    pub(crate) fn z_ordered_indices(&self) -> &[usize] {
        &self.z_order_cache
    }

    /// Validates that the graph is well-formed.
    ///
    /// Checks:
    /// - No duplicate node IDs
    /// - No duplicate edge IDs
    /// - All edges reference existing nodes
    /// - No self-referential edges
    /// - All parent references point to existing nodes
    fn validate_graph(nodes: &[InternalNode<N>], edges: &[Edge<E>]) -> Result<(), Error> {
        // Check for duplicate node IDs
        let mut node_ids: HashSet<&str> = HashSet::with_capacity(nodes.len());
        for node in nodes {
            if !node_ids.insert(node.id()) {
                return Err(Error::DuplicateNodeId {
                    node_id: node.id().to_string(),
                });
            }
        }

        // Check for duplicate edge IDs
        let mut edge_ids: HashSet<&str> = HashSet::with_capacity(edges.len());
        for edge in edges {
            if !edge_ids.insert(&edge.id) {
                return Err(Error::DuplicateEdgeId {
                    edge_id: edge.id.clone(),
                });
            }
        }

        // Validate edges reference existing nodes and aren't self-referential
        for edge in edges {
            if edge.source == edge.target {
                return Err(Error::SelfReferentialEdge {
                    edge_id: edge.id.clone(),
                    node_id: edge.source.clone(),
                });
            }
            if !node_ids.contains(edge.source.as_str()) {
                return Err(Error::InvalidEdgeReference {
                    edge_id: edge.id.clone(),
                    node_id: edge.source.clone(),
                });
            }
            if !node_ids.contains(edge.target.as_str()) {
                return Err(Error::InvalidEdgeReference {
                    edge_id: edge.id.clone(),
                    node_id: edge.target.clone(),
                });
            }
        }

        // Validate parent references and handles
        for node in nodes {
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

        Ok(())
    }

    /// Validates handle IDs for a node.
    ///
    /// Checks that multiple handles of the same type all have IDs (no ambiguity)
    /// and that no duplicate handle IDs exist within the same type.
    fn validate_handles(node_id: &str, handles: &[crate::types::Handle]) -> Result<(), Error> {
        for handle_type in [HandleType::Source, HandleType::Target] {
            let type_name = match handle_type {
                HandleType::Source => "source",
                HandleType::Target => "target",
            };

            let handles_of_type: Vec<_> = handles
                .iter()
                .filter(|h| h.handle_type == handle_type)
                .collect();

            if handles_of_type.len() <= 1 {
                continue;
            }

            // Multiple handles of this type — all must have IDs
            let none_count = handles_of_type.iter().filter(|h| h.id.is_none()).count();
            if none_count > 0 {
                return Err(Error::AmbiguousHandles {
                    node_id: node_id.to_string(),
                    handle_type: type_name,
                    count: none_count,
                });
            }

            // Check for duplicate IDs
            let mut seen = HashSet::new();
            for handle in &handles_of_type {
                if let Some(id) = &handle.id
                    && !seen.insert(id)
                {
                    return Err(Error::DuplicateHandleId {
                        node_id: node_id.to_string(),
                        handle_type: type_name,
                        handle_id: id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn build_node_lookup(nodes: &[InternalNode<N>]) -> HashMap<String, usize> {
        nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id().to_owned(), i))
            .collect()
    }

    fn build_edge_lookup(edges: &[Edge<E>]) -> HashMap<String, usize> {
        edges
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.clone(), i))
            .collect()
    }

    /// Sets all nodes to the width of the widest node.
    pub fn with_uniform_width(mut self) -> Self {
        if let Some(max_width) = self
            .nodes
            .iter()
            .map(|n| n.node.width)
            .max_by(f64::total_cmp)
        {
            for node in &mut self.nodes {
                node.node.width = max_width;
            }
            self.resolve_hierarchy();
        }
        self
    }

    /// Sets all nodes to the height of the tallest node.
    pub fn with_uniform_height(mut self) -> Self {
        if let Some(max_height) = self
            .nodes
            .iter()
            .map(|n| n.node.height)
            .max_by(f64::total_cmp)
        {
            for node in &mut self.nodes {
                node.node.height = max_height;
            }
            self.resolve_hierarchy();
        }
        self
    }

    /// Adds uniform padding to all nodes.
    pub fn with_uniform_padding(mut self, horizontal: f64, vertical: f64) -> Self {
        for node in &mut self.nodes {
            node.node.width += horizontal * 2.0;
            node.node.height += vertical * 2.0;
        }
        self.resolve_hierarchy();
        self
    }

    /// Returns the canvas area from the last render.
    pub fn canvas_area(&self) -> Rect {
        self.render_context.canvas_area
    }

    /// Returns `true` if a drag operation is in progress.
    pub fn is_dragging(&self) -> bool {
        self.drag_state.is_active()
    }

    /// Toggles the interaction lock.
    pub fn toggle_lock(&mut self) {
        self.locked = !self.locked;
    }

    /// Extracts a snapshot of the flow graph.
    ///
    /// Returns the nodes, edges, and viewport as a [`FlowSnapshot`].
    pub fn to_snapshot(&self) -> FlowSnapshot<N, E>
    where
        N: Clone,
        E: Clone,
    {
        FlowSnapshot {
            nodes: self.nodes().cloned().collect(),
            edges: self.edges.clone(),
            viewport: self.viewport,
        }
    }

    /// Restores a flow from a snapshot.
    ///
    /// Creates a new [`Flow`] from a [`FlowSnapshot`],
    /// validating graph integrity (duplicate IDs, invalid refs, self-loops).
    pub fn from_snapshot(snapshot: FlowSnapshot<N, E>) -> Result<Self, Error> {
        let mut state = Self::with_graph(snapshot.nodes, snapshot.edges)?;
        state.viewport = snapshot.viewport;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Node, Position};

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_round_trip() {
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
        ];
        let edges: Vec<Edge<StepEdge>> = vec![Edge::new("e1", "a", "b")];
        let viewport = Viewport::new(5.0, 10.0, 1.5);

        let mut state = Flow::with_graph(nodes, edges).unwrap();
        state.viewport = viewport;

        let snapshot = state.to_snapshot();
        let json = serde_json::to_string(&snapshot).unwrap();
        let restored_snapshot: FlowSnapshot<TextContent, StepEdge> =
            serde_json::from_str(&json).unwrap();
        let restored = Flow::from_snapshot(restored_snapshot).unwrap();

        // Verify nodes
        assert_eq!(restored.nodes().count(), 2);
        let node_a = restored.node("a").unwrap();
        assert_eq!(node_a.position, Position::new(0.0, 0.0));
        let node_b = restored.node("b").unwrap();
        assert_eq!(node_b.position, Position::new(20.0, 0.0));

        // Verify edges
        assert_eq!(restored.edges().len(), 1);
        assert_eq!(restored.edges()[0].source, "a");
        assert_eq!(restored.edges()[0].target, "b");

        // Verify viewport
        assert_eq!(restored.viewport.x, 5.0);
        assert_eq!(restored.viewport.y, 10.0);
        assert_eq!(restored.viewport.zoom, 1.5);
    }

    #[test]
    fn test_hierarchy_with_missing_parent() {
        let orphan = Node::new(
            "orphan",
            Position::new(50.0, 50.0),
            (20.0, 20.0),
            TextContent::from("Orphan"),
        )
        .with_parent("nonexistent");

        let result = Flow::<TextContent, StepEdge>::with_graph(vec![orphan], vec![]);

        assert!(matches!(result, Err(Error::InvalidParentReference { .. })));
    }

    #[test]
    fn test_invalid_edge_reference() {
        let node = Node::new(
            "node1",
            Position::new(0.0, 0.0),
            (10.0, 5.0),
            TextContent::from("Node 1"),
        );

        let edge: Edge<StepEdge> = Edge::new("e1", "node1", "nonexistent");

        let result = Flow::with_graph(vec![node], vec![edge]);

        assert!(matches!(result, Err(Error::InvalidEdgeReference { .. })));
    }

    #[test]
    fn test_remove_selected_removes_connected_edges() {
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

        let edges: Vec<Edge<StepEdge>> = vec![
            Edge::new("e1", "a", "b"),
            Edge::new("e2", "b", "c"),
            Edge::new("e3", "a", "c"),
        ];

        let mut state = Flow::with_graph(nodes, edges).unwrap();
        assert_eq!(state.nodes.len(), 3);
        assert_eq!(state.edges().len(), 3);

        state.select_node("b");
        let removed = state.remove_selected_nodes();

        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].id, "b");
        assert_eq!(state.nodes.len(), 2);
        assert_eq!(state.edges().len(), 1);
        assert_eq!(state.edges()[0].id, "e3");
        assert!(!state.has_selected_nodes() && !state.has_selected_edges());
    }
}
