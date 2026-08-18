//! Static graph display with no interactivity.
//!
//! Demonstrates the minimal setup for a read-only flow graph:
//! `from_edges()` for quick graph construction, `request_fit_view()` to
//! show all nodes, and `ratatui::run()` for terminal lifecycle.
//!
//! Press 'q' to quit.

use crossterm::event::{self, Event as CrosstermEvent, KeyCode};
use rataflow::Background;
use rataflow_examples::{ExampleMeta, render_shell, view_only::create_flow};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::view_only().with_quit()
}

fn main() -> std::io::Result<()> {
    ratatui::run(run_app)
}

fn run_app(terminal: &mut ratatui::DefaultTerminal) -> std::io::Result<()> {
    let mut flow = create_flow();
    flow.request_fit_view();

    loop {
        terminal.draw(|frame| {
            let area = render_shell(frame, frame.area(), &meta());
            frame.render_widget(Background::new(&flow), area);
            frame.render_widget(&mut flow, area);
        })?;

        match event::read()? {
            CrosstermEvent::Key(key) => {
                if key.code == KeyCode::Char('q') {
                    return Ok(());
                }
            }
            CrosstermEvent::Resize(_, _) => {
                flow.request_fit_view();
            }
            _ => {}
        }
    }
}
