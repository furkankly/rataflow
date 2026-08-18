//! Custom edge routing and rendering via the [`EdgeContent`] trait.
//!
//! Four visually distinct edge types, each demonstrating a different `EdgeContent`
//! capability: style builders, labeled paths, raw buffer decoration, and custom
//! path geometry.
//!
//! Controls:
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
use rataflow::{Background, Controls, EventResponse, FlowEvent, MiniMap};
use rataflow_examples::autopilot::{DemoPilot, Step};
use rataflow_examples::{ExampleMeta, MyEdge, render_shell};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::custom_edges().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::custom_edges::create_flow();

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    flow.request_fit_view();

    let mut last_tick = Instant::now();
    // Recording-only scripted pointer (RATAFLOW_DEMO=1), fired by 'g'.
    let mut demo = DemoPilot::from_env();

    'app: loop {
        let now = Instant::now();
        let elapsed = now - last_tick;
        last_tick = now;
        flow.tick_animation(elapsed);
        flow.tick_auto_pan(elapsed);
        demo.tick_into(&mut flow, elapsed);

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
                        // This example's own instruction is "click any edge to
                        // see its selected styling", so the script does exactly
                        // that: walk down the five renderings, clicking each.
                        // Edges have no terminal rect, so each click aims at the
                        // midpoint between the pair of nodes it joins.
                        let mid = |r: (i32, i32, i32, i32)| {
                            (((r.0 + r.2) / 2) as f64, ((r.1 + r.3) / 2) as f64)
                        };
                        let pairs = [
                            ("a1", "a2"),
                            ("b1", "b2"),
                            ("c1", "c2"),
                            ("d1", "d2"),
                            ("e1n", "e2n"),
                        ];
                        let mut steps = Vec::new();
                        let mut first = None;
                        for (src, dst) in pairs {
                            let (Some(a), Some(b)) =
                                (flow.node_terminal_rect(src), flow.node_terminal_rect(dst))
                            else {
                                continue;
                            };
                            let (ax, ay) = mid(a);
                            let (bx, by) = mid(b);
                            let (cx, cy) = ((ax + bx) / 2.0, (ay + by) / 2.0);
                            let (cx, cy) = (cx.max(0.0) as u16, cy.max(0.0) as u16);
                            if first.is_none() {
                                first = Some((cx.saturating_sub(14), cy + 6));
                            }
                            steps.push(Step::MoveTo {
                                col: cx,
                                row: cy,
                                secs: 0.4,
                            });
                            steps.push(Step::Dwell(0.12));
                            steps.push(Step::Press);
                            steps.push(Step::Dwell(0.08));
                            steps.push(Step::Release);
                            // Long enough to read the selected styling before the
                            // pointer leaves for the next one.
                            steps.push(Step::Dwell(0.55));
                        }
                        // Close on the alphabet itself: zoom into the last edge so
                        // the individual cells are legible, then back out.
                        steps.push(Step::Scroll {
                            up: true,
                            clicks: 2,
                        });
                        steps.push(Step::Dwell(0.9));
                        steps.push(Step::Scroll {
                            up: false,
                            clicks: 2,
                        });
                        steps.push(Step::Dwell(0.4));
                        demo.start(steps, first.unwrap_or((2, 2)));
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
                            if let FlowEvent::ConnectionCompleted(conn) = event {
                                flow.add_edge_from_connection(conn, MyEdge::default());
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
