use rataflow::{Flow, Palette, StepEdge, TextContent, Theme};
use ratatui::style::Color;

/// Pastel palette with soft pinks, lavenders, and warm tones.
pub const SAKURA: Palette = Palette {
    canvas_bg: Color::Indexed(225), // pale pink
    surface: Color::Indexed(217),   // light pink
    muted: Color::Indexed(175),     // mauve
    subtle: Color::Indexed(218),    // pink
    accent: Color::Indexed(168),    // rose
    text: Color::Indexed(89),       // deep magenta
    success: Color::Indexed(114),   // pale green
    error: Color::Indexed(167),     // soft red
};

pub fn theme_name(theme: &Theme) -> &'static str {
    match theme {
        Theme::Dark => "Dark",
        Theme::Light => "Light",
        Theme::Custom(_) => "Sakura",
    }
}

pub fn next_theme(current: &Theme) -> Theme {
    match current {
        Theme::Dark => Theme::Light,
        Theme::Light => Theme::Custom(SAKURA),
        Theme::Custom(_) => Theme::Dark,
    }
}

/// Apply theme to flow — built-in content types resolve from `flow.theme` at render time.
pub fn apply_theme(flow: &mut Flow<TextContent, StepEdge>, theme: Theme) {
    flow.theme = theme;
}
