//! Runtime theme switching demonstration.
//!
//! Shows how to toggle between Dark, Light, and a custom pastel theme at runtime.
//! Press `t` to cycle themes — all elements update immediately.
//!
//! ## Theming
//!
//! Set `flow.theme` — built-in content types (`TextContent`, `StepEdge`, `StraightEdge`)
//! and library-rendered elements (background, controls, minimap, handles, edge preview)
//! all resolve from `flow.theme` at render time. Custom `NodeContent`/`EdgeContent`
//! implementations can read `ctx.theme.palette()` for consistent colors.
//!
//! `Theme::Custom(palette)` builds a fully custom palette (see the Sakura theme).
//!
//! Controls:
//! - t: toggle theme (Dark / Light / Sakura)
//! - Arrow keys / h/j/k/l: navigate nodes / pan
//! - +/-: zoom | f: fit view | c: center on selected
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
use rataflow::{Background, Controls, EventResponse, FlowEvent, MiniMap, StepEdge, Theme};
use rataflow_examples::{
    ExampleMeta, render_indicator, render_shell,
    theming::{apply_theme, next_theme, theme_name},
};
use ratatui::style::Style;

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::theming().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::basic::basic();

    let mut current_theme = Theme::Dark;

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

            // Theme indicator (top-right)
            let palette = current_theme.palette();
            let name = theme_name(&current_theme);
            let label = format!("THEME: {name:6}");
            let style = Style::default().fg(palette.text).bg(palette.surface);
            render_indicator(frame, area, &label, style);
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    CrosstermEvent::Key(key) => match key.code {
                        KeyCode::Char('q') => break 'app,
                        KeyCode::Char('t') => {
                            current_theme = next_theme(&current_theme);
                            apply_theme(&mut flow, current_theme);
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
