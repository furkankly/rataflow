//! `DemoApp` state bundle and `Demo` trait for type-erased website dispatch.
//!
//! Standard demos get `Demo` via the blanket impl on `DemoApp`. Custom demos
//! (events panel, undo-redo, etc.) implement `Demo` directly, delegating
//! render/flow_ops to an inner `DemoApp`.

use std::time::Duration;

use rataflow::{
    Background, Controls, EdgeContent, EventResponse, Flow, FlowEvent, FlowOps, MiniMap,
    NodeContent,
};
use ratatui::layout::Rect;
use ratatui::Frame;

/// State bundle for website demos.
///
/// Groups `Flow` with standard rendering.
pub struct DemoApp<N: NodeContent, E: EdgeContent> {
    pub flow: Flow<N, E>,
}

impl<N: NodeContent, E: EdgeContent> DemoApp<N, E> {
    pub fn from_flow(flow: Flow<N, E>) -> Self {
        Self { flow }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Background::new(&self.flow), area);
        frame.render_widget(&mut self.flow, area);
        frame.render_widget(Controls::new(&self.flow), area);
        frame.render_widget(MiniMap::new(&self.flow), area);
    }
}

pub trait Demo {
    /// Forward key events with controls delegation.
    fn handle_key(&mut self, event: rataflow::KeyEvent);

    /// Forward mouse events, handling ConnectionCompleted.
    fn handle_mouse(&mut self, event: rataflow::MouseEvent);

    /// Render the full widget composition.
    fn render(&mut self, frame: &mut Frame, area: Rect);

    /// Access the underlying Flow for non-generic operations.
    fn flow_ops(&mut self) -> &mut dyn FlowOps;

    /// Animation tick (default no-op).
    fn tick(&mut self, _elapsed_ms: f64) {}
}

impl<N: NodeContent, E: EdgeContent + Default> Demo for DemoApp<N, E> {
    fn handle_key(&mut self, event: rataflow::KeyEvent) {
        let r = self.flow.handle_controls_key_event(event);
        if matches!(r, EventResponse::NotHandled) {
            self.flow.handle_key_event(event);
        }
    }

    fn handle_mouse(&mut self, event: rataflow::MouseEvent) {
        for e in self.flow.handle_mouse_event(event).into_events() {
            match e {
                FlowEvent::ConnectionCompleted(conn) => {
                    self.flow.add_edge_from_connection(conn, E::default());
                }
                FlowEvent::ReconnectionCompleted {
                    edge_id,
                    new_connection,
                    ..
                } => {
                    self.flow.reconnect_edge(&edge_id, new_connection);
                }
                _ => {}
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.render(frame, area);
    }

    fn flow_ops(&mut self) -> &mut dyn FlowOps {
        &mut self.flow
    }

    fn tick(&mut self, elapsed_ms: f64) {
        self.flow
            .tick_auto_pan(Duration::from_millis(elapsed_ms as u64));
    }
}
