//! Demo wrapper structs for all website examples.
//!
//! Standard demos use `DemoApp` directly via its blanket `Demo` impl.
//! Custom demos hold a `DemoApp` internally and override specific methods.

use std::collections::VecDeque;
use std::time::Duration;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use rataflow::{
    default_controls_key_binding, default_flow_key_binding, Background, BackgroundStyle,
    BackgroundVariant, ConnectionMode, Controls, ControlsOrientation, ControlsPosition,
    ControlsStyle, EventResponse, FitViewOptions, Flow, FlowAction, FlowEvent, FlowOps, KeyCode,
    MiniMap, MiniMapPosition, MiniMapStyle, Node, Position, StepEdge, TextContent, Theme,
};
use rataflow_examples::{
    context_menu::{run, target_at, Menu, Target},
    custom_controls_bindings, custom_flow_bindings,
    floating_edges::{Attach, DemoEdge},
    render_indicator, render_status,
    save_restore::{pretty_json, restore, save},
    theming::{apply_theme, next_theme, theme_name},
    History,
};

use crate::demo::{Demo, DemoApp};
use crate::DemoEntry;

// ============================================================================
// Entry constructors (return DemoEntry with metadata)
// ============================================================================

pub fn entry_basic() -> DemoEntry {
    DemoEntry {
        demo: Box::new(new_basic()),
        meta: rataflow_examples::meta::basic(),
    }
}

pub fn entry_view_only() -> DemoEntry {
    DemoEntry {
        demo: Box::new(ViewOnlyDemo::new()),
        meta: rataflow_examples::meta::view_only(),
    }
}

pub fn entry_custom_nodes() -> DemoEntry {
    DemoEntry {
        demo: Box::new(new_custom_nodes()),
        meta: rataflow_examples::meta::custom_nodes(),
    }
}

pub fn entry_node_flags() -> DemoEntry {
    DemoEntry {
        demo: Box::new(NodeFlagsDemo::new()),
        meta: rataflow_examples::meta::node_flags(),
    }
}

pub fn entry_hierarchy() -> DemoEntry {
    DemoEntry {
        demo: Box::new(new_hierarchy()),
        meta: rataflow_examples::meta::hierarchy(),
    }
}

pub fn entry_custom_edges() -> DemoEntry {
    DemoEntry {
        demo: Box::new(CustomEdgesDemo::new()),
        meta: rataflow_examples::meta::custom_edges(),
    }
}

pub fn entry_floating_edges() -> DemoEntry {
    DemoEntry {
        demo: Box::new(FloatingEdgesDemo::new()),
        meta: rataflow_examples::meta::floating_edges(),
    }
}

pub fn entry_edge_routing() -> DemoEntry {
    DemoEntry {
        demo: Box::new(EdgeRoutingDemo::new()),
        meta: rataflow_examples::meta::edge_routing(),
    }
}

pub fn entry_animating_edges() -> DemoEntry {
    DemoEntry {
        demo: Box::new(AnimatingEdgesDemo::new()),
        meta: rataflow_examples::meta::animating_edges(),
    }
}

pub fn entry_multi_select() -> DemoEntry {
    DemoEntry {
        demo: Box::new(MultiSelectDemo::new()),
        meta: rataflow_examples::meta::multi_select(),
    }
}

pub fn entry_custom_bindings() -> DemoEntry {
    DemoEntry {
        demo: Box::new(CustomBindingsDemo::new()),
        meta: rataflow_examples::meta::custom_bindings(),
    }
}

pub fn entry_events() -> DemoEntry {
    DemoEntry {
        demo: Box::new(EventsDemo::new()),
        meta: rataflow_examples::meta::events(),
    }
}

pub fn entry_validation() -> DemoEntry {
    DemoEntry {
        demo: Box::new(ValidationDemo::new()),
        meta: rataflow_examples::meta::validation(),
    }
}

pub fn entry_companion_widgets() -> DemoEntry {
    DemoEntry {
        demo: Box::new(CompanionWidgetsDemo::new()),
        meta: rataflow_examples::meta::companion_widgets(),
    }
}

pub fn entry_custom_layout() -> DemoEntry {
    DemoEntry {
        demo: Box::new(new_custom_layout()),
        meta: rataflow_examples::meta::custom_layout(),
    }
}

pub fn entry_undo_redo() -> DemoEntry {
    DemoEntry {
        demo: Box::new(UndoRedoDemo::new()),
        meta: rataflow_examples::meta::undo_redo(),
    }
}

pub fn entry_mutations() -> DemoEntry {
    DemoEntry {
        demo: Box::new(MutationsDemo::new()),
        meta: rataflow_examples::meta::mutations(),
    }
}

pub fn entry_theming() -> DemoEntry {
    DemoEntry {
        demo: Box::new(ThemingDemo::new()),
        meta: rataflow_examples::meta::theming(),
    }
}

