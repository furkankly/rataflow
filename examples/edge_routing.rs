//! Edge routing showcase for all handle position combinations.
//!
//! Displays a 4x4 grid for each edge type (Step and Straight),
//! covering all 16 source->target handle position pairings.
//!
//! - Arrow keys: navigate between nodes
//! - h/j/k/l: pan viewport
//! - +/-: zoom in/out
//! - f: fit view
//! - Scroll: zoom at cursor
//! - Drag empty space: pan
//! - Drag a handle: connect two nodes, stepped like the first grid
//! - q: quit

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{Background, Controls, EventResponse, FlowEvent};
use rataflow_examples::autopilot::{DemoPilot, Step};
use rataflow_examples::edge_routing::RoutingEdge;
use rataflow_examples::{ExampleMeta, render_shell};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::edge_routing().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::edge_routing::create_flow();

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
            demo.draw(frame.buffer_mut());
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    CrosstermEvent::Key(key) => {
                        if demo.enabled() && key.code == KeyCode::Char('g') {
                            // A routing example's story is a route being
                            // RECOMPUTED, so the gesture drags one target node
                            // around a loop: the step edge re-elbows continuously
                            // as the relative position of source and target sweeps
                            // through every quadrant. A static grid of 16 combos
                            // shows the outcomes; this shows the rule producing
                            // them.
                            //
                            // The node is found by id prefix, not hardcoded — the
                            // ids embed a Unicode arrow ("step_tgt_T→R") and the
                            // grid's contents are not a stable thing to name.
                            let size = flow.canvas_size();
                            let (cw, ch) = (size.width, size.height);
                            let mut best: Option<(f64, u16, u16)> = None;
                            for node in flow.nodes() {
                                if !node.id.starts_with("step_tgt_") {
                                    continue;
                                }
                                let Some(r) = flow.node_terminal_rect(&node.id) else {
                                    continue;
                                };
                                let (cx, cy) = ((r.0 + r.2) as f64 / 2.0, (r.1 + r.3) as f64 / 2.0);
                                // Most central wins: orbiting a node near an edge
                                // drags the pointer into the auto-pan zone and the
                                // canvas scrolls out from under the gesture.
                                let d = (cx - cw / 2.0).powi(2) + (cy - ch / 2.0).powi(2);
                                if best.is_none_or(|(bd, _, _)| d < bd) {
                                    best = Some((d, cx.max(0.0) as u16, cy.max(0.0) as u16));
                                }
                            }
                            if let Some((_, tx, ty)) = best {
                                demo.start(
                                    vec![
                                        // Zoom FIRST. An earlier cut orbited at fit
                                        // scale and zoomed afterwards, so the drag
                                        // was a few pixels of movement in a dense
                                        // grid and the zoom revealed a static
                                        // picture. The elbows have to be legible
                                        // WHILE they are being recomputed.
                                        Step::MoveTo {
                                            col: tx,
                                            row: ty,
                                            secs: 0.5,
                                        },
                                        Step::Scroll {
                                            up: true,
                                            clicks: 3,
                                        },
                                        Step::Dwell(0.5),
                                        Step::Press,
                                        Step::Dwell(0.12),
                                        Step::Orbit {
                                            rx: 12.0,
                                            ry: 6.0,
                                            secs: 2.4,
                                        },
                                        Step::Dwell(0.35),
                                        Step::Release,
                                        Step::Dwell(0.5),
                                        Step::Scroll {
                                            up: false,
                                            clicks: 3,
                                        },
                                        Step::Dwell(0.4),
                                    ],
                                    (tx.saturating_sub(16), ty + 6),
                                );
                            }
                        }
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
                                // A drawn edge gets `RoutingEdge`'s own default,
                                // which is the stepped route — the same thing the
                                // first section shows.
                                FlowEvent::ConnectionCompleted(conn) => {
                                    flow.add_edge_from_connection(conn, RoutingEdge::default());
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
