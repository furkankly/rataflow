//! Handle rendering for flow graphs.
//!
//! Handles are rendered by the library — there is no custom handle rendering trait.
//! Use [`HandleStyle`] on [`Handle`](crate::Handle) instances to configure their appearance.

use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::types::HandlePosition;

/// Single character or direction-aware characters for handle rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum HandleChars {
    Single(char),
    Directional {
        top: char,
        right: char,
        bottom: char,
        left: char,
    },
}

impl HandleChars {
    fn for_position(self, position: HandlePosition) -> char {
        match self {
            HandleChars::Single(ch) => ch,
            HandleChars::Directional {
                top,
                right,
                bottom,
                left,
            } => match position {
                HandlePosition::Top => top,
                HandlePosition::Right => right,
                HandlePosition::Bottom => bottom,
                HandlePosition::Left => left,
            },
        }
    }
}

/// Visual configuration for handle rendering.
/// Style defaults to the current theme when not set.
///
/// # Example
///
/// ```no_run
/// use rataflow::{Handle, HandleStyle, HandlePosition};
/// use ratatui::style::{Color, Style};
///
/// // ASCII character
/// Handle::source(HandlePosition::Right)
///     .with_style(HandleStyle::ascii());
///
/// // Explicit color
/// Handle::source(HandlePosition::Right)
///     .with_style(HandleStyle::new('◉', Style::default().fg(Color::Cyan)));
///
/// // Direction-aware characters
/// Handle::source(HandlePosition::Right)
///     .with_style(HandleStyle::directional('┴', '├', '┬', '┤'));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HandleStyle {
    chars: HandleChars,
    char_style: Option<Style>,
}

impl Default for HandleStyle {
    fn default() -> Self {
        Self {
            chars: HandleChars::Single('●'),
            char_style: None,
        }
    }
}

impl HandleStyle {
    /// Creates a new handle style with a single character and explicit style.
    pub fn new(char: char, char_style: Style) -> Self {
        Self {
            chars: HandleChars::Single(char),
            char_style: Some(char_style),
        }
    }

    /// Creates a direction-aware handle style with different chars for each direction.
    ///
    /// The rendered character depends on which side of the node the handle is on.
    pub fn directional(top: char, right: char, bottom: char, left: char) -> Self {
        Self {
            chars: HandleChars::Directional {
                top,
                right,
                bottom,
                left,
            },
            char_style: None,
        }
    }

    /// ASCII-compatible handle style (o).
    pub fn ascii() -> Self {
        Self {
            chars: HandleChars::Single('o'),
            char_style: None,
        }
    }

    /// Disabled handle style (○).
    pub fn disabled() -> Self {
        Self {
            chars: HandleChars::Single('○'),
            char_style: None,
        }
    }

    /// Sets a single handle character. Not direction-aware.
    pub fn with_char(mut self, char: char) -> Self {
        self.chars = HandleChars::Single(char);
        self
    }

    /// Sets direction-aware handle characters with different chars for each direction.
    pub fn with_directional_chars(
        mut self,
        top: char,
        right: char,
        bottom: char,
        left: char,
    ) -> Self {
        self.chars = HandleChars::Directional {
            top,
            right,
            bottom,
            left,
        };
        self
    }

    /// Sets the style applied to the handle character.
    pub fn with_char_style(mut self, char_style: Style) -> Self {
        self.char_style = Some(char_style);
        self
    }

    /// Returns the character for the given handle position.
    pub(crate) fn char_for_position(&self, position: HandlePosition) -> char {
        self.chars.for_position(position)
    }

    /// Returns a copy with `char_style` resolved to the fallback if not set.
    pub(crate) fn resolved_style(self, char_fallback: Style) -> Self {
        Self {
            char_style: self.char_style.or(Some(char_fallback)),
            ..self
        }
    }
}

/// Renders a handle marker at the given terminal position.
pub(crate) fn render_handle(
    x: i32,
    y: i32,
    canvas_area: Rect,
    style: &HandleStyle,
    position: HandlePosition,
    buf: &mut Buffer,
) {
    // Check bounds
    if x < canvas_area.x as i32
        || x >= (canvas_area.x + canvas_area.width) as i32
        || y < canvas_area.y as i32
        || y >= (canvas_area.y + canvas_area.height) as i32
    {
        return;
    }

    buf[(x as u16, y as u16)]
        .set_char(style.char_for_position(position))
        .set_style(style.char_style.unwrap_or_default());
}
