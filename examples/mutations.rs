//! Mutations — interactive runtime graph mutation through the Flow API.
//!
//! Controls:
//! - a: add node at viewport center
//! - x: delete selected nodes + edges
//! - r: cycle size (small → medium → large)
//! - g: nudge selected node right 5 units
//! - n: rename selected node via content_mut
//! - m: label every node with its id via nodes_content_mut
//! - e: select next edge
//! - b: cycle edge label (none → "flow" → "data" → none)
//! - w: toggle edge animated
//! - t: toggle lock
//! - Arrow keys / h/j/k/l / +/- / f / c: standard flow bindings
//! - q: quit

use std::{io::stdout, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{Background, Controls, EventResponse, FlowEvent, MiniMap, Node, Position, StepEdge};
use rataflow_examples::{ExampleMeta, render_shell};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::mutations().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::mutations::create_flow();
    let mut counter: usize = 0;
    let mut last_tick = std::time::Instant::now();

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    flow.request_fit_view();

    'app: loop {
        let now = std::time::Instant::now();
        let elapsed = now - last_tick;
        last_tick = now;
        flow.tick_animation(elapsed);
        flow.tick_auto_pan(elapsed);

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

                        // --- Graph structure ---
                        KeyCode::Char('a') => {
                            let id = format!("new_{}", counter);
                            counter += 1;
                            let area = flow.canvas_area();
                            let mut node = Node::from_text(&id, (0.0, 0.0), id.as_str());
                            let center = flow.viewport.canvas_to_world(Position::new(
                                area.width as f64 / 2.0,
                                area.height as f64 / 2.0,
                            ));
                            node.position = Position::new(
                                center.x - node.dimensions().width / 2.0,
                                center.y - node.dimensions().height / 2.0,
                            );
                            let _ = flow.add_node(node);
                        }
                        KeyCode::Char('x') => {
                            flow.remove_selected_nodes();
                            flow.remove_selected_edges();
                        }

                        // --- Node mutations ---
                        KeyCode::Char('r') => {
                            if let Some(id) = flow.first_selected_node_id()
                                && let Some(n) = flow.node(&id)
                            {
                                let w = n.dimensions().width;
                                let (nw, nh) = if w < 10.0 {
                                    (16.0, 5.0) // medium
                                } else if w < 20.0 {
                                    (24.0, 7.0) // large
                                } else {
                                    (5.0, 3.0) // small
                                };
                                flow.set_node_dimensions(&id, nw, nh);
                            }
                        }
                        KeyCode::Char('g') => {
                            if let Some(id) = flow.first_selected_node_id() {
                                flow.move_node(&id, Position::new(5.0, 0.0));
                            }
                        }
                        KeyCode::Char('n') => {
                            if let Some(id) = flow.first_selected_node_id()
                                && let Some(content) = flow.node_content_mut(&id)
                            {
                                let current = content.text.to_string();
                                content.text = format!("{}'", current).into();
                            }
                        }
                        KeyCode::Char('m') => {
                            for (id, content) in flow.nodes_content_mut() {
                                content.text = id.to_string().into();
                            }
                        }

                        // --- Edge mutations ---
                        KeyCode::Char('e') => {
                            flow.select_next_edge();
                        }
                        KeyCode::Char('b') => {
                            if let Some(id) = flow.first_selected_edge_id() {
                                let current_label = flow.edge(&id).and_then(|e| e.label.clone());
                                let new_label = match current_label.as_deref() {
                                    None => Some("flow".to_string()),
                                    Some("flow") => Some("data".to_string()),
                                    Some(_) => None,
                                };
                                flow.set_edge_label(&id, new_label);
                            }
                        }
                        KeyCode::Char('w') => {
                            if let Some(id) = flow.first_selected_edge_id()
                                && let Some(e) = flow.edge(&id)
                            {
                                let animated = !e.animated;
                                flow.set_edge_animated(&id, animated);
                            }
                        }

                        // --- Flow config ---
                        KeyCode::Char('t') => {
                            flow.locked = !flow.locked;
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
                            if let FlowEvent::ConnectionCompleted(conn) = event {
                                flow.add_edge_from_connection(conn, StepEdge::default());
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
