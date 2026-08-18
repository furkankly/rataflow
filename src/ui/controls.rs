//! Controls widget for flow graph viewport manipulation.
//!
//! Provides a panel with buttons for zoom in, zoom out, fit view, and lock/unlock
//! interactivity.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::content::{EdgeContent, NodeContent};
use crate::state::Flow;

/// Position of the controls panel within the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlsPosition {
    /// Top-left corner.
    TopLeft,
    /// Top-right corner.
    TopRight,
    /// Bottom-left corner.
    #[default]
    BottomLeft,
    /// Bottom-right corner.
    BottomRight,
}

/// Orientation of the control buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlsOrientation {
    /// Buttons stacked vertically.
    #[default]
    Vertical,
    /// Buttons arranged horizontally.
    Horizontal,
}

/// Visual configuration for controls rendering.
/// Style defaults to the current theme when not set.
#[derive(Debug, Clone, Copy)]
pub struct ControlsStyle {
    /// Style for the panel border.
    border_style: Option<Style>,
    /// Style for button text.
    button_style: Option<Style>,
    /// Style for keyboard hints.
    hint_style: Option<Style>,
    /// Style for disabled buttons.
    disabled_style: Option<Style>,
    /// Whether to show keyboard hints.
    show_hints: bool,
    /// Character for zoom in button.
    zoom_in_char: char,
    /// Character for zoom out button.
    zoom_out_char: char,
    /// Character for fit view button.
    fit_view_char: char,
    /// Character for lock button (locked state).
    lock_char: char,
    /// Character for lock button (unlocked state).
    unlock_char: char,
}

impl Default for ControlsStyle {
    fn default() -> Self {
        Self {
            border_style: None,
            button_style: None,
            hint_style: None,
            disabled_style: None,
            show_hints: true,
            zoom_in_char: '+',
            zoom_out_char: '−',
            fit_view_char: '⊡',
            lock_char: '■',
            unlock_char: '□',
        }
    }
}

impl ControlsStyle {
    /// Returns a copy with `None` styles resolved to theme defaults.
    pub(crate) fn resolved_style(self, palette: &crate::theme::Palette) -> Self {
        Self {
            border_style: self
                .border_style
                .or(Some(Style::default().fg(palette.muted))),
            button_style: self
                .button_style
                .or(Some(Style::default().fg(palette.text))),
            hint_style: self.hint_style.or(Some(Style::default().fg(palette.muted))),
            disabled_style: self
                .disabled_style
                .or(Some(Style::default().fg(palette.muted))),
            ..self
        }
    }

    /// Sets the border style.
    pub fn with_border_style(mut self, style: Style) -> Self {
        self.border_style = Some(style);
        self
    }

    /// Sets the button text style.
    pub fn with_button_style(mut self, style: Style) -> Self {
        self.button_style = Some(style);
        self
    }

    /// Sets the keyboard hint style.
    pub fn with_hint_style(mut self, style: Style) -> Self {
        self.hint_style = Some(style);
        self
    }

    /// Sets the disabled button style.
    pub fn with_disabled_style(mut self, style: Style) -> Self {
        self.disabled_style = Some(style);
        self
    }

    /// Sets whether to show keyboard hints.
    pub fn with_show_hints(mut self, show: bool) -> Self {
        self.show_hints = show;
        self
    }

    /// Sets the zoom in button character.
    pub fn with_zoom_in_char(mut self, ch: char) -> Self {
        self.zoom_in_char = ch;
        self
    }

    /// Sets the zoom out button character.
    pub fn with_zoom_out_char(mut self, ch: char) -> Self {
        self.zoom_out_char = ch;
        self
    }

    /// Sets the fit view button character.
    pub fn with_fit_view_char(mut self, ch: char) -> Self {
        self.fit_view_char = ch;
        self
    }

    /// Sets the lock button character (locked state).
    pub fn with_lock_char(mut self, ch: char) -> Self {
        self.lock_char = ch;
        self
    }

    /// Sets the lock button character (unlocked state).
    pub fn with_unlock_char(mut self, ch: char) -> Self {
        self.unlock_char = ch;
        self
    }
}