pub fn entry_save_restore() -> DemoEntry {
    DemoEntry {
        demo: Box::new(SaveRestoreDemo::new()),
        // false = the pre-save copy, which is what this entry always showed:
        // the wasm demo builds its sidebar once at construction and does
        // not re-render the description when the graph is saved.
        meta: rataflow_examples::meta::save_restore(false),
    }
}

pub fn entry_reconnection() -> DemoEntry {
    DemoEntry {
        demo: Box::new(new_reconnection()),
        meta: rataflow_examples::meta::reconnection(),
    }
}

pub fn entry_context_menu() -> DemoEntry {
    DemoEntry {
        demo: Box::new(ContextMenuDemo::new()),
        meta: rataflow_examples::meta::context_menu(),
    }
}

// ============================================================================
// Standard demos (DemoApp + blanket Demo impl)
// ============================================================================

fn new_basic() -> DemoApp<TextContent, StepEdge> {
    DemoApp::from_flow(rataflow_examples::basic::basic())
}

fn new_reconnection() -> DemoApp<TextContent, StepEdge> {
    DemoApp::from_flow(rataflow_examples::reconnection::create_flow())
}

fn new_custom_nodes() -> DemoApp<rataflow_examples::MyNode, StepEdge> {
    DemoApp::from_flow(rataflow_examples::custom_nodes::create_flow())
}

// ============================================================================
// FloatingEdgesDemo — 'a' cycles the attachment, status line names it
// ============================================================================

pub struct FloatingEdgesDemo {
    app: DemoApp<TextContent, DemoEdge>,
    attach: Attach,
}

impl FloatingEdgesDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::floating_edges::create_flow()),
            attach: Attach::Stepped,
        }
    }
}

impl Demo for FloatingEdgesDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.app
            .flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        match event.code {
            KeyCode::Char('a') => {
                self.attach = self.attach.next();
                rataflow_examples::floating_edges::set_attach(&mut self.app.flow, self.attach);
            }
            _ => Demo::handle_key(&mut self.app, event),
        }
    }

    // Not the blanket impl: that one builds a new edge with `E::default()`, so an
    // edge drawn while mode 3 is on screen would come out rendering as mode 1.
    // New edges take the attachment currently showing, as they do natively.
    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        for e in self.app.flow.handle_mouse_event(event).into_events() {
            match e {
                FlowEvent::ConnectionCompleted(conn) => {
                    self.app.flow.add_edge_from_connection(
                        conn,
                        DemoEdge {
                            attach: self.attach,
                        },
                    );
                }
                FlowEvent::ReconnectionCompleted {
                    edge_id,
                    new_connection,
                    ..
                } => {
                    self.app.flow.reconnect_edge(&edge_id, new_connection);
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.app.render(frame, area);
        let step = Attach::CYCLE
            .iter()
            .position(|a| *a == self.attach)
            .unwrap_or(0)
            + 1;
        render_status(frame, area, &format!("{step}/4  {}", self.attach.label()));
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}

pub struct EdgeRoutingDemo {
    app: DemoApp<TextContent, rataflow_examples::edge_routing::RoutingEdge>,
}

impl EdgeRoutingDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::edge_routing::create_flow()),
        }
    }
}

impl Demo for EdgeRoutingDemo {
    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        Demo::handle_key(&mut self.app, event);
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        Demo::handle_mouse(&mut self.app, event);
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.app.render(frame, area);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }

    fn tick(&mut self, elapsed_ms: f64) {
        let elapsed = Duration::from_millis(elapsed_ms as u64);
        self.app.flow.tick_animation(elapsed);
        self.app.flow.tick_auto_pan(elapsed);
    }
}

// ============================================================================
// CustomEdgesDemo — standard + tick override (native example ticks animation)
// ============================================================================

pub struct CustomEdgesDemo {
    app: DemoApp<TextContent, rataflow_examples::MyEdge>,
}

impl CustomEdgesDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::custom_edges::create_flow()),
        }
    }
}

impl Demo for CustomEdgesDemo {
    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        Demo::handle_key(&mut self.app, event);
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        Demo::handle_mouse(&mut self.app, event);
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.app.render(frame, area);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }

    fn tick(&mut self, elapsed_ms: f64) {
        let elapsed = Duration::from_millis(elapsed_ms as u64);
        self.app.flow.tick_animation(elapsed);
        self.app.flow.tick_auto_pan(elapsed);
    }
}

fn new_hierarchy() -> DemoApp<TextContent, StepEdge> {
    DemoApp::from_flow(rataflow_examples::hierarchy::create_flow())
}

// ============================================================================
// NodeFlagsDemo — runtime flag toggling
// ============================================================================

pub struct NodeFlagsDemo {
    app: DemoApp<TextContent, StepEdge>,
    last_hidden: Option<String>,
}

impl NodeFlagsDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::node_flags::create_flow()),
            last_hidden: None,
        }
    }
}

