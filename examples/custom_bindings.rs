//! Custom key bindings via the action-based pattern.
//!
//! The action pattern separates "what to do" ([`FlowAction`]/[`ControlsAction`]) from
//! "how to trigger it" (key bindings). This example adds custom bindings that take
//! priority, then falls through to defaults.
//!
//! Custom bindings (override defaults):
//! - WASD: pan (instead of hjkl)
//! - Tab/Shift+Tab: navigate (instead of arrow keys)
//! - z/x: zoom (instead of +/-)
//! - Space: fit view (instead of f)
//!
//! Default bindings (fallthrough):
//! - Del/Backspace, Esc, c, m, i, 0, etc.
//!
//! - q: quit

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{
    Background, FlowEvent, StepEdge, default_controls_key_binding, default_flow_key_binding,
};
use rataflow_examples::{
    ExampleMeta, custom_controls_bindings, custom_flow_bindings, render_shell,
};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::custom_bindings().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::basic::basic();
    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    flow.request_fit_view();
    let mut last_tick = Instant::now();

    'app: loop {
        let now = Instant::now();
        flow.tick_auto_pan(now - last_tick);
        last_tick = now;
        terminal.draw(|frame| {
            let area = render_shell(frame, frame.area(), &meta());
            frame.render_widget(Background::new(&flow), area);
            frame.render_widget(&mut flow, area);
        })?;

        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                CrosstermEvent::Key(key) => match key.code {
                    KeyCode::Char('q') => break 'app,
                    _ => {
                        let key_event = key.into();
                        // Try custom bindings first
                        if let Some(action) = custom_controls_bindings(&key_event) {
                            flow.apply_controls_action(action);
                        } else if let Some(action) = custom_flow_bindings(&key_event) {
                            flow.apply(action);
                        // Fall through to defaults
                        } else if let Some(action) = default_controls_key_binding(&key_event) {
                            flow.apply_controls_action(action);
                        } else if let Some(action) = default_flow_key_binding(&key_event) {
                            flow.apply(action);
                        }
                    }
                },
                CrosstermEvent::Mouse(mouse) => {
                    for event in flow.handle_mouse_event(mouse).into_events() {
                        match event {
                            FlowEvent::ConnectionCompleted(conn) => {
                                flow.add_edge_from_connection(conn, StepEdge::default());
                            }
                            FlowEvent::ReconnectionCompleted {
                                edge_id,
                                new_connection,
                                ..
                            } => {
                                flow.reconnect_edge(&edge_id, new_connection);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    execute!(stdout(), DisableMouseCapture)?;
    ratatui::restore();
    Ok(())
}