/// A controls panel widget for viewport manipulation.
///
/// Displays buttons for:
/// - Zoom in (+)
/// - Zoom out (−)
/// - Fit view (⊡)
/// - Lock/unlock interactivity (■/□)
///
/// # Example
///
/// ```no_run
/// # use ratatui::{Frame, layout::Rect};
/// # use rataflow::{Controls, ControlsOrientation, ControlsPosition, Flow};
/// # fn draw(frame: &mut Frame, area: Rect, flow: &Flow) {
/// let controls = Controls::new(flow)
///     .position(ControlsPosition::BottomLeft)
///     .orientation(ControlsOrientation::Vertical);
///
/// frame.render_widget(controls, area);
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Controls<'a, N: NodeContent, E: EdgeContent> {
    /// Reference to the flow state.
    flow: &'a Flow<N, E>,
    /// Position of the panel.
    position: ControlsPosition,
    /// Orientation of the buttons.
    orientation: ControlsOrientation,
    /// Optional style override. When `None`, derived from the theme at render time.
    style: Option<ControlsStyle>,
    /// Whether to show zoom controls.
    show_zoom: bool,
    /// Whether to show fit view button.
    show_fit_view: bool,
    /// Whether to show lock button.
    show_lock: bool,
    /// Optional block wrapper.
    block: Option<Block<'a>>,
}

impl<'a, N: NodeContent, E: EdgeContent> Controls<'a, N, E> {
    /// Creates a new Controls widget.
    pub fn new(flow: &'a Flow<N, E>) -> Self {
        Self {
            flow,
            position: ControlsPosition::default(),
            orientation: ControlsOrientation::default(),
            style: None,
            show_zoom: true,
            show_fit_view: true,
            show_lock: true,
            block: None,
        }
    }

    /// Sets the position of the controls panel.
    pub fn position(mut self, position: ControlsPosition) -> Self {
        self.position = position;
        self
    }