impl Demo for NodeFlagsDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.app
            .flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        use rataflow_examples::node_flags::update_flag_label;

        match event.code {
            KeyCode::Char('d') => {
                if let Some(id) = self.app.flow.first_selected_node_id() {
                    let v = self.app.flow.node(&id).is_none_or(|n| n.draggable);
                    self.app.flow.set_node_draggable(&id, !v);
                    update_flag_label(&mut self.app.flow, &id);
                }
            }
            KeyCode::Char('s') => {
                if let Some(id) = self.app.flow.first_selected_node_id() {
                    let v = self.app.flow.node(&id).is_none_or(|n| n.selectable);
                    self.app.flow.set_node_selectable(&id, !v);
                    update_flag_label(&mut self.app.flow, &id);
                }
            }
            KeyCode::Char('p') => {
                if let Some(id) = self.app.flow.first_selected_node_id() {
                    let v = self.app.flow.node(&id).is_none_or(|n| n.deletable);
                    self.app.flow.set_node_deletable(&id, !v);
                    update_flag_label(&mut self.app.flow, &id);
                }
            }
            KeyCode::Char('o') => {
                if let Some(id) = self.app.flow.first_selected_node_id() {
                    let v = self.app.flow.node(&id).is_none_or(|n| n.connectable);
                    self.app.flow.set_node_connectable(&id, !v);
                    update_flag_label(&mut self.app.flow, &id);
                }
            }
            KeyCode::Char('v') => {
                if let Some(id) = self.last_hidden.take() {
                    self.app.flow.set_node_hidden(&id, false);
                    update_flag_label(&mut self.app.flow, &id);
                } else if let Some(id) = self.app.flow.first_selected_node_id() {
                    self.app.flow.set_node_hidden(&id, true);
                    update_flag_label(&mut self.app.flow, &id);
                    self.last_hidden = Some(id);
                }
            }
            KeyCode::Char('z') => {
                if let Some(id) = self.app.flow.first_selected_node_id() {
                    let next = match self.app.flow.node(&id).map_or(0, |n| n.z_index) {
                        0 => 5,
                        5 => -5,
                        _ => 0,
                    };
                    self.app.flow.set_node_z_index(&id, next);
                    update_flag_label(&mut self.app.flow, &id);
                }
            }
            _ => Demo::handle_key(&mut self.app, event),
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        Demo::handle_mouse(&mut self.app, event);
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.app.render(frame, area);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}

// ============================================================================
// AnimatingEdgesDemo — standard + tick override
// ============================================================================

pub struct AnimatingEdgesDemo {
    app: DemoApp<TextContent, StepEdge>,
}

impl AnimatingEdgesDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::animating_edges()),
        }
    }
}

impl Demo for AnimatingEdgesDemo {
    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        match event.code {
            KeyCode::Char('<') => {
                self.app.flow.animation_speed_ms = (self.app.flow.animation_speed_ms + 20).min(500);
            }
            KeyCode::Char('>') => {
                self.app.flow.animation_speed_ms =
                    self.app.flow.animation_speed_ms.saturating_sub(20).max(20);
            }
            _ => Demo::handle_key(&mut self.app, event),
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        Demo::handle_mouse(&mut self.app, event);
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.app.render(frame, area);

        // Speed indicator (top-right)
        let label = format!("SPEED: {:>3}ms", self.app.flow.animation_speed_ms);
        render_indicator(
            frame,
            area,
            &label,
            Style::default().fg(Color::Indexed(242)),
        );
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }

    fn tick(&mut self, elapsed_ms: f64) {
        let elapsed = Duration::from_millis(elapsed_ms as u64);
        self.app.flow.tick_animation(elapsed);
        self.app.flow.tick_auto_pan(elapsed);
    }
}

// ============================================================================
// CustomLayoutDemo — standard + post-construction layout
// ============================================================================

fn new_custom_layout() -> DemoApp<TextContent, StepEdge> {
    let mut flow = rataflow_examples::custom_layout::create_flow();

    let graph_edges: &[(usize, usize)] = &[
        (0, 1),
        (0, 2),
        (1, 3),
        (1, 4),
        (2, 5),
        (2, 6),
        (3, 7),
        (3, 8),
    ];
    let positions = rataflow_examples::compute_layout(graph_edges, 14.0, 5.0);
    flow.set_node_positions(positions);

    DemoApp::from_flow(flow)
}

// ============================================================================
// ViewOnlyDemo — no key/mouse handling
// ============================================================================

pub struct ViewOnlyDemo {
    flow: Flow<TextContent, StepEdge>,
}

impl ViewOnlyDemo {
    pub fn new() -> Self {
        let flow = rataflow_examples::view_only::create_flow();
        Self { flow }
    }
}

impl Demo for ViewOnlyDemo {
    fn handle_key(&mut self, _event: rataflow::KeyEvent) {}
    fn handle_mouse(&mut self, _event: rataflow::MouseEvent) {}

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Background::new(&self.flow), area);
        frame.render_widget(&mut self.flow, area);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.flow
    }
}

// ============================================================================
// CustomBindingsDemo — custom key handler
// ============================================================================

pub struct CustomBindingsDemo {
    app: DemoApp<TextContent, StepEdge>,
}

impl CustomBindingsDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::basic::basic()),
        }
    }
}

