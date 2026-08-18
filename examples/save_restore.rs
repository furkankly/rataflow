//! Save/restore flow graph with serde.
//!
//! Demonstrates serializing and deserializing a flow graph snapshot
//! using [`Flow::to_snapshot`] and [`Flow::from_snapshot`].
//! The JSON panel on the right shows the serialized snapshot.
//!
//! Controls:
//! - s: save current graph to JSON (stored in memory)
//! - r: restore graph from saved JSON
//! - Drag nodes to move them between save/restore to see it work
//! - Arrow keys / h/j/k/l: navigate / pan
//! - +/-: zoom | f: fit view | c: center on selected
//! - i: toggle interactivity lock
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
use rataflow_examples::{
    ExampleMeta, render_shell,
    save_restore::{pretty_json, restore, save},
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Paragraph},
};

fn meta(saved: bool) -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::save_restore(saved).with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::basic::basic();
    let mut saved_json: Option<String> = None;

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    flow.request_fit_view();
    let mut last_tick = Instant::now();

    'app: loop {
        let now = Instant::now();
        flow.tick_auto_pan(now - last_tick);
        last_tick = now;
        let has_save = saved_json.is_some();
        terminal.draw(|frame| {
            let content = render_shell(frame, frame.area(), &meta(has_save));
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
                .split(content);

            frame.render_widget(Background::new(&flow), chunks[0]);
            frame.render_widget(&mut flow, chunks[0]);
            frame.render_widget(Controls::new(&flow), chunks[0]);
            frame.render_widget(MiniMap::new(&flow), chunks[0]);

            let json_text = saved_json
                .as_deref()
                .map(pretty_json)
                .unwrap_or_else(|| "No snapshot saved yet.\nPress 's' to save.".to_string());

            let lines: Vec<Line> = json_text
                .lines()
                .map(|l| Line::from(l.to_string()))
                .collect();
            let inner_h = chunks[1].height.saturating_sub(2) as usize;
            let scroll = lines.len().saturating_sub(inner_h) as u16;
            let json_widget = Paragraph::new(Text::from(lines))
                .block(Block::bordered().title("JSON Snapshot"))
                .style(
                    Style::default()
                        .fg(Color::Indexed(242))
                        .bg(Color::Indexed(232)),
                )
                .scroll((scroll, 0));
            frame.render_widget(json_widget, chunks[1]);
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    CrosstermEvent::Key(key) => match key.code {
                        KeyCode::Char('q') => break 'app,
                        KeyCode::Char('s') => {
                            saved_json = Some(save(&flow));
                        }
                        KeyCode::Char('r') => {
                            if let Some(ref json) = saved_json
                                && let Some(restored) = restore(json)
                            {
                                flow = restored;
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
