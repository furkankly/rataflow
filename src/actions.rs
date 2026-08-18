//! Actions, events, and key bindings.
//!
//! Actions separate *what* to do from *how* it's triggered, making custom key bindings easy.
//! Each widget type has its own action enum:
//! - [`FlowAction`] — graph interaction (selection, panning, editing) → [`crate::Flow`]
//! - [`ControlsAction`] — viewport controls (zoom, fit, lock) → [`crate::Flow`]
//!
//! Default key binding functions map keys to actions:
//! - [`default_flow_key_binding`] — hjkl, arrows, Del, Esc, c, m
//! - [`default_controls_key_binding`] — +/-, 0, f, i
//!
//! For custom bindings, write your own function and call the widget's `apply` method.
//!
//! [`FlowEvent`] represents meaningful interactions (clicks, drags, state changes).
//! Returned via [`EventResponse`] from all handlers.

use crate::input::{KeyCode, KeyEvent};
use crate::types::{Connection, HandleType};

/// Semantic actions for flow graph interaction.
///
/// Most users don't need this type directly — [`Flow::handle_key_event`](crate::Flow::handle_key_event)
/// and [`Flow::handle_mouse_event`](crate::Flow::handle_mouse_event) use default bindings
/// and return events automatically.
///
/// `FlowAction` exists for **custom key bindings**: write your own `KeyEvent → FlowAction`
/// mapping function and pass the result to [`Flow::apply`](crate::Flow::apply).
///
/// ```no_run
/// # #![allow(unused)]
/// # use rataflow::{Flow, FlowAction, KeyCode, KeyEvent};
/// # let mut flow: Flow = Flow::new();
/// # let key = KeyEvent::new(KeyCode::Char('x'));
/// // Most users: default bindings, just works
/// flow.handle_key_event(key);
///
/// // Custom bindings: map your own keys to actions
/// fn my_bindings(key: &KeyEvent) -> Option<FlowAction> {
///     match key.code {
///         KeyCode::Char('x') => Some(FlowAction::Delete),
///         KeyCode::Char('n') => Some(FlowAction::SelectNext),
///         _ => None,
///     }
/// }
/// if let Some(action) = my_bindings(&key) {
///     flow.apply(action);
/// }
/// ```
///
/// For programmatic state changes that don't need events (setup, scripting, tests),
/// use direct methods like [`Flow::select_node`](crate::Flow::select_node),
/// [`Flow::pan`](crate::Flow::pan), etc.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum FlowAction {
    // === Selection ===
    /// Select the next node in insertion order
    SelectNext,
    /// Select the previous node in insertion order
    SelectPrev,
    /// Select the nearest node above the current selection
    SelectUp,
    /// Select the nearest node below the current selection
    SelectDown,
    /// Select the nearest node to the left of the current selection
    SelectLeft,
    /// Select the nearest node to the right of the current selection
    SelectRight,
    /// Clear the current selection
    ClearSelection,
    /// Toggle multi-select mode (clicks toggle selection without clearing)
    ToggleMultiSelect,

    // === Panning ===
    /// Pan the viewport left
    PanLeft,
    /// Pan the viewport right
    PanRight,
    /// Pan the viewport up
    PanUp,
    /// Pan the viewport down
    PanDown,
    /// Pan by a custom amount (camera perspective).
    ///
    /// Positive `dx` pans right (reveals the right side of the graph), positive
    /// `dy` pans down (reveals the bottom). Same camera-direction convention as
    /// [`PanLeft`](Self::PanLeft)/[`PanRight`](Self::PanRight)/[`PanUp`](Self::PanUp)/[`PanDown`](Self::PanDown).
    Pan {
        /// Horizontal pan amount (positive = pan right).
        dx: f64,
        /// Vertical pan amount (positive = pan down).
        dy: f64,
    },

    // === Editing ===
    /// Delete the currently selected node or edge
    Delete,

    // === Connection ===
    /// Cancel the current connection creation
    CancelConnection,

    // === View ===
    /// Center the viewport on the selected node
    CenterOnSelected,
}

