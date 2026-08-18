//! Basic example demonstrating rataflow.
//!
//! Shows a simple flow graph with nodes and edges.
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
use rataflow::{Background, Controls, EventResponse, FlowEvent, MiniMap, StepEdge};
use rataflow_examples::autopilot::{DemoPilot, Step};
use rataflow_examples::{ExampleMeta, render_shell};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::basic().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::basic::basic();

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    flow.request_fit_view();
    let mut last_tick = Instant::now();
    // Recording-only scripted pointer (RATAFLOW_DEMO=1), fired by 'g'.
    let mut demo = DemoPilot::from_env();

    'app: loop {
        let now = Instant::now();
        let elapsed = now - last_tick;
        flow.tick_auto_pan(elapsed);
        demo.tick_into(&mut flow, elapsed);
        last_tick = now;

        terminal.draw(|frame| {
            let area = render_shell(frame, frame.area(), &meta());
            frame.render_widget(Background::new(&flow), area);
            frame.render_widget(&mut flow, area);
            frame.render_widget(Controls::new(&flow), area);
            frame.render_widget(MiniMap::new(&flow), area);
            demo.draw(frame.buffer_mut());
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    CrosstermEvent::Key(key)
                        if demo.enabled() && key.code == KeyCode::Char('g') =>
                    {
                        // The article this records is about coordinates that stop
                        // being representable — a node panned past the left edge
                        // sits at a column the u16 framebuffer has no name for. So
                        // the gesture is a pan west until a node is mid-clip, a
                        // hold there, then a pan back to show nothing was lost.
                        //
                        // Press point is searched, not offset: pressing on a node
                        // drags the node instead of panning, which demonstrates
                        // the wrong thing entirely.
                        let occupied: Vec<(i32, i32, i32, i32)> = flow
                            .nodes()
                            .filter_map(|n| flow.node_terminal_rect(&n.id))
                            .collect();
                        let size = flow.canvas_size();
                        let (cw, ch) = (size.width as i32, size.height as i32);
                        const INSET: i32 = 8;
                        let free = |x: i32, y: i32| {
                            !occupied.iter().any(|(l, t, r, b)| {
                                x >= l - 2 && x <= r + 2 && y >= t - 1 && y <= b + 1
                            })
                        };
                        let mut spot = None;
                        'search: for y in (INSET..ch - INSET).rev() {
                            for x in (cw / 2)..(cw - INSET) {
                                if free(x, y) && free(INSET + 4, y) {
                                    spot = Some((x as u16, y as u16));
                                    break 'search;
                                }
                            }
                        }
                        let (px, py) = spot.unwrap_or(((cw - INSET) as u16, (ch / 2) as u16));
                        let west = (INSET + 4) as u16;
                        demo.start(
                            vec![
                                Step::MoveTo {
                                    col: px,
                                    row: py,
                                    secs: 0.5,
                                },
                                Step::Dwell(0.15),
                                Step::Press,
                                Step::Dwell(0.1),
                                // Slow, because the frames worth seeing are the few
                                // where a node is half-clipped at the boundary.
                                Step::MoveTo {
                                    col: west,
                                    row: py,
                                    secs: 1.3,
                                },
                                Step::Dwell(0.9),
                                Step::MoveTo {
                                    col: px,
                                    row: py,
                                    secs: 1.0,
                                },
                                Step::Release,
                                Step::Dwell(0.6),
                            ],
                            (px + 12, py + 5),
                        );
                    }
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
