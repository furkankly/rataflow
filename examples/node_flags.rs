//! Per-node interaction flags with runtime toggling.
//!
//! Resizing is per-node (`Node::resizable`), but how it *feels* is per-flow:
//! `min_node_size` is the floor a drag cannot shrink past, and
//! `resize_handle_radius` is how near the corner a drag has to start to count as
//! a resize rather than a move. Both are set below.
//!
//! Controls:
//! - Drag node body to move it (if draggable)
//! - Drag from source handle to create edges (if connectable)
//! - r: toggle resizable, then drag the ◢ grip at the node's bottom-right
//! - Arrow keys or Tab: navigate between selectable nodes
//! - h/j/k/l: pan viewport (keyboard)
//! - +/-: zoom in/out
//! - f: fit view
//! - c: center on selected node
//! - i: toggle interactivity lock
//! - Delete/Backspace: remove selected nodes (if deletable)
//! - d: toggle draggable on selected node
//! - s: toggle selectable on selected node
//! - p: toggle deletable on selected node
//! - o: toggle connectable on selected node
//! - v: toggle hidden (hide selected / unhide last hidden)
//! - z: cycle z-index (0 → 5 → −5 → 0)
//! - q: quit

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{Background, Controls, Dimensions, EventResponse, FlowEvent, MiniMap, StepEdge};
use rataflow_examples::node_flags::update_flag_label;
use rataflow_examples::{ExampleMeta, render_shell};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::node_flags().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::node_flags::create_flow();
    let mut last_hidden: Option<String> = None;

    // The library floor is 1x1, which is all a border and no room for content.
    // An app picks a size its own nodes stay legible at.
    flow.min_node_size = Dimensions::new(10.0, 3.0);
    // A wider grip than the 1.0 default, since one cell is a small target.
    flow.resize_handle_radius = 1.5;

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
                        KeyCode::Char('d') => {
                            if let Some(id) = flow.first_selected_node_id() {
                                let v = flow.node(&id).is_none_or(|n| n.draggable);
                                flow.set_node_draggable(&id, !v);
                                update_flag_label(&mut flow, &id);
                            }
                        }
                        KeyCode::Char('s') => {
                            if let Some(id) = flow.first_selected_node_id() {
                                let v = flow.node(&id).is_none_or(|n| n.selectable);
                                flow.set_node_selectable(&id, !v);
                                update_flag_label(&mut flow, &id);
                            }
                        }
                        KeyCode::Char('p') => {
                            if let Some(id) = flow.first_selected_node_id() {
                                let v = flow.node(&id).is_none_or(|n| n.deletable);
                                flow.set_node_deletable(&id, !v);
                                update_flag_label(&mut flow, &id);
                            }
                        }
                        KeyCode::Char('o') => {
                            if let Some(id) = flow.first_selected_node_id() {
                                let v = flow.node(&id).is_none_or(|n| n.connectable);
                                flow.set_node_connectable(&id, !v);
                                update_flag_label(&mut flow, &id);
                            }
                        }
                        KeyCode::Char('v') => {
                            if let Some(id) = last_hidden.take() {
                                flow.set_node_hidden(&id, false);
                                update_flag_label(&mut flow, &id);
                            } else if let Some(id) = flow.first_selected_node_id() {
                                flow.set_node_hidden(&id, true);
                                update_flag_label(&mut flow, &id);
                                last_hidden = Some(id);
                            }
                        }
                        KeyCode::Char('r') => {
                            if let Some(id) = flow.first_selected_node_id() {
                                let next = flow.node(&id).is_some_and(|n| !n.resizable);
                                flow.set_node_resizable(&id, next);
                                update_flag_label(&mut flow, &id);
                            }
                        }
                        KeyCode::Char('z') => {
                            if let Some(id) = flow.first_selected_node_id() {
                                let next = match flow.node(&id).map_or(0, |n| n.z_index) {
                                    0 => 5,
                                    5 => -5,
                                    _ => 0,
                                };
                                flow.set_node_z_index(&id, next);
                                update_flag_label(&mut flow, &id);
                            }
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
