//! Event handling operations for Flow.
//!
//! Handlers return [`EventResponse`] for user code to react to interactions.
//!
//! - Keyboard: [`handle_key_event`](Flow::handle_key_event) uses default bindings;
//!   [`apply`](Flow::apply) takes a [`FlowAction`] for custom bindings.
//! - Mouse: [`handle_mouse_event`](Flow::handle_mouse_event) follows standard UX
//!   patterns (not remappable).
//! - Programmatic: direct methods (e.g., `select_node()`, `pan()`) mutate state
//!   without emitting events.
//!
//! For viewport controls (zoom, fit, lock), see [`Flow::apply_controls_action`] and
//! [`Flow::handle_controls_key_event`].
//!
//! # Example
//!
//! ```no_run
//! # use rataflow::{Flow, FlowEvent, MouseButton, MouseEvent, MouseEventKind, StepEdge};
//! # fn update_sidebar(_node_ids: &[String]) {}
//! # let mut flow: Flow = Flow::new();
//! # let mouse = MouseEvent::new(MouseEventKind::Down(MouseButton::Left), 0, 0);
//! for event in flow.handle_mouse_event(mouse).into_events() {
//!     match event {
//!         FlowEvent::NodeClicked { node_id } => println!("Clicked: {}", node_id),
//!         FlowEvent::ConnectionCompleted(conn) => {
//!             flow.add_edge_from_connection(conn, StepEdge::default());
//!         }
//!         FlowEvent::SelectionChanged { node_ids, .. } => update_sidebar(&node_ids),
//!         _ => {}
//!     }
//! }
//! ```

use crate::actions::{
    ControlsAction, EventResponse, FlowAction, FlowEvent, default_controls_key_binding,
    default_flow_key_binding,
};
use crate::content::{EdgeContent, NodeContent};
use crate::input::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::state::Flow;
use crate::state::mouse::DragState;
use crate::state::selection::Direction;

/// Default pan amount for keyboard panning (in world units).
pub(crate) const DEFAULT_PAN_AMOUNT: f64 = 5.0;

/// Default zoom factor for zoom in/out operations.
pub(crate) const DEFAULT_ZOOM_FACTOR: f64 = 1.2;

// Wheel-zoom normalization for the ratzilla/wasm path (see `Flow::handle_wheel`).
// Compiled for the `ratzilla` feature and for tests: the math is pure, so the
// fiddly cross-browser part is unit-tested off-wasm even though `handle_wheel`
// itself is wasm-only.
/// Exponential zoom rate per pixel of wheel scroll, for the ratzilla/wasm path
/// (see `Flow::handle_wheel`). Zoom is applied continuously and multiplicatively
/// (`factor = exp(-pixels * rate)`), so total zoom tracks total scroll distance
/// rather than the number of `wheel` events — immune to the trackpad event flood
/// with no threshold, and immediate like native browser zoom. Bigger = faster;
/// `ln(zoom_per_100px) / 100` gives the rate for a target zoom per ~100px detent
/// (`0.004` ≈ ×1.5, `0.007` ≈ ×2).
#[cfg(any(feature = "ratzilla", test))]
const WHEEL_ZOOM_RATE: f64 = 0.007;
/// `WheelEvent.deltaMode` line/page deltas → pixel scale factors.
#[cfg(any(feature = "ratzilla", test))]
const WHEEL_LINE_PX: f64 = 16.0;
#[cfg(any(feature = "ratzilla", test))]
const WHEEL_PAGE_PX: f64 = 800.0;

/// Continuous zoom factor for one browser wheel event: negative `delta_y` (scroll
/// up) → `factor > 1` (zoom in). Multiplicative across events, so a fixed gesture
/// zooms the same regardless of how many events the browser split it into.
///
/// Pure so the cross-browser normalization (Chrome pixel vs Firefox line mode) and
/// the flood-immunity property are unit-testable off-wasm.
#[cfg(any(feature = "ratzilla", test))]
fn wheel_zoom_factor(delta_y: f64, delta_mode: u32) -> f64 {
    let pixels = delta_y
        * match delta_mode {
            1 => WHEEL_LINE_PX,
            2 => WHEEL_PAGE_PX,
            _ => 1.0,
        };
    (-pixels * WHEEL_ZOOM_RATE).exp()
}

