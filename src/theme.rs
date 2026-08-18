//! Theme system for consistent color defaults across all widgets.
//!
//! Provides [`Theme`] (Dark/Light) and [`Palette`] (8 semantic colors).
//! Set [`Flow::theme`](crate::Flow) to switch all library-rendered
//! elements (background, controls, minimap, handles, edge preview) at once.

use ratatui::style::Color;

/// Color theme for all style defaults.
///
/// Selects between predefined color palettes. Set on [`Flow::theme`](crate::Flow)
/// to switch all elements at once — built-in content types and library-rendered
/// elements (background, controls, minimap, handles, edge preview) all resolve
/// from `flow.theme` at render time. Custom content implementations can read
/// `ctx.theme.palette()` for consistent colors.
///
/// # Example
///
/// ```
/// use rataflow::{Flow, Theme};
///
/// let mut flow: Flow = Flow::new().with_theme(Theme::Light);
/// // All elements now use light colors — no per-content apply_theme needed.
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Theme {
    /// Dark theme (default). Light text on dark backgrounds.
    #[default]
    Dark,
    /// Light theme. Dark text on light backgrounds.
    Light,
    /// Custom theme with a user-provided palette.
    ///
    /// Start from a predefined palette and override individual colors:
    ///
    /// ```
    /// use rataflow::{Flow, Theme};
    /// use ratatui::style::Color;
    ///
    /// let mut palette = Theme::Dark.palette();
    /// palette.accent = Color::Cyan;
    /// palette.muted = Color::DarkGray;
    /// let flow: Flow = Flow::new().with_theme(Theme::Custom(palette));
    /// ```
    Custom(Palette),
}

impl Theme {
    /// Returns the color palette for this theme.
    pub const fn palette(&self) -> Palette {
        match self {
            Theme::Dark => Palette::DARK,
            Theme::Light => Palette::LIGHT,
            Theme::Custom(palette) => *palette,
        }
    }
}

/// Semantic color palette used by all style types.
///
/// Each field maps to a visual role rather than a specific widget, so the same
/// palette produces consistent colors across edges, handles, nodes, and
/// companion widgets.
///
/// Use [`Theme::palette()`] to get a predefined palette. Custom
/// [`NodeContent`](crate::NodeContent) / [`EdgeContent`](crate::EdgeContent)
/// implementations can read `flow.theme.palette()` to access the same semantic
/// colors the library uses internally, keeping their rendering consistent with
/// the active theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Palette {
    /// Canvas background fill color.
    pub canvas_bg: Color,
    /// Surface color for node backgrounds, minimap background.
    pub surface: Color,
    /// Muted color for borders, edges, disabled elements.
    pub muted: Color,
    /// Subtle color for ambient visual indicators (background patterns, minimap viewport fill).
    pub subtle: Color,
    /// Accent color for handles, selected borders/edges.
    pub accent: Color,
    /// Text color for node labels, button text.
    pub text: Color,
    /// Success/valid feedback color (e.g., valid connection target).
    pub success: Color,
    /// Error/invalid feedback color (e.g., invalid connection target).
    pub error: Color,
}

impl Palette {
    /// Dark palette — light text on dark backgrounds.
    pub const DARK: Self = Self {
        canvas_bg: Color::Indexed(233),
        surface: Color::Indexed(234),
        muted: Color::Indexed(237),
        subtle: Color::Indexed(240),
        accent: Color::Indexed(248),
        text: Color::Indexed(231),
        success: Color::Indexed(71),
        error: Color::Indexed(167),
    };

    /// Light palette — dark text on light backgrounds.
    pub const LIGHT: Self = Self {
        canvas_bg: Color::Indexed(254),
        surface: Color::Indexed(231),
        muted: Color::Indexed(249),
        subtle: Color::Indexed(252),
        accent: Color::Indexed(240),
        text: Color::Indexed(233),
        success: Color::Indexed(28),
        error: Color::Indexed(124),
    };
}