/// Semantic actions for Controls widget interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ControlsAction {
    /// Zoom in (increase zoom level)
    ZoomIn,
    /// Zoom out (decrease zoom level)
    ZoomOut,
    /// Reset zoom to 1.0
    ResetZoom,
    /// Fit all nodes in view
    FitView,
    /// Toggle interactivity lock
    ToggleLock,
}

/// Events representing meaningful user interactions.
///
/// Returned via [`EventResponse::Event`] from handlers.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FlowEvent {
    // === Click interactions ===
    /// A node was clicked.
    NodeClicked {
        /// The clicked node's ID.
        node_id: String,
    },
    /// An edge was clicked.
    EdgeClicked {
        /// The clicked edge's ID.
        edge_id: String,
    },
    /// Empty canvas area was clicked.
    PaneClicked {
        /// Click position in world coordinates.
        x: f64,
        /// Click position in world coordinates.
        y: f64,
    },

    // === Context menu interactions ===
    /// A node was right-clicked.
    ///
    /// Selection is left alone, so a context menu opened on one node of a
    /// multi-selection does not collapse it. Call
    /// [`Flow::select_node`](crate::Flow::select_node) first if the menu should
    /// act on a single node.
    NodeContextMenu {
        /// The right-clicked node's ID.
        node_id: String,
    },
    /// An edge was right-clicked.
    ///
    /// Selection is left alone — see [`NodeContextMenu`](Self::NodeContextMenu).
    EdgeContextMenu {
        /// The right-clicked edge's ID.
        edge_id: String,
    },
    /// Empty canvas area was right-clicked.
    PaneContextMenu {
        /// Click position in world coordinates.
        x: f64,
        /// Click position in world coordinates.
        y: f64,
    },

    // === Connection lifecycle ===
    /// Started creating a connection by dragging from a handle.
    ConnectionStarted {
        /// The source node ID.
        node_id: String,
        /// The source handle ID. `None` means the only handle of this type on the node.
        handle_id: Option<String>,
    },
    /// A connection drag gesture completed successfully.
    ///
    /// The edge is **not** added automatically — call
    /// [`crate::Flow::add_edge_from_connection`] (or [`crate::Flow::add_edge`]) to
    /// add it to the graph.
    ConnectionCompleted(Connection),
    /// Connection creation was cancelled (released without valid target).
    ConnectionCancelled,

    // === Reconnection lifecycle ===
    /// Started reconnecting an edge by dragging from an endpoint handle.
    ReconnectionStarted {
        /// The edge being reconnected.
        edge_id: String,
        /// Which end is being dragged (`Source` or `Target`).
        handle_type: HandleType,
    },
    /// An edge reconnection completed successfully.
    ///
    /// The edge is **not** updated automatically — call
    /// [`crate::Flow::reconnect_edge`] to apply the new connection,
    /// or handle the update manually.
    ReconnectionCompleted {
        /// The edge that was reconnected.
        edge_id: String,
        /// The old connection (before reconnection).
        old_connection: Connection,
        /// The new connection (after reconnection).
        new_connection: Connection,
    },
    /// Edge reconnection was cancelled (released without valid target).
    ReconnectionCancelled {
        /// The edge whose reconnection was cancelled.
        edge_id: String,
    },

    // === Node drag lifecycle ===
    /// Started dragging a node.
    NodeDragStarted {
        /// The dragged node's ID.
        node_id: String,
    },
    /// A node is being dragged (ongoing movement after threshold).
    NodeDragged {
        /// The dragged node's ID.
        node_id: String,
    },
    /// Finished dragging a node.
    NodeDragEnded {
        /// The dragged node's ID.
        node_id: String,
    },

    // === Node resize lifecycle ===
    /// Started resizing a node by its bottom-right grip.
    NodeResizeStarted {
        /// The node being resized.
        node_id: String,
    },
    /// A node is being resized (ongoing).
    NodeResized {
        /// The node being resized.
        node_id: String,
    },
    /// Finished resizing a node.
    NodeResizeEnded {
        /// The node that was resized.
        node_id: String,
    },

    // === Viewport ===
    /// Viewport was changed (pan or zoom).
    ViewportChanged {
        /// Viewport X offset (pan).
        x: f64,
        /// Viewport Y offset (pan).
        y: f64,
        /// Zoom level.
        zoom: f64,
    },

    // === State change events ===
    /// Selected elements changed.
    ///
    /// Emitted alongside gesture events (e.g., `NodeClicked`, `EdgeClicked`, `PaneClicked`)
    /// when selection is affected. Handle this event to track selection in one place
    /// regardless of input source (keyboard or mouse).
    SelectionChanged {
        /// Currently selected node IDs (after the change).
        node_ids: Vec<String>,
        /// Currently selected edge IDs (after the change).
        edge_ids: Vec<String>,
    },
    /// Elements were deleted.
    ///
    /// Contains the IDs of explicitly selected elements that were removed.
    /// Edges cascade-removed due to node deletion are **not** included in `edge_ids`.
    Deleted {
        /// IDs of deleted nodes.
        node_ids: Vec<String>,
        /// IDs of deleted edges (explicitly selected only, not cascade-removed).
        edge_ids: Vec<String>,
    },
}

