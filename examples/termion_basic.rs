//! Basic example using the termion backend.
//!
//! Same Flow and rendering as `basic.rs` — only terminal setup differs.
//!
//! Run with: `cargo run --example termion_basic --no-default-features --features termion,sugiyama`
//!
//! - Drag node body to move it
//! - Drag from source handle (right side) to create edges
//! - Drag on empty space to pan the canvas
//! - Arrow keys or Tab: navigate between nodes
//! - h/j/k/l: pan viewport (keyboard)
//! - +/-: zoom in/out
//! - f: fit view (zoom to show all nodes)
//! - c: center on selected node
//! - i: toggle interactivity lock
//! - Delete/Backspace: delete selected
//! - q: quit

use std::{io, sync::mpsc, thread, time::Instant};

use rataflow::{
    Background, Controls, EventResponse, Flow, FlowEvent, MiniMap, StepEdge, TextContent,
};
use rataflow_examples::{ExampleMeta, render_shell};
use ratatui::{Terminal, backend::TermionBackend};
use termion::{
    event::Event as TermionEvent,
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::IntoAlternateScreen,
};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::termion_basic().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow: Flow<TextContent, StepEdge> = rataflow_examples::basic::basic();

    let stdout = MouseTerminal::from(io::stdout().into_raw_mode()?.into_alternate_screen()?);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    flow.request_fit_view();

    // Termion's event iterator is blocking with no poll(), so read on a
    // separate thread and drain the channel before each frame — same
    // batching behaviour as the crossterm examples.
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for event in io::stdin().events().flatten() {
            if tx.send(event).is_err() {
                return;
            }
        }
    });

    // Block for the first event, then drain any queued behind it.
    let mut last_tick = Instant::now();
    while let Ok(first) = rx.recv() {
        let now = Instant::now();
        flow.tick_auto_pan(now - last_tick);
        last_tick = now;
        let mut quit = false;
        for event in std::iter::once(first).chain(rx.try_iter()) {
            match event {
                TermionEvent::Key(termion::event::Key::Char('q')) => quit = true,
                TermionEvent::Key(key) => {
                    let response = flow.handle_controls_key_event(key);
                    if matches!(response, EventResponse::NotHandled) {
                        flow.handle_key_event(key);
                    }
                }
                TermionEvent::Mouse(mouse) => {
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
        if quit {
            break;
        }

        terminal.draw(|frame| render(frame, &mut flow))?;
    }

    Ok(())
}

fn render(frame: &mut ratatui::Frame, flow: &mut Flow<TextContent, StepEdge>) {
    let area = render_shell(frame, frame.area(), &meta());
    frame.render_widget(Background::new(&*flow), area);
    frame.render_widget(&mut *flow, area);
    frame.render_widget(Controls::new(&*flow), area);
    frame.render_widget(MiniMap::new(&*flow), area);
}
