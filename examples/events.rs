//! FlowEvent showcase — raw event logging.
//!
//! This example demonstrates all FlowEvent variants by logging them as they occur.
//! Interact with the graph to see events in the log panel.
//!
//! Controls:
//! - Click nodes/edges to select
//! - Drag nodes to move them
//! - Drag from handles to connect
//! - Drag empty space to pan
//! - +/- or scroll to zoom
//! - f: fit view
//! - h/j/k/l to pan with keyboard
//! - c: center on selected node
//! - i: toggle interactivity lock
//! - Delete/Backspace: delete selected
//! - q: quit

use std::{
    collections::VecDeque,
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{Background, Controls, EventResponse, FlowEvent, StepEdge};
use rataflow_examples::{ExampleMeta, render_shell};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Paragraph},
};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::events().with_quit()
}

fn log_event(log: &mut VecDeque<String>, max_entries: usize, event: &FlowEvent) {
    let message = match event {
        FlowEvent::NodeClicked { node_id } => format!("NodeClicked: {node_id}"),
        FlowEvent::NodeDragStarted { node_id } => format!("NodeDragStarted: {node_id}"),
        FlowEvent::NodeDragged { node_id } => format!("NodeDragged: {node_id}"),
        FlowEvent::NodeDragEnded { node_id } => format!("NodeDragStopped: {node_id}"),
        FlowEvent::NodeResizeStarted { node_id } => format!("NodeResizeStarted: {node_id}"),
        FlowEvent::NodeResized { node_id } => format!("NodeResized: {node_id}"),
        FlowEvent::NodeResizeEnded { node_id } => format!("NodeResizeEnded: {node_id}"),
        FlowEvent::EdgeClicked { edge_id } => format!("EdgeClicked: {edge_id}"),
        FlowEvent::PaneClicked { x, y } => format!("PaneClicked: ({x:.1}, {y:.1})"),
        FlowEvent::NodeContextMenu { node_id } => format!("NodeContextMenu: {node_id}"),
        FlowEvent::EdgeContextMenu { edge_id } => format!("EdgeContextMenu: {edge_id}"),
        FlowEvent::PaneContextMenu { x, y } => format!("PaneContextMenu: ({x:.1}, {y:.1})"),
        FlowEvent::ViewportChanged { x, y, zoom } => {
            format!("ViewportChanged: ({x:.1}, {y:.1}) z={zoom:.2}")
        }
        FlowEvent::SelectionChanged { node_ids, edge_ids } => {
            format!("SelectionChanged: nodes={node_ids:?} edges={edge_ids:?}")
        }
        FlowEvent::ConnectionStarted { node_id, handle_id } => {
            format!("ConnectionStarted: {node_id} handle={handle_id:?}")
        }
        FlowEvent::ConnectionCompleted(conn) => {
            format!("ConnectionCompleted: {} -> {}", conn.source, conn.target)
        }
        FlowEvent::ConnectionCancelled => "ConnectionCancelled".to_string(),
        FlowEvent::Deleted { node_ids, edge_ids } => {
            format!("Deleted: nodes={node_ids:?} edges={edge_ids:?}")
        }
        FlowEvent::ReconnectionStarted {
            edge_id,
            handle_type,
        } => {
            format!("ReconnectionStarted: {edge_id} ({handle_type:?})")
        }
        FlowEvent::ReconnectionCompleted {
            edge_id,
            old_connection,
            new_connection,
        } => {
            format!(
                "ReconnectionCompleted: {edge_id} {} -> {} => {} -> {}",
                old_connection.source,
                old_connection.target,
                new_connection.source,
                new_connection.target,
            )
        }
        FlowEvent::ReconnectionCancelled { edge_id } => {
            format!("ReconnectionCancelled: {edge_id}")
        }
        // `FlowEvent` is `#[non_exhaustive]`, so a wildcard is required. New
        // variants land here until this example grows an arm for them.
        other => format!("{other:?}"),
    };

    log.push_back(message);
    while log.len() > max_entries {
        log.pop_front();
    }
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::basic::basic();
    let mut event_log: VecDeque<String> = VecDeque::new();
    let mut log_max: usize = 100;

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    flow.request_fit_view();
    let mut last_tick = Instant::now();

    'app: loop {
        let now = Instant::now();
        flow.tick_auto_pan(now - last_tick);
        last_tick = now;
        terminal.draw(|frame| {
            let content = render_shell(frame, frame.area(), &meta());
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(content);

            frame.render_widget(Background::new(&flow), chunks[0]);
            frame.render_widget(&mut flow, chunks[0]);
            frame.render_widget(Controls::new(&flow), chunks[0]);

            let inner_h = chunks[1].height.saturating_sub(2) as usize;
            log_max = inner_h;
            let lines: Vec<Line> = event_log.iter().map(|s| Line::from(s.as_str())).collect();
            let scroll = lines.len().saturating_sub(inner_h) as u16;
            let log_widget = Paragraph::new(Text::from(lines))
                .block(Block::bordered().title("Events"))
                .style(
                    Style::default()
                        .fg(Color::Indexed(242))
                        .bg(Color::Indexed(232)),
                )
                .scroll((scroll, 0));
            frame.render_widget(log_widget, chunks[1]);
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    CrosstermEvent::Key(key) => match key.code {
                        KeyCode::Char('q') => break 'app,
                        _ => {
                            let response = flow.handle_controls_key_event(key);
                            if matches!(response, EventResponse::NotHandled) {
                                for event in flow.handle_key_event(key).into_events() {
                                    log_event(&mut event_log, log_max, &event);
                                }
                            } else {
                                for event in response.into_events() {
                                    log_event(&mut event_log, log_max, &event);
                                }
                            }
                        }
                    },
                    CrosstermEvent::Mouse(mouse) => {
                        for event in flow.handle_mouse_event(mouse).into_events() {
                            match &event {
                                FlowEvent::ConnectionCompleted(conn) => {
                                    flow.add_edge_from_connection(
                                        conn.clone(),
                                        StepEdge::default(),
                                    );
                                }
                                FlowEvent::ReconnectionCompleted {
                                    edge_id,
                                    new_connection,
                                    ..
                                } => {
                                    flow.reconnect_edge(edge_id, new_connection.clone());
                                }
                                _ => {}
                            }
                            log_event(&mut event_log, log_max, &event);
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
