//! Companion widget customization example.
//!
//! Shows the same simple graph as `basic.rs`, but focuses on customizing
//! the companion widgets: Background, Controls, and MiniMap.
//!
//! Demonstrates:
//!   - Background: cross pattern with tight gap, pastel-tinted colors
//!   - Controls: horizontal orientation, top-right, pastel-tinted style
//!   - MiniMap: bottom-left, custom size/margin, bordered, pastel-tinted style
//!
//! All color overrides use a cool pastel palette so widgets feel cohesive.
//! See `theming.rs` for full theme switching.
//!
//! Controls:
//!   - Arrow keys or Tab: navigate between nodes
//!   - h/j/k/l: pan viewport
//!   - +/-: zoom in/out
//!   - f: fit view
//!   - c: center on selected node
//!   - i: toggle interactivity lock
//!   - Delete/Backspace: delete selected
//!   - q: quit

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{
    Background, BackgroundStyle, BackgroundVariant, Controls, ControlsOrientation,
    ControlsPosition, ControlsStyle, EventResponse, FlowEvent, MiniMap, MiniMapPosition,
    MiniMapStyle, StepEdge,
};
use rataflow_examples::{ExampleMeta, render_shell};
use ratatui::{
    style::{Color, Style},
    widgets::Block,
};

// Cool pastel tint: soft lavenders and pale blues from the 256-color cube.
// 236 (dark gray) → 60 (slate blue) → 109 (muted cyan) → 146 (lavender) → 153 (pale sky)
const TINT_BASE: Color = Color::Indexed(236);
const TINT_SLATE: Color = Color::Indexed(60);
const TINT_MIST: Color = Color::Indexed(109);
const TINT_LAVENDER: Color = Color::Indexed(146);
const TINT_SKY: Color = Color::Indexed(153);

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::companion_widgets().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::basic::basic();

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

            // Cross pattern with tight spacing and pastel tint
            frame.render_widget(
                Background::new(&flow)
                    .variant(BackgroundVariant::Cross)
                    .gap(6, 3)
                    .style(
                        BackgroundStyle::default()
                            .with_bg_color(TINT_BASE)
                            .with_pattern_color(TINT_MIST),
                    ),
                area,
            );

            frame.render_widget(&mut flow, area);

            // Horizontal controls, top-right
            frame.render_widget(
                Controls::new(&flow)
                    .orientation(ControlsOrientation::Horizontal)
                    .position(ControlsPosition::TopRight)
                    .style(
                        ControlsStyle::default()
                            .with_border_style(Style::default().fg(TINT_MIST))
                            .with_button_style(Style::default().fg(TINT_SKY))
                            .with_zoom_in_char('▲')
                            .with_zoom_out_char('▼')
                            .with_fit_view_char('◇')
                            .with_lock_char('●')
                            .with_unlock_char('○'),
                    ),
                area,
            );

            // Bordered minimap, bottom-left, wider than default
            frame.render_widget(
                MiniMap::new(&flow)
                    .position(MiniMapPosition::BottomLeft)
                    .size(30, 10)
                    .margin(2)
                    .block(Block::bordered().border_style(Style::default().fg(TINT_MIST)))
                    .style(
                        MiniMapStyle::default()
                            .with_bg_color(TINT_SLATE)
                            .with_node_color(TINT_LAVENDER)
                            .with_viewport_color(TINT_MIST),
                    ),
                area,
            );
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