impl Demo for CustomBindingsDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.app
            .flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        // Try custom bindings first, then fall through to defaults
        if let Some(action) = custom_controls_bindings(&event) {
            self.app.flow.apply_controls_action(action);
        } else if let Some(action) = custom_flow_bindings(&event) {
            self.app.flow.apply(action);
        } else if let Some(action) = default_controls_key_binding(&event) {
            self.app.flow.apply_controls_action(action);
        } else if let Some(action) = default_flow_key_binding(&event) {
            self.app.flow.apply(action);
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        for flow_event in self.app.flow.handle_mouse_event(event).into_events() {
            match flow_event {
                FlowEvent::ConnectionCompleted(conn) => {
                    self.app
                        .flow
                        .add_edge_from_connection(conn, StepEdge::default());
                }
                FlowEvent::ReconnectionCompleted {
                    edge_id,
                    new_connection,
                    ..
                } => {
                    self.app.flow.reconnect_edge(&edge_id, new_connection);
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Background::new(&self.app.flow), area);
        frame.render_widget(&mut self.app.flow, area);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}

// ============================================================================
// CompanionWidgetsDemo — custom widget configuration
// ============================================================================

pub struct CompanionWidgetsDemo {
    app: DemoApp<TextContent, StepEdge>,
}

impl CompanionWidgetsDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::basic::basic()),
        }
    }
}

impl Demo for CompanionWidgetsDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.app
            .flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        Demo::handle_key(&mut self.app, event);
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        Demo::handle_mouse(&mut self.app, event);
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Cool pastel tint: soft lavenders and pale blues from the 256-color cube.
        const TINT_BASE: Color = Color::Indexed(236);
        const TINT_SLATE: Color = Color::Indexed(60);
        const TINT_MIST: Color = Color::Indexed(109);
        const TINT_LAVENDER: Color = Color::Indexed(146);
        const TINT_SKY: Color = Color::Indexed(153);

        // Cross pattern with tight spacing and pastel tint
        frame.render_widget(
            Background::new(&self.app.flow)
                .variant(BackgroundVariant::Cross)
                .gap(6, 3)
                .style(
                    BackgroundStyle::default()
                        .with_bg_color(TINT_BASE)
                        .with_pattern_color(TINT_MIST),
                ),
            area,
        );

        frame.render_widget(&mut self.app.flow, area);

        // Horizontal controls, top-right
        frame.render_widget(
            Controls::new(&self.app.flow)
                .orientation(ControlsOrientation::Horizontal)
                .position(ControlsPosition::TopRight)
                .style(
                    ControlsStyle::default()
                        .with_border_style(Style::default().fg(TINT_MIST))
                        .with_button_style(Style::default().fg(TINT_SKY)),
                ),
            area,
        );

        // Bordered minimap, bottom-left, wider than default
        frame.render_widget(
            MiniMap::new(&self.app.flow)
                .position(MiniMapPosition::BottomLeft)
                .size(30, 10)
                .margin(2)
                .block(Block::bordered().border_style(Style::default().fg(TINT_MIST)))
                .style(
                    MiniMapStyle::default()
                        .with_bg_color(TINT_SLATE)
                        .with_node_color(TINT_LAVENDER)
                        .with_viewport_color(TINT_MIST),
                ),
            area,
        );
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}

// ============================================================================
// MultiSelectDemo — custom key handler
// ============================================================================

pub struct MultiSelectDemo {
    app: DemoApp<TextContent, StepEdge>,
}

impl MultiSelectDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::basic::basic()),
        }
    }
}

impl Demo for MultiSelectDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.app
            .flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        match event.code {
            KeyCode::Char('m') => {
                self.app.flow.multi_select_mode = !self.app.flow.multi_select_mode;
            }
            // The page hands the right button to the canvas, so the box gesture
            // works here as it does natively. Kept anyway: it is the same escape
            // hatch the native example offers a terminal that keeps the button,
            // and the sidebar lists it either way.
            KeyCode::Char('b') => {
                self.app.flow.selection_on_drag = !self.app.flow.selection_on_drag;
            }
            KeyCode::Char('d') => {
                self.app.flow.apply(FlowAction::Delete);
            }
            KeyCode::Char('s') => {
                let ids: Vec<String> = self
                    .app
                    .flow
                    .selected_nodes()
                    .map(|n| n.id.clone())
                    .collect();
                self.app
                    .flow
                    .request_fit_view_with_options(FitViewOptions::default().with_nodes(ids));
            }
            _ => {
                Demo::handle_key(&mut self.app, event);
            }
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        Demo::handle_mouse(&mut self.app, event);
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.app.render(frame, area);

        // Multi-select indicator (top-right)
        let (text, style) = if self.app.flow.multi_select_mode {
            (
                "MULTI: ON ",
                Style::default()
                    .fg(Color::Indexed(232))
                    .bg(Color::Indexed(179)),
            )
        } else {
            ("MULTI: OFF", Style::default().fg(Color::Indexed(242)))
        };
        render_indicator(frame, area, text, style);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}

