//! Animated edges (marching ants) example.
//!
//! Demonstrates `Edge::animated` for edges with a marching ants dash pattern.
//! The animation is driven by `Flow::tick_animation()` which advances the
//! internal clock — the pattern shifts each tick, creating apparent motion.
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
//! - </> : slower/faster animation
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
use rataflow_examples::{ExampleMeta, muted_style, render_indicator, render_shell};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::animating_edges().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::animating_edges();

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    flow.request_fit_view();

    let mut last_tick = Instant::now();

    'app: loop {
        let now = Instant::now();
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

            // Speed indicator (top-right)
            let label = format!("SPEED: {:>3}ms", flow.animation_speed_ms);
            render_indicator(frame, area, &label, muted_style());
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    CrosstermEvent::Key(key) => {
                        if key.code == KeyCode::Char('q') {
                            break 'app;
                        }
                        if key.code == KeyCode::Char('<') {
                            flow.animation_speed_ms = (flow.animation_speed_ms + 20).min(500);
                        } else if key.code == KeyCode::Char('>') {
                            flow.animation_speed_ms =
                                flow.animation_speed_ms.saturating_sub(20).max(20);
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