/// Response from action and input handlers.
///
/// Returned by [`crate::Flow::apply`], [`crate::Flow::handle_key_event`],
/// [`crate::Flow::handle_mouse_event`], [`crate::Flow::apply_controls_action`],
/// and [`crate::Flow::handle_controls_key_event`].
///
/// - `NotHandled` — input not consumed; fall through to next handler
/// - `Handled` — input consumed, no events produced
/// - `Event(Vec<FlowEvent>)` — input produced events the app may react to
///
/// # Example
///
/// ```no_run
/// # #![allow(unused)]
/// # use rataflow::{EventResponse, Flow, FlowEvent, KeyCode, KeyEvent};
/// # use rataflow::{MouseButton, MouseEvent, MouseEventKind, StepEdge};
/// # let mut flow: Flow = Flow::new();
/// # let key = KeyEvent::new(KeyCode::Char('f'));
/// # let mouse = MouseEvent::new(MouseEventKind::Down(MouseButton::Left), 0, 0);
/// // Controls-to-flow fallthrough pattern
/// let response = flow.handle_controls_key_event(key);
/// if matches!(response, EventResponse::NotHandled) {
///     flow.handle_key_event(key);
/// }
///
/// // Reacting to semantic events
/// for event in flow.handle_mouse_event(mouse).into_events() {
///     if let FlowEvent::ConnectionCompleted(conn) = event {
///         flow.add_edge_from_connection(conn, StepEdge::default());
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum EventResponse {
    /// Event was not consumed by this handler — fall through to next handler.
    NotHandled,
    /// Event was consumed but produced no semantic event.
    Handled,
    /// Event was consumed and produced one or more semantic events.
    Event(Vec<FlowEvent>),
}

impl EventResponse {
    /// Returns the events as a slice, or empty for non-event responses.
    pub fn events(&self) -> &[FlowEvent] {
        match self {
            EventResponse::Event(events) => events,
            _ => &[],
        }
    }

    /// Consumes self and returns an iterator over events.
    /// Returns an empty iterator for `NotHandled` and `Handled`.
    pub fn into_events(self) -> std::vec::IntoIter<FlowEvent> {
        match self {
            EventResponse::Event(events) => events.into_iter(),
            _ => Vec::new().into_iter(),
        }
    }
}