    /// Sets the orientation of the control buttons.
    pub fn orientation(mut self, orientation: ControlsOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets the style configuration, overriding theme-derived defaults.
    pub fn style(mut self, style: ControlsStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets whether to show zoom controls.
    pub fn show_zoom(mut self, show: bool) -> Self {
        self.show_zoom = show;
        self
    }

    /// Sets whether to show the fit view button.
    pub fn show_fit_view(mut self, show: bool) -> Self {
        self.show_fit_view = show;
        self
    }

    /// Sets whether to show the lock button.
    pub fn show_lock(mut self, show: bool) -> Self {
        self.show_lock = show;
        self
    }

    /// Sets an optional block wrapper.
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Computes the required size for the controls panel.
    fn compute_size(&self, style: &ControlsStyle) -> (u16, u16) {
        let button_count = self.button_count();

        match self.orientation {
            ControlsOrientation::Vertical => {
                // Each button is one line, width depends on hints
                let width = if style.show_hints { 7 } else { 3 }; // " + " or " + [+]"
                let height = button_count as u16;
                (width + 2, height + 2) // +2 for borders
            }
            ControlsOrientation::Horizontal => {
                // Buttons side by side
                let width_per_button = if style.show_hints { 5 } else { 3 };
                let width = button_count as u16 * width_per_button;
                (width + 2, 3) // +2 for borders, height 3 (1 content + 2 borders)
            }
        }
    }

    fn button_count(&self) -> usize {
        let mut count = 0;
        if self.show_zoom {
            count += 2; // + and -
        }
        if self.show_fit_view {
            count += 1;
        }
        if self.show_lock {
            count += 1;
        }
        count
    }

    /// Computes the position of the controls panel within the given area.
    fn compute_rect(&self, area: Rect, style: &ControlsStyle) -> Rect {
        let (width, height) = self.compute_size(style);
        let margin = 1;

        let x = match self.position {
            ControlsPosition::TopLeft | ControlsPosition::BottomLeft => area.x + margin,
            ControlsPosition::TopRight | ControlsPosition::BottomRight => {
                area.x + area.width.saturating_sub(width + margin)
            }
        };

        let y = match self.position {
            ControlsPosition::TopLeft | ControlsPosition::TopRight => area.y + margin,
            ControlsPosition::BottomLeft | ControlsPosition::BottomRight => {
                area.y + area.height.saturating_sub(height + margin)
            }
        };

        Rect::new(x, y, width.min(area.width), height.min(area.height))
    }

    /// Creates the button content.
    fn render_buttons(
        &self,
        can_zoom_in: bool,
        can_zoom_out: bool,
        is_locked: bool,
        style: &ControlsStyle,
        buf: &mut Buffer,
        inner: Rect,
    ) {
        let buttons = self.collect_buttons(can_zoom_in, can_zoom_out, is_locked, style);

        match self.orientation {
            ControlsOrientation::Vertical => {
                for (i, &(icon, hint, enabled)) in buttons.iter().enumerate() {
                    if i as u16 >= inner.height {
                        break;
                    }
                    let y = inner.y + i as u16;
                    self.render_button_line(
                        style,
                        buf,
                        inner.x,
                        y,
                        inner.width,
                        icon,
                        hint,
                        enabled,
                    );
                }
            }
            ControlsOrientation::Horizontal => {
                let mut x = inner.x;
                for (icon, hint, enabled) in buttons {
                    let width = if style.show_hints { 5 } else { 3 };
                    if x + width > inner.x + inner.width {
                        break;
                    }
                    self.render_button_cell(style, buf, x, inner.y, icon, hint, enabled);
                    x += width;
                }
            }
        }
    }

    fn collect_buttons(
        &self,
        can_zoom_in: bool,
        can_zoom_out: bool,
        is_locked: bool,
        style: &ControlsStyle,
    ) -> Vec<(char, &'static str, bool)> {
        let mut buttons = Vec::new();

        if self.show_zoom {
            buttons.push((style.zoom_in_char, "+", can_zoom_in));
            buttons.push((style.zoom_out_char, "-", can_zoom_out));
        }
        if self.show_fit_view {
            buttons.push((style.fit_view_char, "f", true));
        }
        if self.show_lock {
            let icon = if is_locked {
                style.lock_char
            } else {
                style.unlock_char
            };
            buttons.push((icon, "i", true));
        }

        buttons
    }

    #[allow(clippy::too_many_arguments)]
    fn render_button_line(
        &self,
        style: &ControlsStyle,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        width: u16,
        icon: char,
        hint: &str,
        enabled: bool,
    ) {
        let char_style = if enabled {
            style.button_style.unwrap_or_default()
        } else {
            style.disabled_style.unwrap_or_default()
        };

        let content = if style.show_hints {
            format!(" {} [{}]", icon, hint)
        } else {
            format!(" {} ", icon)
        };

        let line = Line::from(vec![Span::styled(content, char_style)]);

        buf.set_line(x, y, &line, width);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_button_cell(
        &self,
        style: &ControlsStyle,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        icon: char,
        _hint: &str,
        enabled: bool,
    ) {
        let char_style = if enabled {
            style.button_style.unwrap_or_default()
        } else {
            style.disabled_style.unwrap_or_default()
        };

        let content = format!(" {} ", icon);
        buf.set_string(x, y, &content, char_style);
    }
}

impl<N: NodeContent, E: EdgeContent> Widget for Controls<'_, N, E> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 5 || area.height < 3 {
            return;
        }

        let palette = self.flow.theme.palette();
        let style = self.style.unwrap_or_default().resolved_style(&palette);

        // Compute button states inline from flow
        let can_zoom_in = self.flow.viewport.zoom < self.flow.max_zoom;
        let can_zoom_out = self.flow.viewport.zoom > self.flow.min_zoom;
        let is_locked = self.flow.locked;

        let panel_rect = self.compute_rect(area, &style);

        // Render block/border
        let inner = if let Some(block) = &self.block {
            let inner = block.inner(panel_rect);
            block.clone().render(panel_rect, buf);
            inner
        } else {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(style.border_style.unwrap_or_default());
            let inner = block.inner(panel_rect);
            block.render(panel_rect, buf);
            inner
        };

        // Render buttons
        self.render_buttons(can_zoom_in, can_zoom_out, is_locked, &style, buf, inner);
    }
}