// ============================================================================
// ValidationDemo — custom key handler
// ============================================================================

pub struct ValidationDemo {
    app: DemoApp<TextContent, StepEdge>,
}

impl ValidationDemo {
    pub fn new() -> Self {
        let flow = rataflow_examples::validation::create_flow();
        Self {
            app: DemoApp::from_flow(flow),
        }
    }
}

impl Demo for ValidationDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.app
            .flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        match event.code {
            KeyCode::Char('o') => {
                self.app.flow.connection_mode = match self.app.flow.connection_mode {
                    ConnectionMode::Strict => ConnectionMode::Loose,
                    ConnectionMode::Loose => ConnectionMode::Strict,
                };
            }
            _ => {
                Demo::handle_key(&mut self.app, event);
            }
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        Demo::handle_mouse(&mut self.app, event);
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.app.render(frame, area);

        // Connection mode indicator (top-right)
        let (text, style) = if matches!(self.app.flow.connection_mode, ConnectionMode::Strict) {
            ("MODE: Strict", Style::default().fg(Color::Indexed(242)))
        } else {
            (
                "MODE: Loose ",
                Style::default()
                    .fg(Color::Indexed(232))
                    .bg(Color::Indexed(179)),
            )
        };
        render_indicator(frame, area, text, style);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}

// ============================================================================
// EventsDemo — custom render (event log panel)
// ============================================================================

pub struct EventsDemo {
    app: DemoApp<TextContent, StepEdge>,
    event_log: VecDeque<String>,
    log_max: usize,
}

impl EventsDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::basic::basic()),
            event_log: VecDeque::new(),
            log_max: 100,
        }
    }
}

fn log_event(log: &mut VecDeque<String>, max_entries: usize, event: &FlowEvent) {
    let msg = match event {
        FlowEvent::NodeClicked { node_id } => format!("NodeClicked: {node_id}"),
        FlowEvent::NodeDragStarted { node_id } => format!("NodeDragStarted: {node_id}"),
        FlowEvent::NodeDragged { node_id } => format!("NodeDragged: {node_id}"),
        FlowEvent::NodeDragEnded { node_id } => format!("NodeDragStopped: {node_id}"),
        FlowEvent::EdgeClicked { edge_id } => format!("EdgeClicked: {edge_id}"),
        FlowEvent::PaneClicked { x, y } => format!("PaneClicked: ({x:.1}, {y:.1})"),
        FlowEvent::NodeContextMenu { node_id } => format!("NodeContextMenu: {node_id}"),
        FlowEvent::EdgeContextMenu { edge_id } => format!("EdgeContextMenu: {edge_id}"),
        FlowEvent::PaneContextMenu { x, y } => format!("PaneContextMenu: ({x:.1}, {y:.1})"),
        FlowEvent::NodeResizeStarted { node_id } => format!("NodeResizeStarted: {node_id}"),
        FlowEvent::NodeResized { node_id } => format!("NodeResized: {node_id}"),
        FlowEvent::NodeResizeEnded { node_id } => format!("NodeResizeEnded: {node_id}"),
        FlowEvent::ViewportChanged { x, y, zoom } => {
            format!("ViewportChanged: ({x:.1}, {y:.1}) z={zoom:.2}")
        }
        FlowEvent::SelectionChanged { node_ids, edge_ids } => {
            format!("SelectionChanged: nodes={node_ids:?} edges={edge_ids:?}")
        }
        FlowEvent::ConnectionStarted { node_id, handle_id } => {
            format!("ConnectionStarted: {node_id} handle={handle_id:?}")
        }
        FlowEvent::ConnectionCompleted(conn) => {
            format!("ConnectionCompleted: {} -> {}", conn.source, conn.target)
        }
        FlowEvent::ConnectionCancelled => "ConnectionCancelled".to_string(),
        FlowEvent::Deleted { node_ids, edge_ids } => {
            format!("Deleted: nodes={node_ids:?} edges={edge_ids:?}")
        }
        FlowEvent::ReconnectionStarted {
            edge_id,
            handle_type,
        } => {
            format!("ReconnectionStarted: {edge_id} ({handle_type:?})")
        }
        FlowEvent::ReconnectionCompleted {
            edge_id,
            old_connection,
            new_connection,
        } => {
            format!(
                "ReconnectionCompleted: {edge_id} {} -> {} => {} -> {}",
                old_connection.source,
                old_connection.target,
                new_connection.source,
                new_connection.target,
            )
        }
        FlowEvent::ReconnectionCancelled { edge_id } => {
            format!("ReconnectionCancelled: {edge_id}")
        }
        // FlowEvent is #[non_exhaustive], so the library can add a variant
        // without breaking semver — and without this arm the website stops
        // compiling the moment it does. Debug rather than "unknown": this is an
        // event LOG, so an unnamed-but-visible event beats a silent one.
        other => format!("{other:?}"),
    };

    log.push_back(msg);
    while log.len() > max_entries {
        log.pop_front();
    }
}