#[cfg(test)]
mod wheel_tests {
    use super::{WHEEL_LINE_PX, WHEEL_PAGE_PX, wheel_zoom_factor};

    #[test]
    fn zero_delta_is_identity() {
        assert_eq!(wheel_zoom_factor(0.0, 0), 1.0);
    }

    #[test]
    fn scroll_up_zooms_in_down_zooms_out() {
        assert!(wheel_zoom_factor(-100.0, 0) > 1.0); // up → zoom in
        assert!(wheel_zoom_factor(100.0, 0) < 1.0); // down → zoom out
    }

    #[test]
    fn multiplicative_so_event_count_does_not_matter() {
        // The whole point: one 100px event == two 50px == four 25px (flood immunity).
        let one = wheel_zoom_factor(-100.0, 0);
        let two = wheel_zoom_factor(-50.0, 0) * wheel_zoom_factor(-50.0, 0);
        let four: f64 = (0..4).map(|_| wheel_zoom_factor(-25.0, 0)).product();
        assert!((one - two).abs() < 1e-9);
        assert!((one - four).abs() < 1e-9);
    }

    #[test]
    fn line_mode_scales_by_line_px() {
        assert!(
            (wheel_zoom_factor(3.0, 1) - wheel_zoom_factor(3.0 * WHEEL_LINE_PX, 0)).abs() < 1e-12
        );
    }

