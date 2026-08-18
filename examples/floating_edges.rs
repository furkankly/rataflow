//! Edges that re-pick their sides as the nodes move.
//!
//! An ordinary edge attaches to the handles its nodes declare and stays there,
//! however they are arranged — on default nodes that means leaving the source's
//! right and arriving on the target's left, even for a node sitting directly above.
//! [`FloatingEdge`](rataflow::FloatingEdge) derives its endpoints from the node
//! rectangles instead, re-picking every frame as the nodes move.
//!
//! Press `a` to cycle four attachments and drag a node under each:
//!
//! 1. `FloatingEdge`, stepped — the middle of whichever side faces the target
//! 2. `FloatingEdge`, straight — where the line between the two centers crosses
//!    the outline, so the endpoint slides as the node moves
//! 3. The same straight edge with `FloatingAttachment::Midpoint` — back to
//!    snapping, for comparison
//! 4. `StepEdge` — the handles the nodes declare, wherever the target is
//!
//! Two and three are the pair worth watching, and they differ by one field. Route
//! and attachment are separate settings on [`FloatingEdge`](rataflow::FloatingEdge):
//! each route has the attachment that suits it by default (a straight edge draws in
//! braille, which has the sub-cell resolution to show an endpoint sliding; a stepped
//! one draws in box-drawing characters, which do not), and either can take the
//! other.
//!
//! Nothing is declared to make any of this work — no handles on the relevant sides,
//! no flag on the edge or the graph. The `●` markers show where the handles are; the
//! floating variants ignore them, which is the point.
//!
//! Controls:
//! - Left-drag a node: the edges re-attach as it moves
//! - Left-drag a handle: connect two satellites and watch the new edge float too
//! - a: cycle the attachment
//! - q: quit

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode,
        KeyEventKind,
    },
    execute,
};
use rataflow::{Background, Controls, Flow, FlowEvent, TextContent};
use rataflow_examples::autopilot::{DemoPilot, Key, Step};
use rataflow_examples::floating_edges::{Attach, DemoEdge};
use rataflow_examples::{ExampleMeta, floating_edges, render_shell, render_status};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::floating_edges().with_quit()
}

/// The example's own key handling, lifted out of the event loop so a scripted
/// key travels the same path a typed one does. `a` is this example's logic, not
/// the library's, so it cannot be shortcut into the flow.
fn handle_key(
    code: KeyCode,
    flow: &mut Flow<TextContent, DemoEdge>,
    attach: &mut Attach,
) -> KeyOutcome {
    match code {
        KeyCode::Char('q') => return KeyOutcome::Quit,
        KeyCode::Char('a') => {
            *attach = attach.next();
            floating_edges::set_attach(flow, *attach);
        }
        _ => {
            flow.handle_key_event(crossterm::event::KeyEvent::new(
                code,
                crossterm::event::KeyModifiers::NONE,
            ));
        }
    }
    KeyOutcome::Continue
}

enum KeyOutcome {
    Continue,
    Quit,
}

/// Orbit radii, in cells. Cells are about twice as tall as wide, so `rx` twice
/// `ry` is a visually round lap.
///
/// Wide enough that there is an EDGE to watch. The first cut used 16x8, which
/// parked the node a couple of cells off the hub: the attachment still moved,
/// but across a stub too short to read. The radius has to clear both node bodies
/// and leave a run of line between them.
const ORBIT_RX: f64 = 25.0;
const ORBIT_RY: f64 = 12.0;

