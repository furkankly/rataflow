//! Background widget for flow graph canvas.
//!
//! Provides a patterned background that moves with the viewport.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

use crate::content::{EdgeContent, NodeContent};
use crate::state::Flow;

/// Background pattern variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackgroundVariant {
    /// Dot pattern (·)
    #[default]
    Dots,
    /// Line pattern (─ │)
    Lines,
    /// Cross pattern (┼)
    Cross,
}

/// Visual configuration for background rendering.
/// Style defaults to the current theme when not set.
#[derive(Debug, Clone, Copy, Default)]
pub struct BackgroundStyle {
    /// Color of the pattern elements.
    pattern_color: Option<Color>,
    /// Background fill color. `Some(None)` removes the background.
    bg_color: Option<Option<Color>>,
}

impl BackgroundStyle {
    /// Returns a copy with `None` colors resolved to theme defaults.
    pub(crate) fn resolved_style(self, palette: &crate::theme::Palette) -> Self {
        Self {
            pattern_color: self.pattern_color.or(Some(palette.subtle)),
            bg_color: self.bg_color.or(Some(Some(palette.canvas_bg))),
        }
    }

    /// Sets the pattern color.
    pub fn with_pattern_color(mut self, color: Color) -> Self {
        self.pattern_color = Some(color);
        self
    }

    /// Sets the background fill color.
    pub fn with_bg_color(mut self, color: impl Into<Option<Color>>) -> Self {
        self.bg_color = Some(color.into());
        self
    }
}

/// A background pattern widget for flow graphs.
///
/// Renders a repeating pattern (dots, lines, or crosses) that
/// moves with the viewport to create the illusion of an infinite canvas.
///
/// # Example
///
/// ```no_run
/// # use ratatui::{Frame, layout::Rect};
/// # use rataflow::{Background, BackgroundVariant, Flow};
/// # fn draw(frame: &mut Frame, area: Rect, flow: &Flow) {
/// let background = Background::new(flow)
///     .variant(BackgroundVariant::Dots)
///     .gap(5, 3);
///
/// frame.render_widget(background, area);
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Background<'a, N: NodeContent, E: EdgeContent> {
    /// Reference to the flow state.
    flow: &'a Flow<N, E>,
    /// Pattern variant.
    variant: BackgroundVariant,
    /// Gap between pattern elements (x, y).
    gap: (u16, u16),
    /// Optional style override. When `None`, derived from the theme at render time.
    style: Option<BackgroundStyle>,
}

impl<'a, N: NodeContent, E: EdgeContent> Background<'a, N, E> {
    /// Creates a new Background widget.
    pub fn new(flow: &'a Flow<N, E>) -> Self {
        Self {
            flow,
            variant: BackgroundVariant::default(),
            gap: (10, 5), // Default gap: 10 horizontal, 5 vertical
            style: None,
        }
    }

