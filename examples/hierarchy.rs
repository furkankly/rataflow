//! Parent-child node relationships with relative positioning.
//!
//! Child positions are relative to their parent's top-left corner.
//! Demonstrates nesting, multi-level hierarchy, `NodeExtent::Parent`,
//! and `expand_parent` (parent auto-grows to contain child).
//!
//! Select a node to see both numbers at the bottom: the position it stores, and
//! the absolute bounds `Flow::node_bounds` resolves it to. For a root node they
//! agree; for a nested child they do not, which is the whole point of the query —
//! persisting or hit-testing a hierarchy needs the resolved answer, and walking
//! the parent chain by hand is how that answer drifts out of sync.
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

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{Background, Controls, EventResponse, Flow, FlowEvent, MiniMap, StepEdge};
use rataflow_examples::{ExampleMeta, render_shell, render_status};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::hierarchy().with_quit()
}

/// Stored position versus resolved absolute bounds, for the selected node.
fn positions_of_selected<N: rataflow::NodeContent>(flow: &Flow<N, StepEdge>) -> String {
    let Some(node) = flow.selected_nodes().next() else {
        return "select a node to compare its stored and absolute position".to_string();
    };
    let stored = node.position;
    let parent = node.parent_id.as_deref().unwrap_or("none");
    match flow.node_bounds(&node.id) {
        Some(bounds) => format!(
            "{}  parent: {parent}  stored: ({:.0}, {:.0})  absolute: ({:.0}, {:.0})",
            node.id,
            stored.x,
            stored.y,
            bounds.x(),
            bounds.y()
        ),
        None => format!("{}: no bounds", node.id),
    }
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::hierarchy::create_flow();

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

            render_status(frame, area, &positions_of_selected(&flow));
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