/// The recording script: drag one node around the same loop under each of the
/// four attachments, cycling with 'a' between passes.
///
/// Separated from the node it aims at, the way `overview`'s is, so the LENGTH of
/// the script can be answered without a terminal or a layout — `satellite`/`hub`
/// decide where the pointer goes, never how long it takes. That is what lets
/// `RATAFLOW_DEMO=duration` print a number the tape's `Sleep` can be set from
/// instead of one copied by hand.
fn demo_steps(satellite: (u16, u16), hub: (u16, u16)) -> Vec<Step> {
    // [`Step::Orbit`] anchors its ellipse at `(start.x - rx, start.y)`, so
    // starting the lap `rx` to the RIGHT of the hub puts the hub at the centre
    // of the circle. That is what makes one pass sweep the satellite through
    // every side of the hub instead of wobbling near one of them.
    let lap_start = (hub.0 + ORBIT_RX as u16, hub.1);

    let mut steps = vec![
        Step::MoveTo {
            col: satellite.0,
            row: satellite.1,
            secs: 0.5,
        },
        Step::Press,
        Step::Dwell(0.12),
        // Bring it in before circling. A satellite sits at the edge of the
        // fitted view by construction, and an orbit out there keeps the pointer
        // in the five-cell auto-pan zone for the whole lap — the first cut of
        // this recording spent four passes flying away from the graph.
        Step::MoveTo {
            col: lap_start.0,
            row: lap_start.1,
            secs: 0.7,
        },
        Step::Dwell(0.15),
    ];

    for pass in 0..4 {
        // No 'a' before the first pass: the example already starts on mode 1, so
        // cycling first would open on mode 2 and end where it began.
        if pass > 0 {
            steps.push(Step::Key(Key::Char('a')));
            steps.push(Step::Dwell(0.45));
            steps.push(Step::Press);
            steps.push(Step::Dwell(0.12));
        }
        steps.push(Step::Orbit {
            rx: ORBIT_RX,
            ry: ORBIT_RY,
            secs: 1.6,
        });
        steps.push(Step::Dwell(0.2));
        steps.push(Step::Release);
        steps.push(Step::Dwell(0.35));
    }
    steps.push(Step::Fit);
    steps.push(Step::Dwell(0.6));
    steps
}

fn main() -> rataflow_examples::Result<()> {
    // Answered before ratatui starts, so it prints to a normal stdout.
    if std::env::var("RATAFLOW_DEMO").as_deref() == Ok("duration") {
        println!(
            "{:.2}",
            rataflow_examples::autopilot::duration(&demo_steps((0, 0), (0, 0)))
        );
        return Ok(());
    }

    let mut flow = floating_edges::create_flow();
    let mut attach = Attach::Stepped;

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
        // Scripted keys go through the same handler a typed key does, because
        // 'a' is this example's logic rather than the flow's.
        for k in demo.take_keys() {
            let Key::Char(c) = k else { continue };
            if let KeyOutcome::Quit = handle_key(KeyCode::Char(c), &mut flow, &mut attach) {
                break 'app;
            }
        }
        last_tick = now;

        terminal.draw(|frame| {
            let area = render_shell(frame, frame.area(), &meta());
            frame.render_widget(Background::new(&flow), area);
            frame.render_widget(&mut flow, area);
            frame.render_widget(Controls::new(&flow), area);

            let step = Attach::CYCLE.iter().position(|a| *a == attach).unwrap_or(0) + 1;
            render_status(frame, area, &format!("{step}/4  {}", attach.label()));
            demo.draw(frame.buffer_mut());
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    // Terminals that negotiate the kitty keyboard protocol also
                    // report releases; without this every key would act twice.
                    CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                        if demo.enabled() && key.code == KeyCode::Char('g') {
                            // The story here is ATTACHMENT, so the script drags
                            // the same node around the same loop under each of
                            // the four modes: what changes between passes is
                            // only where the edge meets the node. Cycling with
                            // 'a' between orbits rather than mid-drag keeps each
                            // pass readable, and the status bar names the mode
                            // being shown.
                            //
                            // It grabs the satellite and orbits it AROUND the
                            // hub, which sweeps the facing side through all four
                            // sides at both ends of the edge. Dragging the hub
                            // instead moves only the shared end and never gets
                            // far enough for a side to change.
                            let center = |id: &str| {
                                flow.node_terminal_rect(id).map(|r| {
                                    (
                                        ((r.0 + r.2) / 2).max(0) as u16,
                                        ((r.1 + r.3) / 2).max(0) as u16,
                                    )
                                })
                            };
                            if let (Some(sat), Some(hub)) = (center("north"), center("hub")) {
                                demo.start(
                                    demo_steps(sat, hub),
                                    (hub.0.saturating_sub(20), hub.1 + 12),
                                );
                            }
                        }
                        if let KeyOutcome::Quit = handle_key(key.code, &mut flow, &mut attach) {
                            break 'app;
                        }
                    }
                    CrosstermEvent::Mouse(mouse) => {
                        for event in flow.handle_mouse_event(mouse).into_events() {
                            match event {
                                // The handles are connectable, so dragging one
                                // draws a preview; the edge only exists if the
                                // app makes it. New edges take the attachment
                                // on screen so they behave like the rest.
                                FlowEvent::ConnectionCompleted(conn) => {
                                    flow.add_edge_from_connection(conn, DemoEdge { attach });
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
