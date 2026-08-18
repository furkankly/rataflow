//! Connection validation via [`ConnectionMode`], validators, and handle flags.
//!
//! Controls:
//! - Drag from source handle to target handle to create connections
//! - Arrow keys or Tab: navigate between nodes
//! - h/j/k/l: pan viewport (keyboard)
//! - +/-: zoom in/out
//! - f: fit view
//! - c: center on selected node
//! - i: toggle interactivity lock
//! - Delete/Backspace: delete selected
//! - o: toggle Strict/Loose mode
//! - q: quit

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{Background, ConnectionMode, Controls, EventResponse, FlowEvent, MiniMap, StepEdge};
use rataflow_examples::autopilot::{DemoPilot, Step};
use rataflow_examples::{ExampleMeta, accent_style, muted_style, render_indicator, render_shell};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::validation().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::validation::create_flow();

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
        for event in demo.tick_into(&mut flow, elapsed) {
            // Same handling the real mouse path gets below: without this the
            // scripted drag previews a connection and then drops it.
            if let FlowEvent::ConnectionCompleted(conn) = event {
                flow.add_edge_from_connection(conn, StepEdge::default());
            }
        }
        last_tick = now;
        terminal.draw(|frame| {
            let area = render_shell(frame, frame.area(), &meta());
            frame.render_widget(Background::new(&flow), area);
            frame.render_widget(&mut flow, area);
            frame.render_widget(Controls::new(&flow), area);
            frame.render_widget(MiniMap::new(&flow), area);

            // Connection mode indicator (top-right)
            let (text, style) = if matches!(flow.connection_mode, ConnectionMode::Strict) {
                ("MODE: Strict", muted_style())
            } else {
                ("MODE: Loose ", accent_style())
            };
            render_indicator(frame, area, text, style);
            // Last, so the pointer is never painted over by the indicator.
            demo.draw(frame.buffer_mut());
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    CrosstermEvent::Key(key)
                        if demo.enabled() && key.code == KeyCode::Char('g') =>
                    {
                        // The article this records is about the gesture: press a
                        // handle, drag, and a live preview validated against every
                        // other handle as it moves. So the script makes one
                        // connection that lands and two that are refused — and the
                        // two refusals fail for DIFFERENT reasons, which is what
                        // this example exists to show.
                        //
                        //   source -> no_outgoing   accepted (its TARGET handle is
                        //                           plain; only its source handle
                        //                           is blocked)
                        //   source -> rejected      refused by the validator
                        //                           (conn.target != "rejected")
                        //   source -> no_incoming   refused by a handle flag
                        //                           (connectable_end(false))
                        //
                        // An earlier cut dragged source -> target, which already
                        // carries edge e1: it made a duplicate of an edge sitting
                        // right there, so "accepted" looked like nothing happening.
                        // The accepted drag has to CREATE something visible.
                        //
                        // Handles sit on node edges: sources right, targets left.
                        // There is no handle-rect API, so each is the midpoint of
                        // the corresponding node edge.
                        let right = |id: &str| {
                            flow.node_terminal_rect(id)
                                .map(|r| (r.2.max(0) as u16, ((r.1 + r.3) / 2).max(0) as u16))
                        };
                        let left = |id: &str| {
                            flow.node_terminal_rect(id)
                                .map(|r| (r.0.max(0) as u16, ((r.1 + r.3) / 2).max(0) as u16))
                        };
                        if let (Some(src), Some(ok), Some(bad_rule), Some(bad_flag)) = (
                            right("source"),
                            left("no_outgoing"),
                            left("rejected"),
                            left("no_incoming"),
                        ) {
                            // One attempt: reach for the handle, press, drag the
                            // preview across, hold at the destination, release.
                            let attempt = |to: (u16, u16), hold: f64| {
                                vec![
                                    Step::MoveTo {
                                        col: src.0,
                                        row: src.1,
                                        secs: 0.5,
                                    },
                                    Step::Dwell(0.18),
                                    Step::Press,
                                    Step::Dwell(0.1),
                                    // Slow enough that the preview trailing the
                                    // pointer is legible — that preview IS the
                                    // subject, not the edge left behind.
                                    Step::MoveTo {
                                        col: to.0,
                                        row: to.1,
                                        secs: 0.9,
                                    },
                                    // Hold at the destination: for a refusal this is
                                    // the only moment it exists at all.
                                    Step::Dwell(hold),
                                    Step::Release,
                                    Step::Dwell(0.55),
                                ]
                            };
                            let mut steps = attempt(ok, 0.3);
                            steps.extend(attempt(bad_rule, 0.55));
                            steps.extend(attempt(bad_flag, 0.55));
                            // Rest on the finished graph: one new edge, and no trace
                            // whatever of the two that were refused.
                            steps.push(Step::Dwell(0.7));
                            demo.start(steps, (src.0.saturating_sub(14), src.1 + 6));
                        }
                    }
                    CrosstermEvent::Key(key) => match key.code {
                        KeyCode::Char('q') => break 'app,
                        KeyCode::Char('o') => {
                            flow.connection_mode =
                                if matches!(flow.connection_mode, ConnectionMode::Strict) {
                                    ConnectionMode::Loose
                                } else {
                                    ConnectionMode::Strict
                                };
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