/// Default key bindings for flow graph actions.
///
/// Returns `Some(action)` if the key matches a default binding, `None` otherwise.
/// For viewport/controls bindings, see [`default_controls_key_binding`].
///
/// # Default Bindings
///
/// | Key | Action |
/// |-----|--------|
/// | `↑` / `↓` / `←` / `→` | Directional spatial navigation |
/// | `Tab` | Select next node (insertion order) |
/// | `Shift+Tab` | Select previous node (insertion order) |
/// | `h` | Pan left |
/// | `j` | Pan down |
/// | `k` | Pan up |
/// | `l` | Pan right |
/// | `Delete` / `Backspace` | Delete selected |
/// | `Escape` | Cancel connection |
/// | `c` | Center on selected |
/// | `m` | Toggle multi-select mode |
///
/// # Example
///
/// ```no_run
/// # #![allow(unused)]
/// # use rataflow::{Flow, KeyCode, KeyEvent};
/// use rataflow::{FlowAction, default_flow_key_binding};
///
/// # let mut flow: Flow = Flow::new();
/// # let key = KeyEvent::new(KeyCode::Char('n'));
/// if let Some(action) = default_flow_key_binding(&key) {
///     flow.apply(action);
/// }
/// ```
pub fn default_flow_key_binding(key: &KeyEvent) -> Option<FlowAction> {
    // Tab with shift = SelectPrev
    if key.code == KeyCode::Tab && key.modifiers.shift {
        return Some(FlowAction::SelectPrev);
    }

    // Tab without modifiers = SelectNext
    if key.code == KeyCode::Tab && !key.modifiers.any() {
        return Some(FlowAction::SelectNext);
    }

    // All other bindings require no modifiers
    if key.modifiers.any() {
        return None;
    }

    match key.code {
        // Directional spatial navigation
        KeyCode::Up => Some(FlowAction::SelectUp),
        KeyCode::Down => Some(FlowAction::SelectDown),
        KeyCode::Left => Some(FlowAction::SelectLeft),
        KeyCode::Right => Some(FlowAction::SelectRight),

        // Vim-style panning
        KeyCode::Char('h') => Some(FlowAction::PanLeft),
        KeyCode::Char('j') => Some(FlowAction::PanDown),
        KeyCode::Char('k') => Some(FlowAction::PanUp),
        KeyCode::Char('l') => Some(FlowAction::PanRight),

        // Editing
        KeyCode::Delete | KeyCode::Backspace => Some(FlowAction::Delete),

        // Connection
        KeyCode::Esc => Some(FlowAction::CancelConnection),

        // View
        KeyCode::Char('c') => Some(FlowAction::CenterOnSelected),
        KeyCode::Char('m') => Some(FlowAction::ToggleMultiSelect),

        _ => None,
    }
}

/// Default key bindings for Controls widget actions.
///
/// Returns `Some(action)` if the key matches a controls binding, `None` otherwise.
/// Handles viewport manipulation: zoom, fit, and lock.
///
/// # Default Bindings
///
/// | Key | Action |
/// |-----|--------|
/// | `+` / `=` | Zoom in |
/// | `-` / `_` | Zoom out |
/// | `0` | Reset zoom |
/// | `f` | Fit view |
/// | `i` | Toggle lock |
///
/// # Example
///
/// ```no_run
/// # #![allow(unused)]
/// # use rataflow::{Flow, KeyCode, KeyEvent};
/// use rataflow::{ControlsAction, default_controls_key_binding, default_flow_key_binding};
///
/// # let mut flow: Flow = Flow::new();
/// # let key = KeyEvent::new(KeyCode::Char('+'));
/// // Controls-first fallthrough pattern
/// if let Some(action) = default_controls_key_binding(&key) {
///     flow.apply_controls_action(action);
/// } else if let Some(action) = default_flow_key_binding(&key) {
///     flow.apply(action);
/// }
/// ```
pub fn default_controls_key_binding(key: &KeyEvent) -> Option<ControlsAction> {
    if key.modifiers.any() {
        return None;
    }

    match key.code {
        KeyCode::Char('+') | KeyCode::Char('=') => Some(ControlsAction::ZoomIn),
        KeyCode::Char('-') | KeyCode::Char('_') => Some(ControlsAction::ZoomOut),
        KeyCode::Char('0') => Some(ControlsAction::ResetZoom),
        KeyCode::Char('f') => Some(ControlsAction::FitView),
        KeyCode::Char('i') => Some(ControlsAction::ToggleLock),
        _ => None,
    }
}