impl Demo for EventsDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.app
            .flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        let response = self.app.flow.handle_controls_key_event(event);
        if matches!(response, EventResponse::NotHandled) {
            for flow_event in self.app.flow.handle_key_event(event).into_events() {
                log_event(&mut self.event_log, self.log_max, &flow_event);
            }
        } else {
            for flow_event in response.into_events() {
                log_event(&mut self.event_log, self.log_max, &flow_event);
            }
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        for flow_event in self.app.flow.handle_mouse_event(event).into_events() {
            match &flow_event {
                FlowEvent::ConnectionCompleted(conn) => {
                    self.app
                        .flow
                        .add_edge_from_connection(conn.clone(), StepEdge::default());
                }
                FlowEvent::ReconnectionCompleted {
                    edge_id,
                    new_connection,
                    ..
                } => {
                    self.app
                        .flow
                        .reconnect_edge(edge_id, new_connection.clone());
                }
                _ => {}
            }
            log_event(&mut self.event_log, self.log_max, &flow_event);
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        frame.render_widget(Background::new(&self.app.flow), chunks[0]);
        frame.render_widget(&mut self.app.flow, chunks[0]);
        frame.render_widget(Controls::new(&self.app.flow), chunks[0]);

        let inner_h = chunks[1].height.saturating_sub(2) as usize;
        self.log_max = inner_h;
        let lines: Vec<Line> = self
            .event_log
            .iter()
            .map(|s| Line::from(s.as_str()))
            .collect();
        let scroll = lines.len().saturating_sub(inner_h) as u16;
        let log_widget = Paragraph::new(Text::from(lines))
            .block(Block::bordered().title("Events"))
            .style(
                Style::default()
                    .fg(Color::Indexed(242))
                    .bg(Color::Indexed(232)),
            )
            .scroll((scroll, 0));
        frame.render_widget(log_widget, chunks[1]);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}

// ============================================================================
// UndoRedoDemo — custom key/mouse handlers
// ============================================================================

pub struct UndoRedoDemo {
    app: DemoApp<TextContent, StepEdge>,
    history: History,
    counter: u32,
}

impl UndoRedoDemo {
    pub fn new() -> Self {
        let app = DemoApp::from_flow(rataflow_examples::basic::basic());
        let history = History::new(&app.flow);
        Self {
            app,
            history,
            counter: 0,
        }
    }
}

impl Demo for UndoRedoDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.app
            .flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        match event.code {
            KeyCode::Char('u') => {
                self.history.undo(&mut self.app.flow);
                return;
            }
            KeyCode::Char('U') => {
                self.history.redo(&mut self.app.flow);
                return;
            }
            _ => {}
        }

        match event.code {
            KeyCode::Char('a') => {
                let id = format!("new_{}", self.counter);
                self.counter += 1;
                let area = self.app.flow.canvas_area();
                let offset = ((self.counter - 1) % 10) as f64 * 3.0;
                let center = self.app.flow.viewport.canvas_to_world(Position::new(
                    area.width as f64 / 2.0,
                    area.height as f64 / 2.0,
                ));
                let node =
                    Node::from_text(&id, (center.x + offset, center.y + offset), id.as_str());
                let _ = self.app.flow.add_node(node);
                self.history.push(&self.app.flow);
            }
            KeyCode::Delete | KeyCode::Backspace => {
                self.app.flow.remove_selected_nodes();
                self.app.flow.remove_selected_edges();
                self.history.push(&self.app.flow);
            }
            KeyCode::Char('c') => {
                self.app.flow.center_on_selected();
            }
            _ => {
                Demo::handle_key(&mut self.app, event);
            }
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        for flow_event in self.app.flow.handle_mouse_event(event).into_events() {
            match flow_event {
                FlowEvent::ConnectionCompleted(conn) => {
                    self.app
                        .flow
                        .add_edge_from_connection(conn, StepEdge::default());
                    self.history.push(&self.app.flow);
                }
                FlowEvent::NodeDragEnded { .. } => {
                    self.history.push(&self.app.flow);
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.app.render(frame, area);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}

// ============================================================================
// MutationsDemo — runtime graph mutation key handler
// ============================================================================

pub struct MutationsDemo {
    app: DemoApp<TextContent, StepEdge>,
    counter: u32,
}

pub struct ThemingDemo {
    app: DemoApp<TextContent, StepEdge>,
    current_theme: Theme,
}

pub struct SaveRestoreDemo {
    app: DemoApp<TextContent, StepEdge>,
    saved_json: Option<String>,
}

impl MutationsDemo {
    pub fn new() -> Self {
        let flow = rataflow_examples::mutations::create_flow();
        Self {
            app: DemoApp::from_flow(flow),
            counter: 0,
        }
    }
}

impl Demo for MutationsDemo {
    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        match event.code {
            // Graph structure
            KeyCode::Char('a') => {
                let id = format!("new_{}", self.counter);
                self.counter += 1;
                let area = self.app.flow.canvas_area();
                let mut node = Node::from_text(&id, (0.0, 0.0), id.as_str());
                let center = self.app.flow.viewport.canvas_to_world(Position::new(
                    area.width as f64 / 2.0,
                    area.height as f64 / 2.0,
                ));
                node.position = Position::new(
                    center.x - node.dimensions().width / 2.0,
                    center.y - node.dimensions().height / 2.0,
                );
                let _ = self.app.flow.add_node(node);
            }
            KeyCode::Char('x') => {
                self.app.flow.remove_selected_nodes();
                self.app.flow.remove_selected_edges();
            }

            // Node mutations
            KeyCode::Char('r') => {
                if let Some(id) = self.app.flow.first_selected_node_id() {
                    if let Some(n) = self.app.flow.node(&id) {
                        let w = n.dimensions().width;
                        let (nw, nh) = if w < 10.0 {
                            (16.0, 5.0)
                        } else if w < 20.0 {
                            (24.0, 7.0)
                        } else {
                            (5.0, 3.0)
                        };
                        self.app.flow.set_node_dimensions(&id, nw, nh);
                    }
                }
            }
            KeyCode::Char('g') => {
                if let Some(id) = self.app.flow.first_selected_node_id() {
                    self.app.flow.move_node(&id, Position::new(5.0, 0.0));
                }
            }
            KeyCode::Char('n') => {
                if let Some(id) = self.app.flow.first_selected_node_id() {
                    if let Some(content) = self.app.flow.node_content_mut(&id) {
                        let current = content.text.to_string();
                        content.text = format!("{}'", current).into();
                    }
                }
            }
            KeyCode::Char('m') => {
                for (id, content) in self.app.flow.nodes_content_mut() {
                    content.text = id.to_string().into();
                }
            }

            // Edge mutations
            KeyCode::Char('e') => {
                self.app.flow.select_next_edge();
            }
            KeyCode::Char('b') => {
                if let Some(id) = self.app.flow.first_selected_edge_id() {
                    let current_label = self.app.flow.edge(&id).and_then(|e| e.label.clone());
                    let new_label = match current_label.as_deref() {
                        None => Some("flow".to_string()),
                        Some("flow") => Some("data".to_string()),
                        Some(_) => None,
                    };
                    self.app.flow.set_edge_label(&id, new_label);
                }
            }
            KeyCode::Char('w') => {
                if let Some(id) = self.app.flow.first_selected_edge_id() {
                    if let Some(e) = self.app.flow.edge(&id) {
                        let animated = !e.animated;
                        self.app.flow.set_edge_animated(&id, animated);
                    }
                }
            }

            // Flow config
            KeyCode::Char('t') => {
                self.app.flow.locked = !self.app.flow.locked;
            }

            _ => {
                Demo::handle_key(&mut self.app, event);
            }
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        Demo::handle_mouse(&mut self.app, event);
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.app.render(frame, area);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }

    fn tick(&mut self, elapsed_ms: f64) {
        let elapsed = Duration::from_millis(elapsed_ms as u64);
        self.app.flow.tick_animation(elapsed);
        self.app.flow.tick_auto_pan(elapsed);
    }
}

// ============================================================================
// ThemingDemo — runtime theme switching
// ============================================================================

impl ThemingDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::basic::basic()),
            current_theme: Theme::Dark,
        }
    }
}

