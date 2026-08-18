use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};

/// Metadata for example shell framing (optional sidebar).
#[derive(Clone)]
pub struct ExampleMeta<'a> {
    pub title: &'a str,
    /// Sidebar description. `None` skips the sidebar entirely.
    pub description: Option<&'a str>,
    /// Key-action pairs shown in the sidebar, e.g. `("q", "quit")`.
    pub keys: Vec<(&'a str, &'a str)>,
}

impl<'a> ExampleMeta<'a> {
    /// Prepend the quit binding.
    ///
    /// The shared metadata in [`crate::meta`] omits `q` deliberately: it is the
    /// one entry that is genuinely per-platform, since the wasm build runs in a
    /// browser tab with nothing to quit. Native binaries add it back here, so
    /// the difference lives in one call rather than in two copies of the list.
    pub fn with_quit(mut self) -> Self {
        self.keys.insert(0, ("q", "quit"));
        self
    }
}

/// Accent for transient overlays drawn *inside* the canvas.
///
/// The mode indicator and the status bar report the same kind of thing — what the
/// app is doing right now, on top of the graph — so they share one colour and one
/// pair of styles. A widget with its own panel (an event log, a JSON dump) is a
/// different layer and does not use these.
pub const ACCENT: Color = Color::Indexed(179);
const ACCENT_TEXT: Color = Color::Indexed(232);
const MUTED: Color = Color::Indexed(242);

const SIDEBAR_WIDTH: u16 = 32;
const MIN_WIDTH_FOR_SIDEBAR: u16 = 60;

const BG: Color = Color::Indexed(234);
const BORDER: Color = Color::Indexed(240);
const TITLE: Color = Color::Indexed(75);
const HEADING: Color = Color::Indexed(231);
const BODY: Color = Color::Indexed(250);
const KEY: Color = Color::Indexed(117);
const ACTION: Color = Color::Indexed(248);
const BRAND: Color = Color::Indexed(242);

/// Render the example shell (optional sidebar) and return the content area.
pub fn render_shell(frame: &mut Frame, area: Rect, meta: &ExampleMeta) -> Rect {
    let show_sidebar = meta.description.is_some() && area.width >= MIN_WIDTH_FOR_SIDEBAR;

    if show_sidebar {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(SIDEBAR_WIDTH)])
            .split(area);

        render_sidebar(frame, cols[1], meta);
        cols[0]
    } else {
        area
    }
}

/// Render a status indicator badge in the top-right corner of the given area.
/// Style for an active mode — readable against the accent.
pub fn accent_style() -> Style {
    Style::default().fg(ACCENT_TEXT).bg(ACCENT)
}

/// Style for an inactive mode: present, not shouting.
pub fn muted_style() -> Style {
    Style::default().fg(MUTED)
}

/// Draws a one-line status bar along the bottom of the canvas.
///
/// For "what just happened" or "what mode am I in" — the running commentary an
/// example needs when the answer is not visible in the graph itself.
pub fn render_status(frame: &mut Frame, area: Rect, text: &str) {
    let line = Line::styled(format!(" {text} "), accent_style());
    frame.buffer_mut().set_line(
        area.x,
        area.y + area.height.saturating_sub(1),
        &line,
        area.width,
    );
}

pub fn render_indicator(frame: &mut Frame, area: Rect, label: &str, style: Style) {
    let width = label.len() as u16 + 2;
    let indicator = Paragraph::new(label)
        .style(style)
        .block(Block::default().borders(Borders::ALL));
    let indicator_area = Rect::new(
        area.x + area.width.saturating_sub(width + 1),
        area.y + 1,
        width,
        3,
    );
    frame.render_widget(indicator, indicator_area);
}

fn render_sidebar(frame: &mut Frame, area: Rect, meta: &ExampleMeta) {
    let mut lines: Vec<Line> = Vec::new();

    // Description text — first line bold heading, rest body.
    if let Some(description) = meta.description {
        for (i, part) in description.split('\n').enumerate() {
            if i > 0 {
                lines.push(Line::raw(""));
            }
            let color = if i == 0 { HEADING } else { BODY };
            let mut style = Style::default().fg(color);
            if i == 0 {
                style = style.add_modifier(Modifier::BOLD);
            }
            lines.push(Line::from(Span::styled(part, style)));
        }
    }

    if meta.description.is_some() && !meta.keys.is_empty() {
        lines.push(Line::raw(""));
    }

    // Key listings.
    let max_key_width = meta.keys.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    for (key, action) in &meta.keys {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{key:>max_key_width$}"),
                Style::default().fg(KEY).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(*action, Style::default().fg(ACTION)),
        ]));
    }

    // Brand pushed to bottom.
    let inner_height = area.height.saturating_sub(4) as usize;
    if lines.len() + 1 < inner_height {
        lines.resize(inner_height - 1, Line::raw(""));
    }
    lines.push(Line::from(Span::styled(
        "rataflow",
        Style::default().fg(BRAND),
    )));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::bordered()
                .title(format!(" {} ", meta.title))
                .border_style(Style::default().fg(BORDER))
                .title_style(Style::default().fg(TITLE).add_modifier(Modifier::BOLD))
                .padding(Padding::uniform(1)),
        )
        .style(Style::default().bg(BG))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
