//! Undo/redo with snapshot-based history.
//!
//! Controls:
//! - a: add a new node (undoable)
//! - Delete/Backspace: delete selected (undoable)
//! - Drag nodes to move them (undoable on drop)
//! - Drag from handles to connect (undoable)
//! - u: undo
//! - U (shift+u): redo
//! - Arrow keys / h/j/k/l: navigate / pan
//! - +/-: zoom | f: fit view | c: center on selected
//! - i: toggle interactivity lock
//! - q: quit

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{Background, Controls, EventResponse, FlowEvent, MiniMap, Node, Position, StepEdge};
use rataflow_examples::{ExampleMeta, History, render_shell};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::undo_redo().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::basic::basic();
    let mut history = History::new(&flow);
    let mut counter: usize = 0;

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
                    CrosstermEvent::Key(key) => match key.code {
                        KeyCode::Char('q') => break 'app,
                        KeyCode::Char('u') => {
                            history.undo(&mut flow);
                        }
                        KeyCode::Char('U') => {
                            history.redo(&mut flow);
                        }
                        KeyCode::Char('a') => {
                            let id = format!("new_{}", counter);
                            counter += 1;
                            let area = flow.canvas_area();
                            let offset = ((counter - 1) % 10) as f64 * 3.0;
                            let center = flow.viewport.canvas_to_world(Position::new(
                                area.width as f64 / 2.0,
                                area.height as f64 / 2.0,
                            ));
                            let node = Node::from_text(
                                &id,
                                (center.x + offset, center.y + offset),
                                id.as_str(),
                            );
                            let _ = flow.add_node(node);
                            history.push(&flow);
                        }
                        KeyCode::Delete | KeyCode::Backspace => {
                            flow.remove_selected_nodes();
                            flow.remove_selected_edges();
                            history.push(&flow);
                        }
                        _ => {
                            let response = flow.handle_controls_key_event(key);
                            if matches!(response, EventResponse::NotHandled) {
                                flow.handle_key_event(key);
                            }
                        }
                    },
                    CrosstermEvent::Mouse(mouse) => {
                        for event in flow.handle_mouse_event(mouse).into_events() {
                            match event {
                                FlowEvent::ConnectionCompleted(conn) => {
                                    flow.add_edge_from_connection(conn, StepEdge::default());
                                    history.push(&flow);
                                }
                                FlowEvent::NodeDragEnded { .. } => {
                                    history.push(&flow);
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
