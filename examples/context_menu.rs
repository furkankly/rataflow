//! A context menu per target: node, edge, and empty canvas.
//!
//! A right-click reports what was under it — `NodeContextMenu`, `EdgeContextMenu`
//! or `PaneContextMenu` — and the menu is built from that. The event arrives on
//! release, and only if the press never turned into a drag, so the same button can
//! also carry a drag gesture (see the `multi_select` example).
//!
//! Some terminals keep the right button for themselves — Warp opens its own menu
//! and never forwards the event. Space is the keyboard path, and also what a real
//! app would offer: ask `Flow::pick` what is under the cursor and open the same menu.
//!
//! Controls:
//! - Right-click a node, an edge, or empty canvas: open its menu  (or press Space)
//! - Left-click a menu item to run it, or j/k + Enter, Esc to dismiss
//! - Left-drag a node to move it, or a handle to connect two nodes
//! - q: quit

use std::{
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode,
        KeyEventKind, MouseButton, MouseEventKind,
    },
    execute,
};
use rataflow::{Background, Controls, FlowEvent, StepEdge};
use rataflow_examples::{
    ExampleMeta,
    context_menu::{Menu, Target, create_flow, run, target_at},
    render_shell, render_status,
};
use ratatui::layout::Rect;

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::context_menu().with_quit()
}

fn main() -> rataflow_examples::Result<()> {
    let mut flow = create_flow();
    let mut menu: Option<Menu> = None;
    let mut counter = 0usize;
    let mut status = String::from("right-click something, or press Space");
    // Tracked so `m` can open a menu wherever the pointer last was.
    let mut cursor: Option<(u16, u16)> = None;

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    flow.request_fit_view();
    let mut last_tick = Instant::now();
    let mut area = Rect::default();

    'app: loop {
        let now = Instant::now();
        flow.tick_auto_pan(now - last_tick);
        flow.tick_animation(now - last_tick);
        last_tick = now;

        terminal.draw(|frame| {
            area = render_shell(frame, frame.area(), &meta());
            frame.render_widget(Background::new(&flow), area);
            frame.render_widget(&mut flow, area);
            frame.render_widget(Controls::new(&flow), area);

            render_status(frame, area, &format!("last action: {status}"));

            if let Some(menu) = &menu {
                menu.render(frame.buffer_mut());
            }
        })?;

        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    // Terminals that negotiate the kitty keyboard protocol also
                    // report releases; without this every key would act twice.
                    CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                        // An open menu takes the keyboard; otherwise the flow does.
                        // Note the `else` rather than a `continue` — continuing here
                        // would skip the poll check below and block on `read()`.
                        if let Some(open) = &mut menu {
                            match key.code {
                                KeyCode::Esc => menu = None,
                                KeyCode::Up | KeyCode::Char('k') => open.select_prev(),
                                KeyCode::Down | KeyCode::Char('j') => open.select_next(),
                                KeyCode::Enter => {
                                    status = run(&mut flow, open, &mut counter);
                                    menu = None;
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') => break 'app,
                                // Fallback for terminals that swallow right-click:
                                // ask the flow what is under the cursor and open the
                                // same menu the mouse would have.
                                KeyCode::Char(' ') => {
                                    let (column, row) = cursor.unwrap_or((
                                        area.x + area.width / 2,
                                        area.y + area.height / 2,
                                    ));
                                    let target = target_at(&mut flow, area, column, row);
                                    menu = Some(Menu::open(target, column, row, area));
                                }
                                _ => {
                                    flow.handle_key_event(key);
                                }
                            }
                        }
                    }
                    CrosstermEvent::Mouse(mouse) => {
                        cursor = Some((mouse.column, mouse.row));
                        // A left click inside an open menu picks an item; anywhere
                        // else dismisses it. Either way the flow never sees it.
                        let mut consumed = false;
                        if let Some(mut open) = menu.take() {
                            if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                                if let Some(index) = open.item_at(mouse.column, mouse.row) {
                                    open.selected = index;
                                    status = run(&mut flow, &open, &mut counter);
                                }
                                // Ran or dismissed; either way the flow never sees it.
                                consumed = true;
                            } else {
                                // Not a click on the menu — leave it open.
                                menu = Some(open);
                            }
                        }

                        if consumed {
                            // The menu ate this click; the flow must not also see it.
                            if !event::poll(Duration::ZERO)? {
                                break;
                            }
                            continue;
                        }

                        let (column, row) = (mouse.column, mouse.row);
                        for event in flow.handle_mouse_event(mouse).into_events() {
                            match event {
                                FlowEvent::NodeContextMenu { node_id } => {
                                    menu =
                                        Some(Menu::open(Target::Node(node_id), column, row, area));
                                }
                                FlowEvent::EdgeContextMenu { edge_id } => {
                                    menu =
                                        Some(Menu::open(Target::Edge(edge_id), column, row, area));
                                }
                                FlowEvent::PaneContextMenu { x, y } => {
                                    menu = Some(Menu::open(Target::Pane(x, y), column, row, area));
                                }
                                // The menu is what this example is about, but the
                                // handles are connectable all the same, so a left-drag
                                // between them has to land somewhere.
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
