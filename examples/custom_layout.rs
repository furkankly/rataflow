//! Custom layout algorithm applied via `set_node_positions()`.
//!
//! Demonstrates how to use your own layout logic instead of the built-in Sugiyama.
//!
//! Controls:
//! - Drag node body to move it
//! - Drag from source handle to create edges
//! - Arrow keys or Tab: navigate between nodes
//! - h/j/k/l: pan viewport (keyboard)
//! - +/-: zoom in/out
//! - f: fit view (zoom to show all nodes)
//! - c: center on selected node
//! - i: toggle interactivity lock
//! - Delete/Backspace: delete selected
//! - q: quit

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{Background, Controls, EventResponse, FlowEvent, MiniMap, StepEdge};
use rataflow_examples::custom_layout::{compute_layout, create_flow};
use rataflow_examples::{ExampleMeta, render_shell};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::custom_layout().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = create_flow();

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
    let positions = compute_layout(graph_edges, 14.0, 5.0);
    flow.set_node_positions(positions);

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
                            if let FlowEvent::ConnectionCompleted(conn) = event {
                                flow.add_edge_from_connection(conn, StepEdge::default());
                            }
                        }
                    }
                    CrosstermEvent::Resize(_, _) => {
                        flow.request_fit_view();
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