    /// Sets the pattern variant.
    pub fn variant(mut self, variant: BackgroundVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the gap between pattern elements.
    pub fn gap(mut self, x: u16, y: u16) -> Self {
        self.gap = (x.max(1), y.max(1));
        self
    }

    /// Sets the style configuration, overriding theme-derived defaults.
    pub fn style(mut self, style: BackgroundStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Renders the dot pattern.
    fn render_dots(
        &self,
        area: Rect,
        buf: &mut Buffer,
        offset_x: f64,
        offset_y: f64,
        zoom: f64,
        bg_style: &BackgroundStyle,
    ) {
        let style = Style::default().fg(bg_style.pattern_color.unwrap_or_default());
        let gap_x = (self.gap.0 as f64 * zoom).max(1.0) as i32;
        let gap_y = (self.gap.1 as f64 * zoom).max(1.0) as i32;

        // Calculate offset based on viewport pan
        let off_x = (offset_x % gap_x as f64) as i32;
        let off_y = (offset_y % gap_y as f64) as i32;

        // Calculate first dot position, then step by gap
        let start_x = (-off_x).rem_euclid(gap_x);
        let start_y = (-off_y).rem_euclid(gap_y);

        // Only iterate positions that will have dots
        let mut y = start_y;
        while y < area.height as i32 {
            let mut x = start_x;
            while x < area.width as i32 {
                let px = area.x + x as u16;
                let py = area.y + y as u16;
                buf[(px, py)].set_char('·').set_style(style);
                x += gap_x;
            }
            y += gap_y;
        }
    }

    /// Renders the lines pattern.
    fn render_lines(
        &self,
        area: Rect,
        buf: &mut Buffer,
        offset_x: f64,
        offset_y: f64,
        zoom: f64,
        bg_style: &BackgroundStyle,
    ) {
        let style = Style::default().fg(bg_style.pattern_color.unwrap_or_default());
        let gap_x = (self.gap.0 as f64 * zoom).max(1.0) as i32;
        let gap_y = (self.gap.1 as f64 * zoom).max(1.0) as i32;

        // Calculate offset based on viewport pan
        let off_x = (offset_x % gap_x as f64) as i32;
        let off_y = (offset_y % gap_y as f64) as i32;

        // Calculate first line positions
        let start_x = (-off_x).rem_euclid(gap_x);
        let start_y = (-off_y).rem_euclid(gap_y);

        // Draw horizontal lines - step by gap_y
        let mut y = start_y;
        while y < area.height as i32 {
            let py = area.y + y as u16;
            for x in 0..area.width {
                buf[(area.x + x, py)].set_char('─').set_style(style);
            }
            y += gap_y;
        }

        // Draw vertical lines - step by gap_x
        let mut x = start_x;
        while x < area.width as i32 {
            let px = area.x + x as u16;
            for y in 0..area.height {
                buf[(px, area.y + y)].set_char('│').set_style(style);
            }
            x += gap_x;
        }

        // Draw intersections - step by both gaps
        let mut y = start_y;
        while y < area.height as i32 {
            let mut x = start_x;
            while x < area.width as i32 {
                let px = area.x + x as u16;
                let py = area.y + y as u16;
                buf[(px, py)].set_char('┼').set_style(style);
                x += gap_x;
            }
            y += gap_y;
        }
    }

    /// Renders the cross pattern.
    fn render_cross(
        &self,
        area: Rect,
        buf: &mut Buffer,
        offset_x: f64,
        offset_y: f64,
        zoom: f64,
        bg_style: &BackgroundStyle,
    ) {
        let style = Style::default().fg(bg_style.pattern_color.unwrap_or_default());
        let gap_x = (self.gap.0 as f64 * zoom).max(1.0) as i32;
        let gap_y = (self.gap.1 as f64 * zoom).max(1.0) as i32;

        // Calculate offset based on viewport pan
        let off_x = (offset_x % gap_x as f64) as i32;
        let off_y = (offset_y % gap_y as f64) as i32;

        // Calculate first cross position, then step by gap
        let start_x = (-off_x).rem_euclid(gap_x);
        let start_y = (-off_y).rem_euclid(gap_y);

        // Only iterate positions that will have crosses
        let mut y = start_y;
        while y < area.height as i32 {
            let mut x = start_x;
            while x < area.width as i32 {
                let px = area.x + x as u16;
                let py = area.y + y as u16;
                buf[(px, py)].set_char('┼').set_style(style);
                x += gap_x;
            }
            y += gap_y;
        }
    }
}

impl<N: NodeContent, E: EdgeContent> Widget for Background<'_, N, E> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let palette = self.flow.theme.palette();
        let style = self.style.unwrap_or_default().resolved_style(&palette);

        let offset_x = self.flow.viewport.x;
        let offset_y = self.flow.viewport.y;
        let zoom = self.flow.viewport.zoom;

        // Fill background color if specified
        if let Some(Some(bg_color)) = style.bg_color {
            for y in area.y..area.y + area.height {
                for x in area.x..area.x + area.width {
                    buf[(x, y)].set_bg(bg_color);
                }
            }
        }

        // Render pattern
        match self.variant {
            BackgroundVariant::Dots => {
                self.render_dots(area, buf, offset_x, offset_y, zoom, &style)
            }
            BackgroundVariant::Lines => {
                self.render_lines(area, buf, offset_x, offset_y, zoom, &style)
            }
            BackgroundVariant::Cross => {
                self.render_cross(area, buf, offset_x, offset_y, zoom, &style)
            }
        }
    }
}