    #[test]
    fn page_mode_scales_by_page_px() {
        assert!((wheel_zoom_factor(1.0, 2) - wheel_zoom_factor(WHEEL_PAGE_PX, 0)).abs() < 1e-12);
    }
}

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    /// Creates a `ViewportChanged` event response with current viewport state.
    pub(crate) fn viewport_changed_response(&self) -> EventResponse {
        EventResponse::Event(vec![FlowEvent::ViewportChanged {
            x: self.viewport.x,
            y: self.viewport.y,
            zoom: self.viewport.zoom,
        }])
    }

    /// **ratzilla (WebAssembly) only.** Zoom in response to a browser `wheel`
    /// event, around the cell under the pointer.
    ///
    /// ratzilla doesn't surface wheel events (they "need special handling"), so a
    /// wasm app installs its own `WheelEvent` listener and forwards each event
    /// here. Zoom is continuous and proportional to the scroll delta (unlike the
    /// terminal's discrete `ScrollUp`/`ScrollDown`), applied multiplicatively — so
    /// total zoom tracks total scroll distance regardless of how many `wheel`
    /// events the browser fired, and it responds immediately like native browser
    /// zoom. Terminals keep using [`handle_mouse_event`](Self::handle_mouse_event).
    ///
    /// `delta_mode` is the DOM `WheelEvent.deltaMode` (0 = pixel, 1 = line,
    /// 2 = page); `column`/`row` are the terminal cell to zoom around.
    ///
    /// ```ignore
    /// // in your `wheel` listener (wasm32 + `ratzilla` only, so not compiled here):
    /// let resp = flow.handle_wheel(e.delta_y(), e.delta_mode(), col, row);
    /// ```
    #[cfg(feature = "ratzilla")]
    pub fn handle_wheel(
        &mut self,
        delta_y: f64,
        delta_mode: u32,
        column: u16,
        row: u16,
    ) -> EventResponse {
        if delta_y == 0.0 {
            return EventResponse::NotHandled;
        }
        let factor = wheel_zoom_factor(delta_y, delta_mode);
        let canvas_pos = self.render_context.terminal_to_canvas(column, row);
        self.zoom_around(factor, canvas_pos);
        self.viewport_changed_response()
    }

    /// Snapshots the current selection into `prev_selection_*` fields.
    ///
    /// Called before selection-mutating paths so the snapshot reflects reality
    /// before mutations — including programmatic changes between handler calls.
    pub(crate) fn snapshot_selection(&mut self) {
        self.prev_selection_node_ids.clear();
        self.prev_selection_node_ids.extend(
            self.nodes
                .iter()
                .filter(|n| n.node.selected)
                .map(|n| n.node.id.clone()),
        );
        self.prev_selection_edge_ids.clear();
        self.prev_selection_edge_ids.extend(
            self.edges
                .iter()
                .filter(|e| e.selected)
                .map(|e| e.id.clone()),
        );
    }

    /// Returns a `SelectionChanged` event if selection differs from the
    /// snapshot taken at handler entry, `None` otherwise.
    pub(crate) fn maybe_selection_changed_event(&self) -> Option<FlowEvent> {
        if self.selection_matches_snapshot() {
            return None;
        }

        Some(FlowEvent::SelectionChanged {
            node_ids: self.selected_nodes().map(|n| n.id.clone()).collect(),
            edge_ids: self.selected_edges().map(|e| e.id.clone()).collect(),
        })
    }

    /// Returns `true` if current selection matches the snapshot.
    fn selection_matches_snapshot(&self) -> bool {
        self.nodes
            .iter()
            .filter(|n| n.node.selected)
            .map(|n| &n.node.id)
            .eq(self.prev_selection_node_ids.iter())
            && self
                .edges
                .iter()
                .filter(|e| e.selected)
                .map(|e| &e.id)
                .eq(self.prev_selection_edge_ids.iter())
    }

    /// Applies a flow action to the flow.
    ///
    /// This is the **custom key bindings** entry point. Most users should call
    /// [`handle_key_event`](Self::handle_key_event) instead, which uses default bindings
    /// and calls this method internally.
    ///
    /// Use `apply()` when you want to map your own keys to actions:
    ///
    /// ```no_run
    /// # #![allow(unused)]
    /// # use rataflow::{Flow, FlowAction, FlowEvent, KeyCode, KeyEvent};
    /// # let mut flow: Flow = Flow::new();
    /// # let key = KeyEvent::new(KeyCode::Char('x'));
    /// fn my_bindings(key: &KeyEvent) -> Option<FlowAction> {
    ///     match key.code {
    ///         KeyCode::Char('x') => Some(FlowAction::Delete),
    ///         _ => None,
    ///     }
    /// }
    ///
    /// if let Some(action) = my_bindings(&key) {
    ///     for event in flow.apply(action).into_events() {
    ///         match event {
    ///             FlowEvent::Deleted { .. } => { /* sync backend */ }
    ///             _ => {}
    ///         }
    ///     }
    /// }
    /// ```
    pub fn apply(&mut self, action: FlowAction) -> EventResponse {
        self.snapshot_selection();

        // When locked, block all flow actions (viewport ops go through ControlsAction)
        if self.locked {
            match action {
                FlowAction::PanLeft
                | FlowAction::PanRight
                | FlowAction::PanUp
                | FlowAction::PanDown
                | FlowAction::Pan { .. }
                | FlowAction::CenterOnSelected => {} // viewport panning allowed
                _ => return EventResponse::NotHandled,
            }
        }

        match action {
            // Selection
            FlowAction::SelectNext => {
                self.select_next_node();
                self.apply_selection_reveal();
                self.maybe_selection_changed_event()
                    .map_or(EventResponse::Handled, |e| EventResponse::Event(vec![e]))
            }
            FlowAction::SelectPrev => {
                self.select_prev_node();
                self.apply_selection_reveal();
                self.maybe_selection_changed_event()
                    .map_or(EventResponse::Handled, |e| EventResponse::Event(vec![e]))
            }
            FlowAction::SelectUp => {
                self.select_node_in_direction(Direction::Up);
                self.apply_selection_reveal();
                self.maybe_selection_changed_event()
                    .map_or(EventResponse::Handled, |e| EventResponse::Event(vec![e]))
            }
            FlowAction::SelectDown => {
                self.select_node_in_direction(Direction::Down);
                self.apply_selection_reveal();
                self.maybe_selection_changed_event()
                    .map_or(EventResponse::Handled, |e| EventResponse::Event(vec![e]))
            }
            FlowAction::SelectLeft => {
                self.select_node_in_direction(Direction::Left);
                self.apply_selection_reveal();
                self.maybe_selection_changed_event()
                    .map_or(EventResponse::Handled, |e| EventResponse::Event(vec![e]))
            }
            FlowAction::SelectRight => {
                self.select_node_in_direction(Direction::Right);
                self.apply_selection_reveal();
                self.maybe_selection_changed_event()
                    .map_or(EventResponse::Handled, |e| EventResponse::Event(vec![e]))
            }
            FlowAction::ClearSelection => {
                self.clear_selection();
                self.maybe_selection_changed_event()
                    .map_or(EventResponse::Handled, |e| EventResponse::Event(vec![e]))
            }
            FlowAction::ToggleMultiSelect => {
                self.multi_select_mode = !self.multi_select_mode;
                EventResponse::Handled
            }

            // Panning
            FlowAction::PanLeft => {
                self.pan(DEFAULT_PAN_AMOUNT, 0.0);
                self.viewport_changed_response()
            }
            FlowAction::PanRight => {
                self.pan(-DEFAULT_PAN_AMOUNT, 0.0);
                self.viewport_changed_response()
            }
            FlowAction::PanUp => {
                self.pan(0.0, DEFAULT_PAN_AMOUNT);
                self.viewport_changed_response()
            }
            FlowAction::PanDown => {
                self.pan(0.0, -DEFAULT_PAN_AMOUNT);
                self.viewport_changed_response()
            }
            FlowAction::Pan { dx, dy } => {
                // FlowAction uses camera perspective; pan() uses content-offset.
                // Camera-right = content-left, so negate.
                self.pan(-dx, -dy);
                self.viewport_changed_response()
            }

            // Editing
            FlowAction::Delete => {
                let removed_nodes = self.remove_selected_nodes();
                let removed_edges = self.remove_selected_edges();
                if removed_nodes.is_empty() && removed_edges.is_empty() {
                    EventResponse::Handled
                } else {
                    let node_ids = removed_nodes.into_iter().map(|n| n.id).collect();
                    let edge_ids = removed_edges.into_iter().map(|e| e.id).collect();
                    let mut events = vec![FlowEvent::Deleted { node_ids, edge_ids }];
                    if let Some(sel) = self.maybe_selection_changed_event() {
                        events.push(sel);
                    }
                    EventResponse::Event(events)
                }
            }

            // Connection
            FlowAction::CancelConnection => {
                let was_creating = matches!(self.drag_state, DragState::CreatingConnection);
                let reconnecting_edge_id =
                    if let DragState::ReconnectingEdge { ref edge_id } = self.drag_state {
                        Some(edge_id.clone())
                    } else {
                        None
                    };
                let had_preview = self.edge_preview.is_some();
                if matches!(
                    self.drag_state,
                    DragState::CreatingConnection | DragState::ReconnectingEdge { .. }
                ) {
                    self.drag_state = DragState::None;
                }
                self.edge_preview = None;
                if let Some(edge_id) = reconnecting_edge_id {
                    EventResponse::Event(vec![FlowEvent::ReconnectionCancelled { edge_id }])
                } else if was_creating || had_preview {
                    EventResponse::Event(vec![FlowEvent::ConnectionCancelled])
                } else {
                    EventResponse::Handled
                }
            }

            // View
            FlowAction::CenterOnSelected => {
                self.center_on_selected();
                self.viewport_changed_response()
            }
        }
    }

    /// Applies the configured [`SelectionReveal`](crate::SelectionReveal) after a
    /// keyboard-nav selection change (the `Select*` actions). Mouse-click and
    /// programmatic selection do not reveal.
    fn apply_selection_reveal(&mut self) {
        match self.selection_reveal {
            crate::SelectionReveal::None => {}
            crate::SelectionReveal::EnsureVisible => self.ensure_selected_node_visible(),
            crate::SelectionReveal::Center => self.center_on_selected(),
        }
    }

    /// Handles a keyboard event with default flow bindings.
    ///
    /// For custom bindings, use [`Self::apply`] with your own binding function.
    ///
    /// # Default Bindings
    ///
    /// | Key | Action |
    /// |-----|--------|
    /// | `↑` / `↓` / `←` / `→` | Directional spatial navigation |
    /// | `Tab` / `Shift+Tab` | Select next/prev node (insertion order) |
    /// | `hjkl` | Pan viewport |
    /// | `Del` / `Backspace` | Delete selected |
    /// | `Esc` | Cancel connection |
    /// | `c` | Center on selected |
    /// | `m` | Toggle multi-select |
    ///
    /// For viewport controls (zoom, fit, lock), see [`Flow::handle_controls_key_event`].
    ///
    /// # Events
    ///
    /// - `SelectionChanged` — arrow/tab navigation
    /// - `ViewportChanged` — panning or centering
    /// - `Deleted` — delete key on selection
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rataflow::{Flow, FlowEvent, KeyCode, KeyEvent};
    /// # let mut flow: Flow = Flow::new();
    /// # let key = KeyEvent::new(KeyCode::Delete);
    /// for event in flow.handle_key_event(key).into_events() {
    ///     match event {
    ///         FlowEvent::SelectionChanged { .. } => { /* update sidebar */ }
    ///         FlowEvent::Deleted { .. } => { /* sync external state */ }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn handle_key_event(&mut self, key: impl Into<KeyEvent>) -> EventResponse {
        let key = key.into();
        if let Some(action) = default_flow_key_binding(&key) {
            self.apply(action)
        } else {
            EventResponse::NotHandled
        }
    }

    /// Handles a mouse event.
    ///
    /// Mouse interactions follow standard UX patterns (not remappable):
    ///
    /// | Input | Action |
    /// |-------|--------|
    /// | Left click | Select node/edge, start drag |
    /// | Drag | Move node or pan viewport |
    /// | Release | Complete operation |
    /// | Scroll | Zoom at cursor |
    ///
    /// # Events
    ///
    /// - `NodeClicked` / `EdgeClicked` / `PaneClicked` — gesture events on release
    /// - `SelectionChanged` — click to select
    /// - `NodeDragStarted` / `NodeMoved` — node dragging
    /// - `ViewportChanged` — panning or zooming
    /// - `ConnectionCompleted` — handle drag to target (app must call `add_edge_from_connection`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #![allow(unused)]
    /// # use rataflow::{Flow, FlowEvent, MouseButton, MouseEvent, MouseEventKind, StepEdge};
    /// # let mut flow: Flow = Flow::new();
    /// # let mouse = MouseEvent::new(MouseEventKind::Down(MouseButton::Left), 0, 0);
    /// for event in flow.handle_mouse_event(mouse).into_events() {
    ///     match event {
    ///         FlowEvent::NodeClicked { node_id } => { /* show details */ }
    ///         FlowEvent::ConnectionCompleted(conn) => {
    ///             flow.add_edge_from_connection(conn, StepEdge::default());
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    ///
    /// See the README for the recommended event loop pattern.
    pub fn handle_mouse_event(&mut self, mouse: impl Into<MouseEvent>) -> EventResponse {
        let mouse = mouse.into();
        // Convert terminal coordinates to world/canvas coordinates
        let world_pos =
            self.render_context
                .terminal_to_world(&self.viewport, mouse.column, mouse.row);
        let canvas_pos = self
            .render_context
            .terminal_to_canvas(mouse.column, mouse.row);

        // Ensure z-order is up-to-date before hit testing
        self.ensure_z_order();

        // When locked, left-click starts panning directly (no hit testing, no selection clearing)
        if self.locked {
            return match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    self.drag_state = DragState::Panning {
                        anchor_canvas: canvas_pos,
                        initial_viewport: self.viewport.offset(),
                    };
                    EventResponse::Handled
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    self.on_mouse_drag(world_pos, canvas_pos)
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    self.drag_state = DragState::None;
                    EventResponse::Handled
                }
                MouseEventKind::Down(MouseButton::Right) => self.on_right_down(world_pos),
                MouseEventKind::Drag(MouseButton::Right) => self.on_right_drag(world_pos),
                MouseEventKind::Up(MouseButton::Right) => self.on_right_up(),
                MouseEventKind::ScrollUp => {
                    self.zoom_around(DEFAULT_ZOOM_FACTOR, canvas_pos);
                    self.viewport_changed_response()
                }
                MouseEventKind::ScrollDown => {
                    self.zoom_around(1.0 / DEFAULT_ZOOM_FACTOR, canvas_pos);
                    self.viewport_changed_response()
                }
                _ => EventResponse::NotHandled,
            };
        }

        // Snapshot selection before any mouse event — deferred selection can
        // mutate selection in drag (threshold exceeded) or up (click).
        self.snapshot_selection();

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some(response) = self.try_start_resize(world_pos) {
                    return response;
                }
                if let Some(response) = self.try_start_selection_box(world_pos) {
                    return response;
                }
                let multi_select = self.multi_select_mode;
                self.on_mouse_down(world_pos, canvas_pos, true, multi_select)
            }
            MouseEventKind::Down(MouseButton::Right) => self.on_right_down(world_pos),
            MouseEventKind::Drag(MouseButton::Right) => self.on_right_drag(world_pos),
            MouseEventKind::Up(MouseButton::Right) => self.on_right_up(),
            MouseEventKind::Drag(MouseButton::Left) => self.on_mouse_drag(world_pos, canvas_pos),
            MouseEventKind::Up(MouseButton::Left) => self.on_mouse_up(world_pos),
            MouseEventKind::ScrollUp => {
                self.zoom_around(DEFAULT_ZOOM_FACTOR, canvas_pos);
                self.viewport_changed_response()
            }
            MouseEventKind::ScrollDown => {
                self.zoom_around(1.0 / DEFAULT_ZOOM_FACTOR, canvas_pos);
                self.viewport_changed_response()
            }
            _ => EventResponse::NotHandled,
        }
    }

    /// Applies a controls action.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rataflow::{Flow, FlowEvent, KeyCode, KeyEvent, default_controls_key_binding};
    /// # let mut flow: Flow = Flow::new();
    /// # let key = KeyEvent::new(KeyCode::Char('+'));
    /// if let Some(action) = default_controls_key_binding(&key) {
    ///     for event in flow.apply_controls_action(action).into_events() {
    ///         match event {
    ///             FlowEvent::ViewportChanged { .. } => { /* ... */ }
    ///             _ => {}
    ///         }
    ///     }
    /// }
    /// ```
    pub fn apply_controls_action(&mut self, action: ControlsAction) -> EventResponse {
        match action {
            ControlsAction::ZoomIn => {
                self.zoom_in();
                self.viewport_changed_response()
            }
            ControlsAction::ZoomOut => {
                self.zoom_out();
                self.viewport_changed_response()
            }
            ControlsAction::ResetZoom => {
                self.reset_zoom();
                self.viewport_changed_response()
            }
            ControlsAction::FitView => {
                self.request_fit_view();
                self.viewport_changed_response()
            }
            ControlsAction::ToggleLock => {
                self.toggle_lock();
                EventResponse::Handled
            }
        }
    }

    /// Handles a keyboard event with default controls bindings.
    ///
    /// For custom bindings, use [`Self::apply_controls_action`] with your own binding function.
    ///
    /// # Default Bindings
    ///
    /// | Key | Action |
    /// |-----|--------|
    /// | `+` / `-` | Zoom in/out |
    /// | `0` | Reset zoom |
    /// | `f` | Fit view |
    /// | `i` | Toggle lock |
    ///
    /// # Events
    ///
    /// - `ViewportChanged` — zoom or fit view
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rataflow::{EventResponse, Flow, KeyCode, KeyEvent};
    /// # let mut flow: Flow = Flow::new();
    /// # let key = KeyEvent::new(KeyCode::Char('f'));
    /// let response = flow.handle_controls_key_event(key);
    /// if matches!(response, EventResponse::NotHandled) {
    ///     flow.handle_key_event(key);
    /// }
    /// ```
    pub fn handle_controls_key_event(&mut self, event: impl Into<KeyEvent>) -> EventResponse {
        let event = event.into();
        if let Some(action) = default_controls_key_binding(&event) {
            self.apply_controls_action(action)
        } else {
            EventResponse::NotHandled
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::EventResponse;
    use crate::types::{Node, Position};
    use crate::ui::TextContent;

    fn make_locked_state() -> Flow {
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
        let mut state = Flow::with_graph(nodes, vec![]).unwrap();
        state.locked = true;
        state
    }

    #[test]
    fn test_locked_mutation_actions_return_not_handled() {
        let mut state = make_locked_state();

        assert_eq!(
            state.apply(FlowAction::SelectNext),
            EventResponse::NotHandled
        );
        assert_eq!(
            state.apply(FlowAction::SelectPrev),
            EventResponse::NotHandled
        );
        assert_eq!(state.apply(FlowAction::SelectUp), EventResponse::NotHandled);
        assert_eq!(
            state.apply(FlowAction::SelectDown),
            EventResponse::NotHandled
        );
        assert_eq!(
            state.apply(FlowAction::SelectLeft),
            EventResponse::NotHandled
        );
        assert_eq!(
            state.apply(FlowAction::SelectRight),
            EventResponse::NotHandled
        );
        assert_eq!(
            state.apply(FlowAction::ClearSelection),
            EventResponse::NotHandled
        );
        assert_eq!(
            state.apply(FlowAction::ToggleMultiSelect),
            EventResponse::NotHandled
        );
        assert_eq!(state.apply(FlowAction::Delete), EventResponse::NotHandled);
        assert_eq!(
            state.apply(FlowAction::CancelConnection),
            EventResponse::NotHandled
        );
    }

    #[test]
    fn test_locked_viewport_actions_still_work() {
        let mut state = make_locked_state();

        // Panning allowed when locked
        assert!(matches!(
            state.apply(FlowAction::PanLeft).events(),
            [FlowEvent::ViewportChanged { .. }]
        ));
        assert!(matches!(
            state.apply(FlowAction::PanRight).events(),
            [FlowEvent::ViewportChanged { .. }]
        ));

        // Center on selected allowed when locked
        assert!(matches!(
            state.apply(FlowAction::CenterOnSelected).events(),
            [FlowEvent::ViewportChanged { .. }]
        ));

        // Zoom/fit actions go through apply_controls_action (tested separately)
    }

    #[test]
    fn test_locked_mouse_down_starts_panning() {
        let mut state = make_locked_state();
        // Set render context so coordinate transforms work
        state.render_context.canvas_area = ratatui::layout::Rect::new(0, 0, 80, 24);

        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 3,
            modifiers: crate::input::Modifiers::default(),
        };
        let event = state.handle_mouse_event(mouse);

        // Should start panning, not select a node
        assert_eq!(event, EventResponse::Handled);
        assert!(matches!(state.drag_state, DragState::Panning { .. }));
    }

    fn make_test_state() -> Flow {
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
        Flow::with_graph(nodes, vec![]).unwrap()
    }

    fn make_test_state_with_edges() -> Flow {
        use crate::types::Edge;
        use crate::ui::StepEdge;

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
        Flow::with_graph(nodes, edges).unwrap()
    }

    #[test]
    fn test_delete_selected_nodes_emits_deleted_and_selection_changed() {
        let mut state = make_test_state();
        state.select_node("a");
        let events: Vec<_> = state.apply(FlowAction::Delete).into_events().collect();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            FlowEvent::Deleted {
                node_ids: vec!["a".into()],
                edge_ids: vec![],
            }
        );
        assert_eq!(
            events[1],
            FlowEvent::SelectionChanged {
                node_ids: vec![],
                edge_ids: vec![],
            }
        );
    }

    #[test]
    fn test_delete_selected_edges_emits_deleted_and_selection_changed() {
        let mut state = make_test_state_with_edges();
        state.select_edge("e1");
        let events: Vec<_> = state.apply(FlowAction::Delete).into_events().collect();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            FlowEvent::Deleted {
                node_ids: vec![],
                edge_ids: vec!["e1".into()],
            }
        );
        assert!(matches!(events[1], FlowEvent::SelectionChanged { .. }));
    }

    #[test]
    fn test_delete_both_nodes_and_edges_emits_deleted() {
        use crate::types::Edge;
        use crate::ui::StepEdge;

        // Two edges so we can select one independently
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
        let edges: Vec<Edge<StepEdge>> = vec![Edge::new("e1", "a", "b"), Edge::new("e2", "b", "c")];
        let mut state: Flow = Flow::with_graph(nodes, edges).unwrap();
        // Select node "a" and edge "e2" (not connected to "a", so not cascade-removed)
        state.toggle_node_selection("a");
        state.toggle_edge_selection("e2");
        let events: Vec<_> = state.apply(FlowAction::Delete).into_events().collect();
        assert!(events.len() >= 2);
        match &events[0] {
            FlowEvent::Deleted { node_ids, edge_ids } => {
                assert_eq!(node_ids, &vec!["a".to_string()]);
                assert_eq!(edge_ids, &vec!["e2".to_string()]);
            }
            other => panic!("Expected Deleted event, got {:?}", other),
        }
        assert!(matches!(events[1], FlowEvent::SelectionChanged { .. }));
    }

    #[test]
    fn test_delete_nothing_selected_returns_handled() {
        let mut state = make_test_state();
        let response = state.apply(FlowAction::Delete);
        assert_eq!(response, EventResponse::Handled);
    }

    #[test]
    fn test_select_next_emits_selection_changed() {
        let mut state = make_test_state();
        let events: Vec<_> = state.apply(FlowAction::SelectNext).into_events().collect();
        assert_eq!(
            events,
            vec![FlowEvent::SelectionChanged {
                node_ids: vec!["a".into()],
                edge_ids: vec![],
            }]
        );
    }

    #[test]
    fn test_select_prev_emits_selection_changed() {
        let mut state = make_test_state();
        let events: Vec<_> = state.apply(FlowAction::SelectPrev).into_events().collect();
        assert_eq!(
            events,
            vec![FlowEvent::SelectionChanged {
                node_ids: vec!["b".into()],
                edge_ids: vec![],
            }]
        );
    }

    #[test]
    fn test_clear_selection_emits_selection_changed() {
        let mut state = make_test_state();
        state.select_node("a");
        let events: Vec<_> = state
            .apply(FlowAction::ClearSelection)
            .into_events()
            .collect();
        assert_eq!(
            events,
            vec![FlowEvent::SelectionChanged {
                node_ids: vec![],
                edge_ids: vec![],
            }]
        );
    }

    #[test]
    fn test_cancel_connection_when_creating_emits_connection_cancelled() {
        let mut state = make_test_state();
        state.drag_state = DragState::CreatingConnection;
        state.set_edge_preview_raw(
            "a".to_string(),
            Some("src".to_string()),
            crate::types::HandleType::Source,
            Position::new(0.0, 0.0),
        );
        let events: Vec<_> = state
            .apply(FlowAction::CancelConnection)
            .into_events()
            .collect();
        assert_eq!(events, vec![FlowEvent::ConnectionCancelled]);
    }

    #[test]
    fn test_cancel_connection_when_idle_returns_handled() {
        let mut state = make_test_state();
        let response = state.apply(FlowAction::CancelConnection);
        assert_eq!(response, EventResponse::Handled);
    }

    #[test]
    fn test_selection_changed_suppressed_when_unchanged() {
        let mut state = make_test_state();
        // Select "a" via event system
        let events: Vec<_> = state.apply(FlowAction::SelectNext).into_events().collect();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], FlowEvent::SelectionChanged { .. }));

        // SelectNext again wraps to "b" — should emit
        let response = state.apply(FlowAction::SelectNext);
        assert!(matches!(response, EventResponse::Event(_)));

        // Clear selection — should emit (going from ["b"] to [])
        let response = state.apply(FlowAction::ClearSelection);
        assert!(matches!(response, EventResponse::Event(_)));

        // Clear again — already empty, should NOT emit
        let response = state.apply(FlowAction::ClearSelection);
        assert_eq!(response, EventResponse::Handled);
    }

    #[test]
    fn test_select_next_single_node_no_change() {
        let nodes = vec![Node::new(
            "only",
            Position::new(0.0, 0.0),
            (10.0, 5.0),
            TextContent::from("Only"),
        )];
        let mut state: Flow = Flow::with_graph(nodes, vec![]).unwrap();

        // First SelectNext selects "only" — emits
        let response = state.apply(FlowAction::SelectNext);
        assert!(matches!(response, EventResponse::Event(_)));

        // Second SelectNext wraps to same node — suppressed
        let response = state.apply(FlowAction::SelectNext);
        assert_eq!(response, EventResponse::Handled);
    }
}