impl Demo for ThemingDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.app
            .flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        match event.code {
            KeyCode::Char('t') => {
                self.current_theme = next_theme(&self.current_theme);
                apply_theme(&mut self.app.flow, self.current_theme);
            }
            _ => Demo::handle_key(&mut self.app, event),
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        for flow_event in self.app.flow.handle_mouse_event(event).into_events() {
            match flow_event {
                FlowEvent::ConnectionCompleted(conn) => {
                    self.app
                        .flow
                        .add_edge_from_connection(conn, StepEdge::default());
                }
                FlowEvent::ReconnectionCompleted {
                    edge_id,
                    new_connection,
                    ..
                } => {
                    self.app.flow.reconnect_edge(&edge_id, new_connection);
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.app.render(frame, area);

        let palette = self.current_theme.palette();
        let name = theme_name(&self.current_theme);
        let label = format!("THEME: {name:6}");
        let style = Style::default().fg(palette.text).bg(palette.surface);
        render_indicator(frame, area, &label, style);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}

// ============================================================================
// SaveRestoreDemo — serde snapshot save/restore with JSON panel
// ============================================================================

impl SaveRestoreDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::basic::basic()),
            saved_json: None,
        }
    }
}

impl Demo for SaveRestoreDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        self.app
            .flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        match event.code {
            KeyCode::Char('s') => {
                self.saved_json = Some(save(&self.app.flow));
            }
            KeyCode::Char('r') => {
                if let Some(ref json) = self.saved_json {
                    if let Some(restored) = restore(json) {
                        self.app.flow = restored;
                    }
                }
            }
            _ => Demo::handle_key(&mut self.app, event),
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        Demo::handle_mouse(&mut self.app, event);
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(area);

        frame.render_widget(Background::new(&self.app.flow), chunks[0]);
        frame.render_widget(&mut self.app.flow, chunks[0]);
        frame.render_widget(Controls::new(&self.app.flow), chunks[0]);
        frame.render_widget(MiniMap::new(&self.app.flow), chunks[0]);

        let json_text = self
            .saved_json
            .as_deref()
            .map(pretty_json)
            .unwrap_or_else(|| "No snapshot saved yet.\nPress 's' to save.".to_string());

        let lines: Vec<Line> = json_text
            .lines()
            .map(|l| Line::from(l.to_string()))
            .collect();
        let inner_h = chunks[1].height.saturating_sub(2) as usize;
        let scroll = lines.len().saturating_sub(inner_h) as u16;
        let json_widget = Paragraph::new(Text::from(lines))
            .block(Block::bordered().title("JSON Snapshot"))
            .style(
                Style::default()
                    .fg(Color::Indexed(242))
                    .bg(Color::Indexed(232)),
            )
            .scroll((scroll, 0));
        frame.render_widget(json_widget, chunks[1]);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}

