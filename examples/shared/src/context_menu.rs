//! The menu model behind the `context_menu` example.
//!
//! The graph is three nodes and two edges and barely matters. What is worth
//! sharing is everything around it: which items a target offers, where the box
//! is drawn, what an item does to the graph. Left in the binary, the website
//! would need its own copy — and a second copy of a menu is a second menu, one
//! `Delete` away from behaving differently in the two places it ships.
//!
//! What stays out is the event loop. Deciding that Space opens a menu, or that
//! a click outside one dismisses it, is plumbing each frontend writes in its own
//! idiom; this module gives both of them the same menu to plumb.

use rataflow::{Edge, Flow, Node, Pick, Position, StepEdge, TextContent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Clear, Widget},
};

use crate::{ACCENT, accent_style, muted_style};

/// What the open menu acts on — determined by which event opened it.
pub enum Target {
    Node(String),
    Edge(String),
    /// World position of the click, so "add node here" lands under the cursor.
    Pane(f64, f64),
}

pub struct Menu {
    pub target: Target,
    pub items: Vec<&'static str>,
    pub selected: usize,
    /// Where the menu is drawn, in terminal cells — used for hit testing its items.
    pub rect: Rect,
}

impl Menu {
    pub fn open(target: Target, column: u16, row: u16, area: Rect) -> Self {
        let items = match target {
            Target::Node(_) => vec!["Rename", "Recolor", "Delete"],
            Target::Edge(_) => vec!["Toggle animation", "Delete"],
            Target::Pane(_, _) => vec!["Add node here", "Select all", "Clear selection"],
        };
        let width = items.iter().map(|i| i.len()).max().unwrap_or(0) as u16 + 4;
        let height = items.len() as u16 + 2;
        // Clamped against the far edge so the box stays whole, and against the
        // near one so it cannot be pushed off an area that does not start at 0.
        // Both shells put their sidebar on the right, so today only the first
        // clamp ever fires; the second is what keeps that from being load-bearing.
        let rect = Rect::new(
            column.min(area.right().saturating_sub(width)).max(area.x),
            row.min(area.bottom().saturating_sub(height)).max(area.y),
            width,
            height,
        );
        Self {
            target,
            items,
            selected: 0,
            rect,
        }
    }

    /// The item under a terminal cell, if the cell is inside the menu body.
    pub fn item_at(&self, column: u16, row: u16) -> Option<usize> {
        let inside = column > self.rect.x
            && column < self.rect.x + self.rect.width - 1
            && row > self.rect.y
            && row < self.rect.y + self.rect.height - 1;
        inside
            .then(|| (row - self.rect.y - 1) as usize)
            .filter(|index| *index < self.items.len())
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1).min(self.items.len() - 1);
    }

    pub fn render(&self, buf: &mut Buffer) {
        Clear.render(self.rect, buf);
        Block::bordered()
            .border_style(Style::default().fg(ACCENT))
            .render(self.rect, buf);

        for (i, item) in self.items.iter().enumerate() {
            let style = if i == self.selected {
                accent_style().add_modifier(Modifier::BOLD)
            } else {
                muted_style()
            };
            let width = self.rect.width as usize - 4;
            let line = Line::styled(format!(" {item:<width$} "), style);
            buf.set_line(
                self.rect.x + 1,
                self.rect.y + 1 + i as u16,
                &line,
                self.rect.width - 2,
            );
        }
    }
}

pub fn create_flow() -> Flow<TextContent, StepEdge> {
    let nodes = vec![
        Node::from_text("a", (8.0, 4.0), "Right-click me"),
        Node::from_text("b", (44.0, 4.0), "Or the edge"),
        Node::from_text("c", (26.0, 16.0), "Or the canvas"),
    ];
    let edges: Vec<Edge<StepEdge>> = vec![
        Edge::new("e1", "a", "b").with_label("edge"),
        Edge::new("e2", "a", "c"),
    ];
    Flow::with_graph(nodes, edges).expect("valid graph")
}

/// What is under a terminal cell, as a menu target.
///
/// The keyboard path both frontends need: a terminal that keeps the right button
/// for itself, or a browser that opens its own menu on it, still has to be able
/// to reach the same menu the mouse would have opened.
pub fn target_at(
    flow: &mut Flow<TextContent, StepEdge>,
    area: Rect,
    column: u16,
    row: u16,
) -> Target {
    let world = flow.viewport.canvas_to_world(Position::new(
        column.saturating_sub(area.x) as f64,
        row.saturating_sub(area.y) as f64,
    ));
    match flow.pick(world) {
        Pick::Node { node_id } | Pick::Handle { node_id, .. } => Target::Node(node_id.to_string()),
        Pick::Edge { edge_id } => Target::Edge(edge_id.to_string()),
        Pick::Nothing => Target::Pane(world.x, world.y),
    }
}

/// Runs the highlighted item and reports what it did, for the status line.
///
/// Every action changes something visible — a menu whose items appear to do
/// nothing is impossible to tell from a menu that is broken.
pub fn run(flow: &mut Flow<TextContent, StepEdge>, menu: &Menu, counter: &mut usize) -> String {
    let palette = [Color::Green, Color::Magenta, Color::Yellow, Color::Blue];
    match (&menu.target, menu.items[menu.selected]) {
        (Target::Node(id), "Rename") => {
            *counter += 1;
            let name = format!("renamed {counter}");
            if let Some(content) = flow.node_content_mut(id) {
                content.text = name.clone().into();
            }
            format!("renamed {id} to \"{name}\"")
        }
        (Target::Node(id), "Recolor") => {
            *counter += 1;
            let color = palette[*counter % palette.len()];
            if let Some(content) = flow.node_content_mut(id) {
                content.border_style = Some(Style::default().fg(color));
            }
            format!("recolored {id} to {color:?}")
        }
        (Target::Node(id), "Delete") => {
            flow.remove_node(id);
            format!("deleted node {id}")
        }
        (Target::Edge(id), "Toggle animation") => {
            let on = flow.edge(id).is_some_and(|e| e.animated);
            flow.set_edge_animated(id, !on);
            format!("edge {id} animation {}", if on { "off" } else { "on" })
        }
        (Target::Edge(id), "Delete") => {
            flow.remove_edge(id);
            format!("deleted edge {id}")
        }
        (Target::Pane(x, y), "Add node here") => {
            *counter += 1;
            let id = format!("new{counter}");
            let _ = flow.add_node(Node::from_text(id.clone(), (*x, *y), id.clone()));
            format!("added {id} at ({x:.0}, {y:.0})")
        }
        (Target::Pane(_, _), "Select all") => {
            flow.select_all_nodes();
            format!("selected {} nodes", flow.selected_nodes().count())
        }
        (Target::Pane(_, _), "Clear selection") => {
            flow.clear_selection();
            "cleared selection".to_string()
        }
        _ => String::new(),
    }
}
