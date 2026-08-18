//! rataflow website — single WASM binary with hash-based demo routing.

use std::cell::{Cell, RefCell};
use std::io;
use std::rc::Rc;

use ratzilla::event::{
    KeyCode as RatzillaKeyCode, KeyEvent as RatzillaKeyEvent, MouseButton as RatzillaMouseButton,
    MouseEvent as RatzillaMouseEvent, MouseEventKind as RatzillaMouseKind,
};
use ratzilla::{
    backend::webgl2::{FontAtlasConfig, WebGl2BackendOptions},
    WebGl2Backend, WebRenderer,
};
use wasm_bindgen::prelude::*;

mod demo;
mod demos;
mod overview;
mod stress_test;

use demo::Demo;
use rataflow_examples::{render_shell, ExampleMeta};

struct DemoEntry {
    demo: Box<dyn Demo>,
    meta: ExampleMeta<'static>,
}

fn create_demo(hash: &str) -> DemoEntry {
    match hash {
        "overview" | "" => overview::entry_overview(),
        "basic" => demos::entry_basic(),
        "view-only" => demos::entry_view_only(),
        "custom-nodes" => demos::entry_custom_nodes(),
        "node-flags" => demos::entry_node_flags(),
        "hierarchy" => demos::entry_hierarchy(),
        "custom-edges" => demos::entry_custom_edges(),
        "edge-routing" => demos::entry_edge_routing(),
        "floating-edges" => demos::entry_floating_edges(),
        "animating-edges" => demos::entry_animating_edges(),
        "reconnection" => demos::entry_reconnection(),
        "multi-select" => demos::entry_multi_select(),
        "context-menu" => demos::entry_context_menu(),
        "custom-bindings" => demos::entry_custom_bindings(),
        "events" => demos::entry_events(),
        "validation" => demos::entry_validation(),
        "companion-widgets" => demos::entry_companion_widgets(),
        "custom-layout" => demos::entry_custom_layout(),
        "undo-redo" => demos::entry_undo_redo(),
        "mutations" => demos::entry_mutations(),
        "theming" => demos::entry_theming(),
        "save-restore" => demos::entry_save_restore(),
        "stress-test" => stress_test::entry_stress_test(),
        _ => overview::entry_overview(),
    }
}

fn get_current_hash() -> String {
    web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default()
        .trim_start_matches('#')
        .to_string()
}

// ============================================================================
// Ratzilla → rataflow event conversion
//
// The library's From impls target the published ratzilla 0.3 API.
// The website uses ratzilla git-main which has a different event API,
// so we convert manually here.
// ============================================================================

fn convert_key(event: RatzillaKeyEvent) -> rataflow::KeyEvent {
    let code = match event.code {
        RatzillaKeyCode::Up => rataflow::KeyCode::Up,
        RatzillaKeyCode::Down => rataflow::KeyCode::Down,
        RatzillaKeyCode::Left => rataflow::KeyCode::Left,
        RatzillaKeyCode::Right => rataflow::KeyCode::Right,
        RatzillaKeyCode::Char(c) => rataflow::KeyCode::Char(c),
        RatzillaKeyCode::Delete => rataflow::KeyCode::Delete,
        RatzillaKeyCode::Backspace => rataflow::KeyCode::Backspace,
        RatzillaKeyCode::Esc => rataflow::KeyCode::Esc,
        RatzillaKeyCode::Enter => rataflow::KeyCode::Enter,
        RatzillaKeyCode::Tab => rataflow::KeyCode::Tab,
        RatzillaKeyCode::Home => rataflow::KeyCode::Home,
        RatzillaKeyCode::End => rataflow::KeyCode::End,
        RatzillaKeyCode::PageUp => rataflow::KeyCode::PageUp,
        RatzillaKeyCode::PageDown => rataflow::KeyCode::PageDown,
        RatzillaKeyCode::F(n) => rataflow::KeyCode::F(n),
        RatzillaKeyCode::Unidentified => rataflow::KeyCode::Other,
    };
    rataflow::KeyEvent {
        code,
        modifiers: rataflow::Modifiers {
            shift: event.shift,
            ctrl: event.ctrl,
            alt: event.alt,
        },
    }
}

fn convert_mouse_button(btn: RatzillaMouseButton) -> rataflow::MouseButton {
    match btn {
        RatzillaMouseButton::Left => rataflow::MouseButton::Left,
        RatzillaMouseButton::Right => rataflow::MouseButton::Right,
        RatzillaMouseButton::Middle => rataflow::MouseButton::Middle,
        _ => rataflow::MouseButton::Unknown,
    }
}