// ============================================================================
// ContextMenuDemo — the menu owns the pointer and the keyboard while it is open
// ============================================================================

pub struct ContextMenuDemo {
    app: DemoApp<TextContent, StepEdge>,
    menu: Option<Menu>,
    counter: usize,
    status: String,
    /// Where the pointer last was, so Space opens a menu under it.
    cursor: Option<(u16, u16)>,
    /// The content area, learned at render — the menu is clamped to it.
    area: Rect,
}

impl ContextMenuDemo {
    pub fn new() -> Self {
        Self {
            app: DemoApp::from_flow(rataflow_examples::context_menu::create_flow()),
            menu: None,
            counter: 0,
            status: String::from("right-click something, or press Space"),
            cursor: None,
            area: Rect::default(),
        }
    }
}

impl Demo for ContextMenuDemo {
    fn tick(&mut self, elapsed_ms: f64) {
        let elapsed = Duration::from_millis(elapsed_ms as u64);
        self.app.flow.tick_auto_pan(elapsed);
        // "Toggle animation" is one of the edge menu's items, so this demo has
        // to drive the animation clock for it to show anything.
        self.app.flow.tick_animation(elapsed);
    }

    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        // An open menu takes the keyboard; otherwise the flow does.
        if let Some(open) = &mut self.menu {
            match event.code {
                KeyCode::Esc => self.menu = None,
                KeyCode::Up | KeyCode::Char('k') => open.select_prev(),
                KeyCode::Down | KeyCode::Char('j') => open.select_next(),
                KeyCode::Enter => {
                    self.status = run(&mut self.app.flow, open, &mut self.counter);
                    self.menu = None;
                }
                _ => {}
            }
            return;
        }

        match event.code {
            KeyCode::Char(' ') => {
                let (column, row) = self.cursor.unwrap_or((
                    self.area.x + self.area.width / 2,
                    self.area.y + self.area.height / 2,
                ));
                let target = target_at(&mut self.app.flow, self.area, column, row);
                self.menu = Some(Menu::open(target, column, row, self.area));
            }
            _ => Demo::handle_key(&mut self.app, event),
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        self.cursor = Some((event.column, event.row));

        // A left click inside an open menu picks an item; anywhere else
        // dismisses it. Either way the flow never sees it.
        if let Some(mut open) = self.menu.take() {
            if matches!(
                event.kind,
                rataflow::MouseEventKind::Down(rataflow::MouseButton::Left)
            ) {
                if let Some(index) = open.item_at(event.column, event.row) {
                    open.selected = index;
                    self.status = run(&mut self.app.flow, &open, &mut self.counter);
                }
                return;
            }
            // Not a click on the menu — leave it open.
            self.menu = Some(open);
        }

        let (column, row) = (event.column, event.row);
        for e in self.app.flow.handle_mouse_event(event).into_events() {
            match e {
                FlowEvent::NodeContextMenu { node_id } => {
                    self.menu = Some(Menu::open(Target::Node(node_id), column, row, self.area));
                }
                FlowEvent::EdgeContextMenu { edge_id } => {
                    self.menu = Some(Menu::open(Target::Edge(edge_id), column, row, self.area));
                }
                FlowEvent::PaneContextMenu { x, y } => {
                    self.menu = Some(Menu::open(Target::Pane(x, y), column, row, self.area));
                }
                FlowEvent::ConnectionCompleted(conn) => {
                    self.app
                        .flow
                        .add_edge_from_connection(conn, StepEdge::default());
                }
                FlowEvent::ReconnectionCompleted {
                    edge_id,
                    new_connection,
                    ..
                } => {
                    self.app.flow.reconnect_edge(&edge_id, new_connection);
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.area = area;
        self.app.render(frame, area);
        render_status(frame, area, &format!("last action: {}", self.status));

        if let Some(menu) = &self.menu {
            menu.render(frame.buffer_mut());
        }
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.app.flow
    }
}
