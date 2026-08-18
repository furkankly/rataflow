//! Three ways to build a selection, and what to do with one.
//!
//! A selection can be assembled one element at a time or swept in bulk:
//!
//! - **Click**, with `multi_select_mode` on, adds elements one by one.
//! - **Right-drag** draws a selection box. This is the default binding: terminals
//!   conventionally use Shift to bypass mouse reporting, so a shift-drag often
//!   never reaches the application at all, and the left button is already spoken
//!   for by node dragging.
//! - **Left-drag** draws the box instead once `selection_on_drag` is set. Dragging
//!   a node still moves it; only the gesture that starts on empty canvas changes.
//!
//! That last flag exists because no single gesture survives every terminal. Warp,
//! for one, keeps the right button for its own menu and never forwards the event —
//! so press `b` here and a left-drag draws the box instead.
//!
//! Controls:
//! - m: toggle multi-select mode, then click to add elements
//! - Right-drag: select every node the box touches
//! - b: draw the box by left-dragging instead
//! - v: select every node currently on screen, via `Flow::nodes_in`
//! - d: delete everything selected | s: fit the view to it
//! - Arrow keys / h/j/k/l: navigate nodes / pan
//! - +/-: zoom | f: fit view | c: center on selected
//! - i: toggle interactivity lock
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
use rataflow::{
    Background, Controls, EventResponse, FitViewOptions, FlowAction, FlowEvent, MiniMap, Rect,
    StepEdge, TextContent,
};
use rataflow_examples::{
    ExampleMeta, accent_style, muted_style, render_indicator, render_shell, render_status,
};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::multi_select().with_quit()
}

/// Selects every node intersecting the visible region.
///
/// `nodes_in` is a spatial query over world coordinates, so the visible region has
/// to be expressed that way too: the two canvas corners, converted.
fn select_in_view(flow: &mut rataflow::Flow<TextContent, StepEdge>) -> usize {
    let size = flow.canvas_size();
    let top_left = flow
        .viewport
        .canvas_to_world(rataflow::Position::new(0.0, 0.0));
    let bottom_right = flow
        .viewport
        .canvas_to_world(rataflow::Position::new(size.width, size.height));
    let view = Rect::from_coords(
        top_left.x,
        top_left.y,
        bottom_right.x - top_left.x,
        bottom_right.y - top_left.y,
    );

    let ids: Vec<String> = flow.nodes_in(view).map(str::to_string).collect();
    flow.clear_selection();
    for id in &ids {
        flow.toggle_node_selection(id);
    }
    ids.len()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = rataflow_examples::basic::basic();
    let mut status = String::from("nothing selected");

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

            // Multi-select indicator (top-right)
            let (text, style) = if flow.multi_select_mode {
                ("MULTI: ON ", accent_style())
            } else {
                ("MULTI: OFF", muted_style())
            };
            render_indicator(frame, area, text, style);

            // Which button draws the box, and what the last gesture caught.
            let button = if flow.selection_on_drag {
                "left-drag"
            } else {
                "right-drag"
            };
            render_status(frame, area, &format!("box: {button}  |  {status}"));
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    // Terminals that negotiate the kitty keyboard protocol also
                    // report releases; without this every key would act twice.
                    CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Char('q') => break 'app,
                        KeyCode::Char('m') => {
                            flow.multi_select_mode = !flow.multi_select_mode;
                        }
                        // Moves the box gesture off the right button, for terminals
                        // that never deliver it.
                        KeyCode::Char('b') => {
                            flow.selection_on_drag = !flow.selection_on_drag;
                        }
                        KeyCode::Char('v') => {
                            let count = select_in_view(&mut flow);
                            status = format!("selected {count} nodes in view");
                        }
                        KeyCode::Char('d') => {
                            let count = flow.selected_nodes().count();
                            flow.apply(FlowAction::Delete);
                            status = format!("deleted {count} nodes");
                        }
                        KeyCode::Char('s') => {
                            let ids: Vec<String> =
                                flow.selected_nodes().map(|n| n.id.clone()).collect();
                            status = format!("fit view to {} nodes", ids.len());
                            flow.request_fit_view_with_options(
                                FitViewOptions::default().with_nodes(ids),
                            );
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
                                FlowEvent::SelectionChanged { node_ids, edge_ids } => {
                                    status = format!(
                                        "selected {} nodes, {} edges",
                                        node_ids.len(),
                                        edge_ids.len()
                                    );
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