fn convert_mouse(event: &RatzillaMouseEvent) -> rataflow::MouseEvent {
    let kind = match event.kind {
        RatzillaMouseKind::ButtonDown(btn) => {
            rataflow::MouseEventKind::Down(convert_mouse_button(btn))
        }
        RatzillaMouseKind::ButtonUp(btn) => rataflow::MouseEventKind::Up(convert_mouse_button(btn)),
        _ => rataflow::MouseEventKind::Moved,
    };
    rataflow::MouseEvent {
        kind,
        column: event.col,
        row: event.row,
        modifiers: rataflow::Modifiers {
            shift: event.shift,
            ctrl: event.ctrl,
            alt: event.alt,
        },
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() -> io::Result<()> {
    let hash = get_current_hash();
    let entry: Rc<RefCell<DemoEntry>> = Rc::new(RefCell::new(create_demo(&hash)));
    let frame_count: Rc<Cell<u32>> = Rc::new(Cell::new(0));

    // Dynamic font atlas: rasterize glyphs on demand from the browser's monospace
    // font rather than blitting from beamterm's fixed static atlas. Any glyph the
    // font provides renders, so there's no need for wasm-only ASCII substitutions
    // (the static atlas omits `−`/`⊡`; dynamic renders them as-is).
    let options = WebGl2BackendOptions::new()
        .grid_id("terminal-container")
        .font_atlas_config(FontAtlasConfig::dynamic(
            &["JetBrains Mono", "Fira Code", "monospace"],
            16.0,
        ));
    let backend = WebGl2Backend::new_with_options(options)?;
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Keyboard handler
    let _ = terminal.on_key_event({
        let entry = entry.clone();
        move |key_event: RatzillaKeyEvent| {
            entry.borrow_mut().demo.handle_key(convert_key(key_event));
        }
    });

    // Track button state for drag detection since ratzilla reports moves without button info.
    let last_cell_pos: Rc<Cell<(u16, u16)>> = Rc::new(Cell::new((0, 0)));

    let _ = terminal.on_mouse_event({
        let entry = entry.clone();
        let last_cell_pos = last_cell_pos.clone();
        // Which button, not whether one: a drag means something different per
        // button (left moves a node, right draws a selection box), so reporting
        // every held-button move as a left drag would make a right-drag move
        // nodes. Only assumable while the browser owns the right button; it no
        // longer does.
        let mut button_held: Option<rataflow::MouseButton> = None;
        move |ratzilla_event: RatzillaMouseEvent| {
            match ratzilla_event.kind {
                RatzillaMouseKind::ButtonDown(btn) => {
                    button_held = Some(convert_mouse_button(btn));
                }
                RatzillaMouseKind::ButtonUp(_) => button_held = None,
                RatzillaMouseKind::SingleClick(_)
                | RatzillaMouseKind::DoubleClick(_)
                | RatzillaMouseKind::Entered
                | RatzillaMouseKind::Exited => return,
                _ => {}
            }

            last_cell_pos.set((ratzilla_event.col, ratzilla_event.row));

            let mut event = convert_mouse(&ratzilla_event);

            // Inject Drag kind when a button is held during Moved.
            // (No let-chain: this crate is still edition 2021.)
            if matches!(event.kind, rataflow::MouseEventKind::Moved) {
                if let Some(button) = button_held {
                    event.kind = rataflow::MouseEventKind::Drag(button);
                }
            }

            entry.borrow_mut().demo.handle_mouse(event);
        }
    });

    // Wheel events use the last known cell position from regular mouse events.
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(container) = document.get_element_by_id("terminal-container") {
                let entry = entry.clone();
                let last_cell_pos = last_cell_pos.clone();
                let closure =
                    Closure::<dyn Fn(web_sys::WheelEvent)>::new(move |e: web_sys::WheelEvent| {
                        e.prevent_default();
                        if e.delta_y() != 0.0 {
                            let (col, row) = last_cell_pos.get();
                            // Continuous, native-feeling zoom, normalized in rataflow.
                            let _ = entry.borrow_mut().demo.flow_ops().handle_wheel(
                                e.delta_y(),
                                e.delta_mode(),
                                col,
                                row,
                            );
                        }
                    });

                let _ = container
                    .add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref());
                closure.forget();
            }
        }
    }

    // The right button belongs to the canvas, not to the browser.
    //
    // A terminal that keeps it costs the `context_menu` demo its subject and
    // `multi_select` its box gesture, and a browser keeps it by default. Scoped
    // to the grid element, so a right-click anywhere else on the page — the
    // sidebar, a link — still gets the page menu it should.
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(container) = document.get_element_by_id("terminal-container") {
                let closure =
                    Closure::<dyn Fn(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| {
                        e.prevent_default()
                    });
                let _ = container.add_event_listener_with_callback(
                    "contextmenu",
                    closure.as_ref().unchecked_ref(),
                );
                closure.forget();
            }
        }
    }

    // Hash change handler — swap demo on navigation
    if let Some(window) = web_sys::window() {
        let entry = entry.clone();
        let frame_count = frame_count.clone();
        let closure = Closure::<dyn Fn()>::new(move || {
            let hash = get_current_hash();
            *entry.borrow_mut() = create_demo(&hash);
            frame_count.set(0);
        });

        let _ =
            window.add_event_listener_with_callback("hashchange", closure.as_ref().unchecked_ref());
        closure.forget();
    }

    // Render loop
    terminal.draw_web({
        let entry = entry.clone();
        let frame_count = frame_count.clone();
        let last_tick = Cell::new(get_performance_now());
        move |f: &mut ratatui::Frame| {
            let area = f.area();

            // Calculate elapsed_ms for tick()
            let now = get_performance_now();
            let elapsed_ms = now - last_tick.get();
            last_tick.set(now);

            let mut entry = entry.borrow_mut();

            entry.demo.tick(elapsed_ms);

            let content = render_shell(f, area, &entry.meta);
            entry.demo.render(f, content);

            // fit_view on frame 1 (need canvas size from first render)
            let count = frame_count.get();
            if count == 1 {
                entry.demo.flow_ops().request_fit_view();
            }
            frame_count.set(count.saturating_add(1));
        }
    });

    Ok(())
}

fn get_performance_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}
