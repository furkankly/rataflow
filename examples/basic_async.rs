//! Async variant of the basic example using tokio.
//!
//! Same graph and layout as `basic.rs`, but uses tokio for async event handling.
//! Demonstrates the channel + drain pattern for non-blocking event processing.
//!
//! Run with: cargo run --example basic_async

use std::io::stdout;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{Background, Controls, EventResponse, Flow, FlowEvent, MiniMap, StepEdge};
use rataflow_examples::{ExampleMeta, render_shell};
use tokio::sync::mpsc;
use tokio::time::Duration;

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::basic_async().with_quit()
}

#[tokio::main]
async fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::basic::basic();

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;

    // Spawn a reader task and forward events through a channel.
    // EventStream lacks non-blocking drain, so we use try_recv() instead.
    let (tx, mut rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        use futures::StreamExt;
        let mut reader = crossterm::event::EventStream::new();
        while let Some(Ok(event)) = reader.next().await {
            if tx.send(event).is_err() {
                break;
            }
        }
    });

    let mut tick = tokio::time::interval(Duration::from_millis(16));
    flow.request_fit_view();
    let mut last_tick = tokio::time::Instant::now();

    'app: loop {
        let now = tokio::time::Instant::now();
        flow.tick_auto_pan(now - last_tick);
        last_tick = now;
        terminal.draw(|frame| {
            let area = render_shell(frame, frame.area(), &meta());

            frame.render_widget(Background::new(&flow), area);
            frame.render_widget(&mut flow, area);
            frame.render_widget(Controls::new(&flow), area);
            frame.render_widget(MiniMap::new(&flow), area);
        })?;

        tokio::select! {
            _ = tick.tick() => {}
            Some(event) = rx.recv() => {
                if handle_event(&event, &mut flow) {
                    break 'app;
                }
            }
        }

        // Drain pending events before next render to avoid input lag.
        while let Ok(event) = rx.try_recv() {
            if handle_event(&event, &mut flow) {
                break 'app;
            }
        }
    }

    execute!(stdout(), DisableMouseCapture)?;
    ratatui::restore();
    Ok(())
}

/// Handle a single event. Returns true if the app should quit.
fn handle_event(event: &CrosstermEvent, flow: &mut Flow<rataflow::TextContent, StepEdge>) -> bool {
    match event {
        CrosstermEvent::Key(key) => {
            if key.code == KeyCode::Char('q') {
                return true;
            }
            let response = flow.handle_controls_key_event(*key);
            if matches!(response, EventResponse::NotHandled) {
                flow.handle_key_event(*key);
            }
        }
        CrosstermEvent::Mouse(mouse) => {
            for event in flow.handle_mouse_event(*mouse).into_events() {
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
    false
}
