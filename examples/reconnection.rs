//! Edge reconnection example.
//!
//! Demonstrates the three reconnection modes:
//! - Both: drag either end of the edge to rewire it
//! - Target only: only the target end can be moved
//! - None: the edge cannot be reconnected at all
//!
//! Select an edge first, then drag near its source or target handle to detach and reconnect.

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{Background, Controls, EventResponse, FlowEvent, MiniMap, StepEdge};
use rataflow_examples::{ExampleMeta, render_shell};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::reconnection().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::reconnection::create_flow();

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
            frame.render_widget(Controls::new(&flow), area);
            frame.render_widget(MiniMap::new(&flow), area);
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    CrosstermEvent::Key(key) => {
                        if key.code == KeyCode::Char('q') {
                            break 'app;
                        }
                        let response = flow.handle_controls_key_event(key);
                        if matches!(response, EventResponse::NotHandled) {
                            flow.handle_key_event(key);
                        }
                    }
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
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
    }

    execute!(stdout(), DisableMouseCapture)?;
    ratatui::restore();
    Ok(())
}
