//! Edge path rendering (Layer 2 of edge rendering).
//!
//! This module provides:
//! - [`render_path`] - Renders any path (step or straight) to a buffer
//!
//! All functions handle clipping internally via `canvas_area`.
//!
//! # Symbol Merging
//!
//! Edge rendering uses ratatui's symbol merging to properly combine overlapping
//! box-drawing characters. This handles scenarios like:
//! - Edges crossing each other (horizontal `─` + vertical `│` → `┼`)
//! - Multiple edges sharing segments or corners

use std::collections::{HashMap, HashSet};

use ratatui::{buffer::Buffer, style::Style, symbols::merge::MergeStrategy};

use super::edge_path::Path;
use crate::state::RenderContext;
use crate::types::{EdgeMarker, Viewport};

// =============================================================================
// Animation Constants
// =============================================================================

/// Total length of the animation pattern (dash + gap) in cells.
pub(crate) const ANIMATION_PATTERN_LENGTH: usize = 3;

/// Number of visible cells in each pattern cycle (2 visible, 1 gap).
const ANIMATION_DASH_LENGTH: usize = 2;

// =============================================================================
// Corner Detection (rendering concern)
// =============================================================================

/// The type of corner at a turning point in an orthogonal path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CornerKind {
    /// Top-left corner (╭) - path goes down and right
    TopLeft,
    /// Top-right corner (╮) - path goes down and left
    TopRight,
    /// Bottom-left corner (╰) - path goes up and right
    BottomLeft,
    /// Bottom-right corner (╯) - path goes up and left
    BottomRight,
}

/// Determines the corner kind at point B given points A -> B -> C.
pub(crate) fn corner_kind_at(a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> Option<CornerKind> {
    // Direction from A to B
    let dx1 = (b.0 - a.0).signum();
    let dy1 = (b.1 - a.1).signum();

    // Direction from B to C
    let dx2 = (c.0 - b.0).signum();
    let dy2 = (c.1 - b.1).signum();

    // No corner if going in same direction
    if dx1 == dx2 && dy1 == dy2 {
        return None;
    }

    // Determine corner based on direction changes
    // Terminal coordinates: x increases right, y increases DOWN
    match ((dx1, dy1), (dx2, dy2)) {
        // ╮ TopRight: coming from left going down, or coming from bottom going left
        ((1, 0), (0, 1)) => Some(CornerKind::TopRight),
        ((0, -1), (-1, 0)) => Some(CornerKind::TopRight),

        // ╭ TopLeft: coming from right going down, or coming from bottom going right
        ((-1, 0), (0, 1)) => Some(CornerKind::TopLeft),
        ((0, -1), (1, 0)) => Some(CornerKind::TopLeft),

        // ╯ BottomRight: coming from left going up, or coming from top going left
        ((1, 0), (0, -1)) => Some(CornerKind::BottomRight),
        ((0, 1), (-1, 0)) => Some(CornerKind::BottomRight),

        // ╰ BottomLeft: coming from right going up, or coming from top going right
        ((-1, 0), (0, -1)) => Some(CornerKind::BottomLeft),
        ((0, 1), (1, 0)) => Some(CornerKind::BottomLeft),

        _ => None,
    }
}

// =============================================================================
// Edge Style
// =============================================================================

/// How an edge's line is rasterized.
///
/// The variants carry their own data, so options only apply where they mean
/// something: a braille stroke has no characters to choose, because its glyph is
/// determined by which sub-cell dots the line passes through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EdgeStroke {
    /// One character per cell.
    Chars {
        /// Character for horizontal runs.
        horizontal: char,
        /// Character for vertical runs.
        vertical: char,
        /// Corner characters `[top-left, top-right, bottom-left, bottom-right]`.
        corners: [char; 4],
    },
    /// Braille sub-cell dots — a 2x4 grid per cell.
    Braille,
}

impl Default for EdgeStroke {
    fn default() -> Self {
        Self::Chars {
            horizontal: '─',
            vertical: '│',
            corners: ['╭', '╮', '╰', '╯'],
        }
    }
}

impl EdgeStroke {
    /// Characters for this stroke, or the defaults when it has none.
    fn chars(&self) -> (char, char, [char; 4]) {
        match *self {
            Self::Chars {
                horizontal,
                vertical,
                corners,
            } => (horizontal, vertical, corners),
            // Unreachable in the character renderer: `render_path` dispatches
            // braille before any of this is read.
            Self::Braille => ('─', '│', ['╭', '╮', '╰', '╯']),
        }
    }
}

/// Visual configuration for edge rendering.
/// Style defaults to the current theme when not set.
///
/// Configures the characters and style used to draw edges: line characters,
/// corner characters, and endpoint markers.
///
/// Use [`Default::default()`] for Unicode box-drawing characters with an arrow
/// marker at the target, or [`EdgeStyle::ascii()`] for ASCII fallback.
/// Customize with builder methods.
///
/// # Example
///
/// ```no_run
/// use rataflow::{EdgeStyle, EdgeMarker};
/// use ratatui::style::{Color, Style};
///
/// // Default Unicode style (arrow at target)
/// let style = EdgeStyle::default();
///
/// // Colored edges with circle markers
/// let style = EdgeStyle::default()
///     .with_stroke_style(Style::default().fg(Color::Green))
///     .with_label_style(Style::default().fg(Color::White))
///     .with_marker_end(EdgeMarker::Circle);
///
/// // Dotted appearance (· per cell)
/// let style = EdgeStyle::dotted();
///
/// // Braille strokes (sub-cell resolution, for diagonals)
/// let style = EdgeStyle::braille();
///
/// // Custom characters and markers
/// let style = EdgeStyle::default()
///     .with_line_chars('─', '│')
///     .with_corner_chars(['╭', '╮', '╰', '╯'])
///     .with_marker_start(EdgeMarker::Circle)
///     .with_marker_end(EdgeMarker::Arrow);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeStyle {
    stroke_style: Option<Style>,
    label_style: Option<Style>,
    /// How the line is rasterized, and any characters that mode uses.
    #[cfg_attr(feature = "serde", serde(default))]
    stroke: EdgeStroke,
    marker_start: Option<EdgeMarker>,
    marker_end: Option<EdgeMarker>,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self {
            stroke_style: None,
            label_style: None,
            stroke: EdgeStroke::default(),
            marker_start: None,
            marker_end: Some(EdgeMarker::ArrowClosed),
        }
    }
}

impl EdgeStyle {
    /// Creates a style with ASCII characters (for limited terminal support).
    ///
    /// Note: For ASCII arrow markers, use `EdgeMarker::Custom('>')` etc.
    pub fn ascii() -> Self {
        Self {
            stroke: EdgeStroke::Chars {
                horizontal: '-',
                vertical: '|',
                corners: ['+', '+', '+', '+'],
            },
            ..Default::default()
        }
    }

    /// Creates a style that draws lines as dots, one per cell.
    ///
    /// Matches [`BackgroundVariant::Dots`](crate::BackgroundVariant::Dots) and
    /// ratatui's `Marker::Dot`. For a gapped solid line, animate the edge instead
    /// — see [`Edge::animated`](crate::Edge::animated).
    pub fn dotted() -> Self {
        Self {
            stroke: EdgeStroke::Chars {
                horizontal: '·',
                vertical: '·',
                corners: ['·', '·', '·', '·'],
            },
            ..Default::default()
        }
    }

    /// Creates a style that draws lines as braille strokes.
    ///
    /// Each cell carries a 2x4 dot grid, so diagonals render as continuous slopes
    /// rather than a staircase of `│`. This is the default for
    /// [`StraightEdge`](crate::StraightEdge); stepped edges are already axis-aligned
    /// and gain nothing.
    ///
    /// Carries no characters — the glyph follows from which sub-cell dots the line
    /// passes through, so there is nothing to choose. Setting one via
    /// [`with_line_chars`](Self::with_line_chars) promotes the style back to
    /// [`EdgeStroke::Chars`]. Markers, color and labels behave as usual.
    ///
    /// Crossing braille edges merge by combining dots. Braille does not merge with
    /// box-drawing characters, so a braille edge crossing a stepped one replaces
    /// that cell.
    pub fn braille() -> Self {
        Self {
            stroke: EdgeStroke::Braille,
            ..Default::default()
        }
    }

    /// Creates a style without any markers.
    pub fn without_markers() -> Self {
        Self {
            marker_start: None,
            marker_end: None,
            ..Default::default()
        }
    }

    /// Sets the style applied to the whole stroke — line, corners and markers.
    ///
    /// Applies in every [`EdgeStroke`] mode; it colors what is drawn, not how it
    /// is rasterized. `None` defers to the theme.
    pub fn with_stroke_style(mut self, stroke_style: Style) -> Self {
        self.stroke_style = Some(stroke_style);
        self
    }

    /// Returns a copy with `None` styles resolved to the provided fallbacks.
    pub(crate) fn resolved_style(self, stroke_fallback: Style, label_fallback: Style) -> Self {
        Self {
            stroke_style: self.stroke_style.or(Some(stroke_fallback)),
            label_style: self.label_style.or(Some(label_fallback)),
            ..self
        }
    }

    /// Sets the style for edge labels.
    ///
    /// By default, labels use the theme's text color. Set this to give labels
    /// a distinct color or background.
    pub fn with_label_style(mut self, label_style: Style) -> Self {
        self.label_style = Some(label_style);
        self
    }

    /// Sets both horizontal and vertical line characters.
    ///
    /// Selects character rendering: a [`Braille`](EdgeStroke::Braille) stroke is
    /// promoted to [`Chars`](EdgeStroke::Chars) so the characters take effect.
    pub fn with_line_chars(mut self, horizontal: char, vertical: char) -> Self {
        let (_, _, corners) = self.stroke.chars();
        self.stroke = EdgeStroke::Chars {
            horizontal,
            vertical,
            corners,
        };
        self
    }

    /// Sets the horizontal line character.
    ///
    /// Selects character rendering: a [`Braille`](EdgeStroke::Braille) stroke is
    /// promoted to [`Chars`](EdgeStroke::Chars) so the character takes effect.
    pub fn with_horizontal_char(mut self, ch: char) -> Self {
        let (_, vertical, corners) = self.stroke.chars();
        self.stroke = EdgeStroke::Chars {
            horizontal: ch,
            vertical,
            corners,
        };
        self
    }

    /// Sets the vertical line character.
    ///
    /// Selects character rendering: a [`Braille`](EdgeStroke::Braille) stroke is
    /// promoted to [`Chars`](EdgeStroke::Chars) so the character takes effect.
    pub fn with_vertical_char(mut self, ch: char) -> Self {
        let (horizontal, _, corners) = self.stroke.chars();
        self.stroke = EdgeStroke::Chars {
            horizontal,
            vertical: ch,
            corners,
        };
        self
    }

    /// Sets the corner characters [top-left, top-right, bottom-left, bottom-right].
    ///
    /// Selects character rendering: a [`Braille`](EdgeStroke::Braille) stroke is
    /// promoted to [`Chars`](EdgeStroke::Chars) so the characters take effect.
    pub fn with_corner_chars(mut self, chars: [char; 4]) -> Self {
        let (horizontal, vertical, _) = self.stroke.chars();
        self.stroke = EdgeStroke::Chars {
            horizontal,
            vertical,
            corners: chars,
        };
        self
    }

    /// Sets how the line is rasterized.
    pub fn with_stroke(mut self, stroke: EdgeStroke) -> Self {
        self.stroke = stroke;
        self
    }

    /// Sets the marker at the start of the edge (source end).
    pub fn with_marker_start(mut self, marker: EdgeMarker) -> Self {
        self.marker_start = Some(marker);
        self
    }

    /// Sets the marker at the end of the edge (target end).
    pub fn with_marker_end(mut self, marker: EdgeMarker) -> Self {
        self.marker_end = Some(marker);
        self
    }

    /// Removes all markers from the edge.
    pub fn with_no_markers(mut self) -> Self {
        self.marker_start = None;
        self.marker_end = None;
        self
    }

    /// Returns the corner character for the given corner kind.
    fn corner_char(&self, kind: CornerKind) -> char {
        match kind {
            CornerKind::TopLeft => self.stroke.chars().2[0],
            CornerKind::TopRight => self.stroke.chars().2[1],
            CornerKind::BottomLeft => self.stroke.chars().2[2],
            CornerKind::BottomRight => self.stroke.chars().2[3],
        }
    }
}

/// Renders an edge path to the buffer.
///
/// This is an internal function used by [`EdgeRenderContext::render_path`](crate::EdgeRenderContext::render_path).
/// Users should call `ctx.render_path(style, label, buf)` instead of this function directly.
///
/// It handles:
/// - Transforming world coordinates to terminal coordinates
/// - Applying endpoint offsets (terminal-space adjustment for handle alignment)
/// - Path segments and corners (using style chars)
/// - Markers at endpoints (if defined in style)
/// - Label at path's label_position (if provided)
///
/// The path is transformed from world to terminal coordinates, then clipped
/// to canvas bounds before rendering.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_path(
    path: &Path,
    style: &EdgeStyle,
    label: Option<&ratatui::text::Text<'_>>,
    viewport: &Viewport,
    render_ctx: &RenderContext,
    source_endpoint_offset: (i32, i32),
    target_endpoint_offset: (i32, i32),
    animation_phase: Option<usize>,
    buf: &mut Buffer,
) {
    let canvas_area = render_ctx.canvas_area;

    // Transform world path to terminal coordinates
    let mut terminal_points: Vec<(i32, i32)> = path
        .points
        .iter()
        .map(|p| render_ctx.world_to_terminal(viewport, *p))
        .collect();

    if terminal_points.len() < 2 {
        return;
    }

    // Apply endpoint offsets to first and last points
    if let Some(first) = terminal_points.first_mut() {
        first.0 += source_endpoint_offset.0;
        first.1 += source_endpoint_offset.1;
    }
    if let Some(last) = terminal_points.last_mut() {
        last.0 += target_endpoint_offset.0;
        last.1 += target_endpoint_offset.1;
    }

    // Transform label position to terminal
    let terminal_label_pos = render_ctx.world_to_terminal(viewport, path.label_position);

    // Braille draws the whole polyline at sub-cell resolution and clips per dot,
    // so it skips the polyline clipper and the corner pass entirely — a stroke
    // through a turn is already continuous. Markers and the label are shared.
    if matches!(style.stroke, EdgeStroke::Braille) {
        render_braille_path(
            path,
            style,
            viewport,
            render_ctx,
            source_endpoint_offset,
            target_endpoint_offset,
            animation_phase,
            buf,
        );
        render_markers_and_label(
            path,
            style,
            label,
            &terminal_points,
            terminal_label_pos,
            canvas_area,
            buf,
        );
        return;
    }

    // Clip to canvas bounds
    let clipped_points = clip_path_to_rect(
        &terminal_points,
        canvas_area.x as i32,
        canvas_area.y as i32,
        (canvas_area.x + canvas_area.width) as i32,
        (canvas_area.y + canvas_area.height) as i32,
    );

    if clipped_points.len() < 2 {
        return;
    }

    // First pass: identify all corner positions so we can exclude them from segment rendering.
    // This prevents a single edge's segments from merging at its own corners (which would
    // produce ┼ instead of the correct corner char).
    let mut corner_positions: HashSet<(i32, i32)> = HashSet::new();
    if clipped_points.len() >= 3 {
        for window in clipped_points.windows(3) {
            let a = window[0];
            let b = window[1];
            let c = window[2];
            if corner_kind_at(a, b, c).is_some() {
                corner_positions.insert(b);
            }
        }
    }

    // Draw line segments between consecutive points, excluding corner positions
    for window in clipped_points.windows(2) {
        let from = window[0];
        let to = window[1];
        render_segment_excluding(from, to, style, &corner_positions, animation_phase, buf);
    }

    // Draw corners at intermediate points using merge_symbol.
    // Since segments excluded these positions, corners won't conflict with their own edge's
    // segments. Using merge allows corners from different edges to combine into T-junctions
    // (e.g., two edges branching from the same point will merge ╮ + ╰ into ┤).
    if clipped_points.len() >= 3 {
        for window in clipped_points.windows(3) {
            let a = window[0];
            let b = window[1];
            let c = window[2];

            if let Some(corner_kind) = corner_kind_at(a, b, c) {
                let corner_char = style.corner_char(corner_kind);
                buf[(b.0 as u16, b.1 as u16)]
                    .merge_symbol(&corner_char.to_string(), MergeStrategy::Fuzzy)
                    .set_style(style.stroke_style.unwrap_or_default());
            }
        }
    }

    render_markers_and_label(
        path,
        style,
        label,
        &terminal_points,
        terminal_label_pos,
        canvas_area,
        buf,
    );
}

/// Draws endpoint markers and the label — shared by the character and braille
/// renderers, which differ only in the stroke between them.
fn render_markers_and_label(
    path: &Path,
    style: &EdgeStyle,
    label: Option<&ratatui::text::Text<'_>>,
    terminal_points: &[(i32, i32)],
    terminal_label_pos: (i32, i32),
    canvas_area: ratatui::layout::Rect,
    buf: &mut Buffer,
) {
    // Helper to check if a point is in bounds
    let is_in_bounds = |point: (i32, i32)| -> bool {
        point.0 >= canvas_area.x as i32
            && point.0 < (canvas_area.x + canvas_area.width) as i32
            && point.1 >= canvas_area.y as i32
            && point.1 < (canvas_area.y + canvas_area.height) as i32
    };

    // Get terminal positions of first/last (with offsets applied)
    let start_terminal = terminal_points.first().copied();
    let end_terminal = terminal_points.last().copied();

    // Draw start marker if defined and in bounds
    if let (Some(marker), Some(start)) = (style.marker_start, start_terminal)
        && is_in_bounds(start)
        && let Some(ch) = marker.char_for_position(path.source_position, true)
    {
        buf[(start.0 as u16, start.1 as u16)]
            .set_char(ch)
            .set_style(style.stroke_style.unwrap_or_default());
    }

    // Draw end marker if defined and in bounds
    if let (Some(marker), Some(end)) = (style.marker_end, end_terminal)
        && is_in_bounds(end)
        && let Some(ch) = marker.char_for_position(path.target_position, false)
    {
        buf[(end.0 as u16, end.1 as u16)]
            .set_char(ch)
            .set_style(style.stroke_style.unwrap_or_default());
    }

    // Draw label clipped to canvas bounds (right/bottom handled by set_string, left needs slicing)
    if let Some(label) = label {
        let (x, y) = terminal_label_pos;
        let lines: Vec<String> = label.lines.iter().map(|l| l.to_string()).collect();
        let total_lines = lines.len() as i32;
        let start_y = y - total_lines / 2;
        let x_min = canvas_area.x as i32;
        let y_min = canvas_area.y as i32;
        let y_max = (canvas_area.y + canvas_area.height) as i32;

        for (i, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            let line_y = start_y + i as i32;
            if line_y < y_min || line_y >= y_max {
                continue;
            }
            let char_count = line.chars().count();
            let half_width = char_count as i32 / 2;
            let line_x = x - half_width;
            // Clip left edge: skip characters that fall before x_min
            let clip_start = (x_min - line_x).max(0) as usize;
            if clip_start >= char_count {
                continue;
            }
            let clipped: String = line.chars().skip(clip_start).collect();
            if !clipped.is_empty() {
                let draw_x = line_x.max(x_min) as u16;
                buf.set_string(
                    draw_x,
                    line_y as u16,
                    &clipped,
                    style.label_style.unwrap_or_default(),
                );
            }
        }
    }
}

// =============================================================================
// Braille Strokes
// =============================================================================

/// First codepoint of the Unicode braille block; the low 8 bits are the dot mask.
const BRAILLE_BASE: u32 = 0x2800;

/// Dot bit for each sub-cell, indexed `[column][row]` in a 2-wide, 4-tall grid.
///
/// The bottom row uses 0x40/0x80 because braille was extended from 6 dots to 8 —
/// the added bits sit above the original six rather than continuing the sequence.
const BRAILLE_DOTS: [[u8; 4]; 2] = [
    [0x01, 0x02, 0x04, 0x40], // left column, rows 0..=3
    [0x08, 0x10, 0x20, 0x80], // right column, rows 0..=3
];

fn is_braille(ch: char) -> bool {
    ('\u{2800}'..='\u{28FF}').contains(&ch)
}

/// Renders a path as braille strokes at 2x4 sub-cell resolution.
///
/// Accumulates the whole polyline into a per-cell mask before writing, so a cell
/// costs one symbol parse and one write however many dots land in it. Dots are
/// OR-ed into any braille already in the buffer — that is what merges crossing
/// braille edges. See INTERNALS.md § "Braille Strokes".
#[allow(clippy::too_many_arguments)]
fn render_braille_path(
    path: &Path,
    style: &EdgeStyle,
    viewport: &Viewport,
    render_ctx: &RenderContext,
    source_endpoint_offset: (i32, i32),
    target_endpoint_offset: (i32, i32),
    animation_phase: Option<usize>,
    buf: &mut Buffer,
) {
    let mut points: Vec<(f64, f64)> = path
        .points
        .iter()
        .map(|p| render_ctx.world_to_terminal_f64(viewport, *p))
        .collect();

    if points.len() < 2 {
        return;
    }

    if let Some(first) = points.first_mut() {
        first.0 += source_endpoint_offset.0 as f64;
        first.1 += source_endpoint_offset.1 as f64;
    }
    if let Some(last) = points.last_mut() {
        last.0 += target_endpoint_offset.0 as f64;
        last.1 += target_endpoint_offset.1 as f64;
    }

    let neg_phase =
        animation_phase.map(|p| ANIMATION_PATTERN_LENGTH - (p % ANIMATION_PATTERN_LENGTH));

    let mut cells: HashMap<(i32, i32), u8> = HashMap::new();
    // Distance along the path in whole cells, so the dash pattern matches the
    // character renderer's (which counts cells, not sub-cells).
    let mut traveled = 0.0_f64;

    for window in points.windows(2) {
        let (x1, y1) = window[0];
        let (x2, y2) = window[1];
        let (dx, dy) = (x2 - x1, y2 - y1);
        let length = dx.hypot(dy);

        // Oversample against the sub-cell grid (2 columns, 4 rows per cell) at 2×
        // so a stroke can never step over a dot.
        let steps = ((dx.abs() * 2.0).max(dy.abs() * 4.0) * 2.0).ceil().max(1.0) as usize;

        for step in 0..=steps {
            let t = step as f64 / steps as f64;

            if let Some(np) = neg_phase {
                let cell_distance = (traveled + length * t) as usize;
                if (cell_distance + np) % ANIMATION_PATTERN_LENGTH >= ANIMATION_DASH_LENGTH {
                    continue;
                }
            }

            plot_braille_dot(x1 + dx * t, y1 + dy * t, render_ctx, &mut cells);
        }
        traveled += length;
    }

    let stroke_style = style.stroke_style.unwrap_or_default();
    for ((x, y), mask) in cells {
        let cell = &mut buf[(x as u16, y as u16)];
        let existing = cell.symbol().chars().next().unwrap_or(' ');
        let merged = if is_braille(existing) {
            (existing as u32 - BRAILLE_BASE) as u8 | mask
        } else {
            mask
        };
        if let Some(ch) = char::from_u32(BRAILLE_BASE + merged as u32) {
            cell.set_char(ch).set_style(stroke_style);
        }
    }
}

/// Records the dot a point falls on, clipping per dot rather than per segment.
fn plot_braille_dot(
    x: f64,
    y: f64,
    render_ctx: &RenderContext,
    cells: &mut HashMap<(i32, i32), u8>,
) {
    if !x.is_finite() || !y.is_finite() {
        return;
    }
    let cell_x = x.floor();
    let cell_y = y.floor();
    let (cx, cy) = (cell_x as i32, cell_y as i32);
    if !render_ctx.is_in_canvas(cx, cy) {
        return;
    }
    // floor() puts the fraction in [0, 1) even for negative coordinates.
    let sub_x = (((x - cell_x) * 2.0) as usize).min(1);
    let sub_y = (((y - cell_y) * 4.0) as usize).min(3);
    *cells.entry((cx, cy)).or_insert(0) |= BRAILLE_DOTS[sub_x][sub_y];
}

/// Clips a path (in terminal coordinates) to a rectangle.
///
/// Uses Cohen-Sutherland line clipping algorithm for each segment.
fn clip_path_to_rect(
    points: &[(i32, i32)],
    x_min: i32,
    y_min: i32,
    x_max: i32,
    y_max: i32,
) -> Vec<(i32, i32)> {
    if points.len() < 2 {
        return vec![];
    }

    let mut result = Vec::new();

    for window in points.windows(2) {
        let from = window[0];
        let to = window[1];

        if let Some((clipped_from, clipped_to)) = clip_segment(from, to, x_min, y_min, x_max, y_max)
        {
            if result.is_empty() || result.last() != Some(&clipped_from) {
                result.push(clipped_from);
            }
            if result.last() != Some(&clipped_to) {
                result.push(clipped_to);
            }
        }
    }

    result
}

/// Clips a single line segment to a rectangle using Cohen-Sutherland algorithm.
fn clip_segment(
    mut from: (i32, i32),
    mut to: (i32, i32),
    x_min: i32,
    y_min: i32,
    x_max: i32,
    y_max: i32,
) -> Option<((i32, i32), (i32, i32))> {
    const INSIDE: u8 = 0;
    const LEFT: u8 = 1;
    const RIGHT: u8 = 2;
    const BOTTOM: u8 = 4;
    const TOP: u8 = 8;

    let outcode = |x: i32, y: i32| -> u8 {
        let mut code = INSIDE;
        if x < x_min {
            code |= LEFT;
        } else if x >= x_max {
            code |= RIGHT;
        }
        if y < y_min {
            code |= TOP;
        } else if y >= y_max {
            code |= BOTTOM;
        }
        code
    };

    let mut outcode_from = outcode(from.0, from.1);
    let mut outcode_to = outcode(to.0, to.1);

    loop {
        if (outcode_from | outcode_to) == INSIDE {
            // Both inside
            return Some((from, to));
        }
        if (outcode_from & outcode_to) != 0 {
            // Both outside same region
            return None;
        }

        // Pick an outside point
        let outcode_out = if outcode_from != INSIDE {
            outcode_from
        } else {
            outcode_to
        };

        // Find intersection
        let (x, y) = {
            let (x1, y1) = (from.0 as f64, from.1 as f64);
            let (x2, y2) = (to.0 as f64, to.1 as f64);

            if outcode_out & TOP != 0 {
                let x = x1 + (x2 - x1) * (y_min as f64 - y1) / (y2 - y1);
                (x.round() as i32, y_min)
            } else if outcode_out & BOTTOM != 0 {
                let x = x1 + (x2 - x1) * ((y_max - 1) as f64 - y1) / (y2 - y1);
                (x.round() as i32, y_max - 1)
            } else if outcode_out & RIGHT != 0 {
                let y = y1 + (y2 - y1) * ((x_max - 1) as f64 - x1) / (x2 - x1);
                (x_max - 1, y.round() as i32)
            } else {
                // LEFT
                let y = y1 + (y2 - y1) * (x_min as f64 - x1) / (x2 - x1);
                (x_min, y.round() as i32)
            }
        };

        if outcode_out == outcode_from {
            from = (x, y);
            outcode_from = outcode(from.0, from.1);
        } else {
            to = (x, y);
            outcode_to = outcode(to.0, to.1);
        }
    }
}

/// Draws a single line segment between two points, excluding specified positions.
///
/// Uses symbol merging to combine overlapping box-drawing characters (e.g., crossing
/// edges produce `┼`). Positions in `exclude` are skipped so corners can be rendered
/// separately with proper merging.
///
/// When `animation_phase` is `Some`, cells are skipped to produce a marching ants effect.
/// The `from`→`to` ordering (source→target) determines the march direction: the phase
/// offset is negated for positive-direction segments and kept as-is for negative-direction
/// segments, so gaps always march from source toward target.
fn render_segment_excluding(
    from: (i32, i32),
    to: (i32, i32),
    style: &EdgeStyle,
    exclude: &HashSet<(i32, i32)>,
    animation_phase: Option<usize>,
    buf: &mut Buffer,
) {
    let (x1, y1) = from;
    let (x2, y2) = to;

    // Pre-convert chars to strings for merge_symbol API
    let (horizontal, vertical, _) = style.stroke.chars();
    let h_char: &str = &horizontal.to_string();
    let v_char: &str = &vertical.to_string();

    if y1 == y2 {
        // Horizontal line — negate phase when segment runs in positive-x direction
        // so gaps march source→target regardless of orientation.
        let eff_phase = animation_phase.map(|p| {
            let p = p % ANIMATION_PATTERN_LENGTH;
            if x1 <= x2 {
                ANIMATION_PATTERN_LENGTH - p
            } else {
                p
            }
        });
        let (x_min, x_max) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
        for x in x_min..=x_max {
            if exclude.contains(&(x, y1)) {
                continue;
            }
            if let Some(ep) = eff_phase
                && (x as usize + ep) % ANIMATION_PATTERN_LENGTH >= ANIMATION_DASH_LENGTH
            {
                continue;
            }
            buf[(x as u16, y1 as u16)]
                .merge_symbol(h_char, MergeStrategy::Fuzzy)
                .set_style(style.stroke_style.unwrap_or_default());
        }
    } else if x1 == x2 {
        // Vertical line — negate phase when segment runs in positive-y direction.
        let eff_phase = animation_phase.map(|p| {
            let p = p % ANIMATION_PATTERN_LENGTH;
            if y1 <= y2 {
                ANIMATION_PATTERN_LENGTH - p
            } else {
                p
            }
        });
        let (y_min, y_max) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
        for y in y_min..=y_max {
            if exclude.contains(&(x1, y)) {
                continue;
            }
            if let Some(ep) = eff_phase
                && (y as usize + ep) % ANIMATION_PATTERN_LENGTH >= ANIMATION_DASH_LENGTH
            {
                continue;
            }
            buf[(x1 as u16, y as u16)]
                .merge_symbol(v_char, MergeStrategy::Fuzzy)
                .set_style(style.stroke_style.unwrap_or_default());
        }
    } else {
        // Diagonal line - use Bresenham's algorithm
        render_diagonal(from, to, style, animation_phase, buf);
    }
}

/// Draws a diagonal line using Bresenham's algorithm.
///
/// Uses symbol merging for consistency with orthogonal line rendering.
/// When `animation_phase` is `Some`, a step counter (distance from source) determines
/// which cells are gaps, so the marching ants always flow from `from` toward `to`.
fn render_diagonal(
    from: (i32, i32),
    to: (i32, i32),
    style: &EdgeStyle,
    animation_phase: Option<usize>,
    buf: &mut Buffer,
) {
    let dx = (to.0 - from.0).abs();
    let dy = (to.1 - from.1).abs();
    let sx = if from.0 < to.0 { 1 } else { -1 };
    let sy = if from.1 < to.1 { 1 } else { -1 };

    // Choose character based on dominant direction
    let (horizontal, vertical, _) = style.stroke.chars();
    let char_for_slope = if dx > dy { horizontal } else { vertical };
    let char_str = char_for_slope.to_string();

    let neg_phase =
        animation_phase.map(|p| ANIMATION_PATTERN_LENGTH - (p % ANIMATION_PATTERN_LENGTH));

    let mut x = from.0;
    let mut y = from.1;
    let mut err = dx - dy;
    let mut step = 0usize;

    loop {
        let skip = if let Some(np) = neg_phase {
            (step + np) % ANIMATION_PATTERN_LENGTH >= ANIMATION_DASH_LENGTH
        } else {
            false
        };

        if !skip {
            buf[(x as u16, y as u16)]
                .merge_symbol(&char_str, MergeStrategy::Fuzzy)
                .set_style(style.stroke_style.unwrap_or_default());
        }

        if x == to.0 && y == to.1 {
            break;
        }

        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
        step += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HandlePosition, Position, ui::edge_path::compute_step_path};
    use ratatui::layout::Rect;

    fn pos(x: f64, y: f64) -> Position {
        Position::new(x, y)
    }

    // Create a default viewport and render context for testing
    fn test_context(canvas: Rect) -> (Viewport, RenderContext) {
        let viewport = Viewport::default();
        let render_ctx = RenderContext::new(canvas);
        (viewport, render_ctx)
    }

    #[test]
    fn test_render_path_horizontal() {
        let path = compute_step_path(
            pos(0.0, 5.0),
            pos(10.0, 5.0),
            HandlePosition::Right,
            HandlePosition::Left,
            0.0,
        );

        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 10));
        let style = EdgeStyle::default();
        let canvas = Rect::new(0, 0, 20, 10);
        let (viewport, render_ctx) = test_context(canvas);

        render_path(
            &path,
            &style,
            None,
            &viewport,
            &render_ctx,
            (0, 0),
            (0, 0),
            None,
            &mut buf,
        );

        // Check horizontal line was drawn
        assert_eq!(buf[(5, 5)].symbol(), "─");
    }

    #[test]
    fn test_render_path_with_corners() {
        let path = compute_step_path(
            pos(0.0, 0.0),
            pos(10.0, 10.0),
            HandlePosition::Right,
            HandlePosition::Left,
            0.0,
        );

        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 20));
        let style = EdgeStyle::default();
        let canvas = Rect::new(0, 0, 20, 20);
        let (viewport, render_ctx) = test_context(canvas);

        render_path(
            &path,
            &style,
            None,
            &viewport,
            &render_ctx,
            (0, 0),
            (0, 0),
            None,
            &mut buf,
        );

        // Path: (0,0) -> (5,0) -> (5,10) -> (10,10)
        // Corners at (5,0) and (5,10)
        // Check that corners are actually rendered
        assert_eq!(
            buf[(5, 0)].symbol(),
            "╮",
            "Expected top-right corner at (5,0)"
        );
        assert_eq!(
            buf[(5, 10)].symbol(),
            "╰",
            "Expected bottom-left corner at (5,10)"
        );
    }

    #[test]
    fn test_merge_symbol_on_empty_cell() {
        // Verify that merge_symbol works on fresh buffer cells
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));

        // Fresh cell should accept the symbol via merge
        buf[(5, 5)].merge_symbol("─", MergeStrategy::Fuzzy);
        assert_eq!(
            buf[(5, 5)].symbol(),
            "─",
            "merge_symbol should set symbol on empty cell"
        );
    }

    #[test]
    fn test_merge_crossing_edges() {
        // Two edges crossing should produce ┼
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        let style = Style::default();

        // Draw horizontal line
        buf[(5, 5)]
            .merge_symbol("─", MergeStrategy::Fuzzy)
            .set_style(style);
        assert_eq!(buf[(5, 5)].symbol(), "─");

        // Draw vertical line crossing it
        buf[(5, 5)]
            .merge_symbol("│", MergeStrategy::Fuzzy)
            .set_style(style);
        assert_eq!(buf[(5, 5)].symbol(), "┼", "Crossing lines should produce ┼");
    }

    #[test]
    fn test_merge_branching_corners() {
        // Two edges branching from the same point should produce T-junction
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        let style = Style::default();

        // First edge: comes from left, turns down (╮)
        buf[(5, 5)]
            .merge_symbol("╮", MergeStrategy::Fuzzy)
            .set_style(style);
        assert_eq!(buf[(5, 5)].symbol(), "╮");

        // Second edge: comes from left, turns up (╯)
        buf[(5, 5)]
            .merge_symbol("╯", MergeStrategy::Fuzzy)
            .set_style(style);
        // Should merge into a T-junction: ┤ (left input, branches up and down)
        assert_eq!(
            buf[(5, 5)].symbol(),
            "┤",
            "Branching corners should produce T-junction"
        );
    }

    #[test]
    fn test_merge_corner_with_line() {
        // A corner merging with a crossing line
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        let style = Style::default();

        // Corner: ╮ (comes from left, goes down)
        buf[(5, 5)]
            .merge_symbol("╮", MergeStrategy::Fuzzy)
            .set_style(style);
        assert_eq!(buf[(5, 5)].symbol(), "╮");

        // Horizontal line crossing through
        buf[(5, 5)]
            .merge_symbol("─", MergeStrategy::Fuzzy)
            .set_style(style);
        // Should add the right segment: ┬ (horizontal line with down branch)
        assert_eq!(
            buf[(5, 5)].symbol(),
            "┬",
            "Corner + line should produce extended junction"
        );
    }

    #[test]
    fn test_edge_marker_char_for_position() {
        use crate::types::EdgeMarker;

        // End marker (arrow pointing into target)
        assert_eq!(
            EdgeMarker::ArrowClosed.char_for_position(HandlePosition::Left, false),
            Some('▶')
        );
        assert_eq!(
            EdgeMarker::ArrowClosed.char_for_position(HandlePosition::Right, false),
            Some('◀')
        );

        // Start marker (arrow pointing away from source)
        assert_eq!(
            EdgeMarker::ArrowClosed.char_for_position(HandlePosition::Right, true),
            Some('▶')
        );

        // Non-directional markers
        assert_eq!(
            EdgeMarker::Circle.char_for_position(HandlePosition::Left, false),
            Some('●')
        );
        assert_eq!(
            EdgeMarker::Custom('★').char_for_position(HandlePosition::Left, false),
            Some('★')
        );

        // None marker
        assert_eq!(
            EdgeMarker::None.char_for_position(HandlePosition::Left, false),
            None
        );
    }

    #[test]
    fn test_corner_kind_at() {
        // Coming from left, going down -> top-right ╮
        assert_eq!(
            corner_kind_at((0, 5), (5, 5), (5, 10)),
            Some(CornerKind::TopRight)
        );

        // Coming from left, going up -> bottom-right ╯
        assert_eq!(
            corner_kind_at((0, 5), (5, 5), (5, 0)),
            Some(CornerKind::BottomRight)
        );

        // Coming from right, going down -> top-left ╭
        assert_eq!(
            corner_kind_at((10, 5), (5, 5), (5, 10)),
            Some(CornerKind::TopLeft)
        );

        // Coming from right, going up -> bottom-left ╰
        assert_eq!(
            corner_kind_at((10, 5), (5, 5), (5, 0)),
            Some(CornerKind::BottomLeft)
        );

        // Straight line -> no corner
        assert_eq!(corner_kind_at((0, 5), (5, 5), (10, 5)), None);
    }

    // --- Animation tests ---

    #[test]
    fn test_animation_gap_pattern_horizontal() {
        // Horizontal segment from (0,5) to (10,5) with phase=0
        // Pattern (3, 2): eff_phase = 3-0 = 3, gap at (x+3)%3 >= 2, i.e. x=2,5,8
        let mut buf = Buffer::empty(Rect::new(0, 0, 15, 10));
        let style = EdgeStyle::default().with_no_markers();
        let exclude = HashSet::new();

        render_segment_excluding((0, 5), (10, 5), &style, &exclude, Some(0), &mut buf);

        // Cells 0,1 drawn, 2 gap, 3,4 drawn, 5 gap, 6,7 drawn, 8 gap, 9,10 drawn
        assert_eq!(buf[(0, 5)].symbol(), "─", "cell 0 should be drawn");
        assert_eq!(buf[(1, 5)].symbol(), "─", "cell 1 should be drawn");
        assert_eq!(buf[(2, 5)].symbol(), " ", "cell 2 should be gap");
        assert_eq!(buf[(3, 5)].symbol(), "─", "cell 3 should be drawn");
        assert_eq!(buf[(5, 5)].symbol(), " ", "cell 5 should be gap");
        assert_eq!(buf[(6, 5)].symbol(), "─", "cell 6 should be drawn");
        assert_eq!(buf[(8, 5)].symbol(), " ", "cell 8 should be gap");
    }

    #[test]
    fn test_animation_phase_shifts_gap_forward() {
        // Phase is subtracted so gaps march rightward (source→target).
        // phase=1: eff_phase = 3-1 = 2, gap at (x+2)%3 >= 2, i.e. x=0,3,6,9
        let mut buf = Buffer::empty(Rect::new(0, 0, 15, 10));
        let style = EdgeStyle::default().with_no_markers();
        let exclude = HashSet::new();

        render_segment_excluding((0, 5), (10, 5), &style, &exclude, Some(1), &mut buf);

        assert_eq!(
            buf[(0, 5)].symbol(),
            " ",
            "cell 0 should be gap (shifted right)"
        );
        assert_eq!(buf[(1, 5)].symbol(), "─", "cell 1 should be drawn");
        assert_eq!(buf[(2, 5)].symbol(), "─", "cell 2 should be drawn");
        assert_eq!(
            buf[(3, 5)].symbol(),
            " ",
            "cell 3 should be gap (shifted right)"
        );
        assert_eq!(
            buf[(6, 5)].symbol(),
            " ",
            "cell 6 should be gap (shifted right)"
        );
    }

    #[test]
    fn test_animation_none_draws_all_cells() {
        // animation_phase=None should draw every cell (no gaps)
        let mut buf = Buffer::empty(Rect::new(0, 0, 15, 10));
        let style = EdgeStyle::default().with_no_markers();
        let exclude = HashSet::new();

        render_segment_excluding((0, 5), (10, 5), &style, &exclude, None, &mut buf);

        for x in 0..=10 {
            assert_eq!(
                buf[(x, 5)].symbol(),
                "─",
                "cell {x} should be drawn with no animation"
            );
        }
    }

    #[test]
    fn test_animation_preserves_corners_and_markers() {
        // Full render_path with animation — corners and markers should still render
        let path = compute_step_path(
            pos(0.0, 0.0),
            pos(10.0, 10.0),
            HandlePosition::Right,
            HandlePosition::Left,
            0.0,
        );

        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 20));
        let style = EdgeStyle::default();
        let canvas = Rect::new(0, 0, 20, 20);
        let (viewport, render_ctx) = test_context(canvas);

        render_path(
            &path,
            &style,
            None,
            &viewport,
            &render_ctx,
            (0, 0),
            (0, 0),
            Some(0),
            &mut buf,
        );

        // Corners should always render regardless of animation
        assert_eq!(
            buf[(5, 0)].symbol(),
            "╮",
            "corner should render with animation"
        );
        assert_eq!(
            buf[(5, 10)].symbol(),
            "╰",
            "corner should render with animation"
        );

        // End marker should always render
        assert_eq!(
            buf[(10, 10)].symbol(),
            "▶",
            "end marker should render with animation"
        );
    }

    #[test]
    fn test_label_clipped_off_left_edge_does_not_panic() {
        // Label positioned so it's entirely left of the canvas.
        // Previously panicked: byte index > line length in &line[clip_start..].
        let path = compute_step_path(
            pos(0.0, 5.0),
            pos(10.0, 5.0),
            HandlePosition::Right,
            HandlePosition::Left,
            0.0,
        );

        let style = EdgeStyle::default();
        // Canvas starts at x=30 — the label at midpoint ~(5,5) is entirely off-screen left
        let canvas = Rect::new(30, 0, 20, 10);
        let (viewport, render_ctx) = test_context(canvas);
        let mut buf = Buffer::empty(Rect::new(30, 0, 20, 10));

        let label = ratatui::text::Text::raw("EdgeContent trait\nstraight path");
        render_path(
            &path,
            &style,
            Some(&label),
            &viewport,
            &render_ctx,
            (0, 0),
            (0, 0),
            None,
            &mut buf,
        );
        // No panic — label lines fully clipped are skipped
    }

    // ---------------------------------------------------------------------
    // Braille strokes
    // ---------------------------------------------------------------------

    use crate::ui::edge_path::compute_straight_path;

    /// Every braille glyph written into `buf`, as (x, y, dot mask).
    fn braille_cells(buf: &Buffer, area: Rect) -> Vec<(u16, u16, u8)> {
        let mut out = Vec::new();
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let ch = buf[(x, y)].symbol().chars().next().unwrap_or(' ');
                if is_braille(ch) && ch != '\u{2800}' {
                    out.push((x, y, (ch as u32 - BRAILLE_BASE) as u8));
                }
            }
        }
        out
    }

    fn render_braille(from: Position, to: Position, canvas: Rect, buf: &mut Buffer) {
        let path = compute_straight_path(from, to, HandlePosition::Right, HandlePosition::Left);
        let (viewport, render_ctx) = test_context(canvas);
        render_path(
            &path,
            &EdgeStyle::braille().with_no_markers(),
            None,
            &viewport,
            &render_ctx,
            (0, 0),
            (0, 0),
            None,
            buf,
        );
    }

    #[test]
    fn setting_a_character_promotes_a_braille_stroke_back_to_chars() {
        // A braille stroke carries no characters, so asking for one has to change
        // the mode — the alternative is silently discarding the request.
        let promoted = EdgeStyle::braille().with_line_chars('*', '*');
        assert_eq!(
            promoted.stroke,
            EdgeStroke::Chars {
                horizontal: '*',
                vertical: '*',
                corners: ['\u{256d}', '\u{256e}', '\u{2570}', '\u{256f}'],
            },
            "corners should fall back to the defaults, not be invented"
        );

        // And it actually renders as characters afterwards.
        let canvas = Rect::new(0, 0, 20, 6);
        let mut buf = Buffer::empty(canvas);
        render_braille(pos(0.0, 3.0), pos(16.0, 3.0), canvas, &mut buf);
        let before = braille_cells(&buf, canvas).len();
        assert!(before > 0, "braille baseline should draw dots");

        let mut buf = Buffer::empty(canvas);
        let path = compute_straight_path(
            pos(0.0, 3.0),
            pos(16.0, 3.0),
            HandlePosition::Right,
            HandlePosition::Left,
        );
        let (viewport, render_ctx) = test_context(canvas);
        render_path(
            &path,
            &promoted.with_no_markers(),
            None,
            &viewport,
            &render_ctx,
            (0, 0),
            (0, 0),
            None,
            &mut buf,
        );
        assert_eq!(
            braille_cells(&buf, canvas).len(),
            0,
            "promoted style must stop drawing braille"
        );
        assert!(
            (0..canvas.width).any(|x| buf[(x, 3)].symbol() == "*"),
            "promoted style should draw the requested character"
        );
    }

    #[test]
    fn braille_stroke_is_continuous_along_a_diagonal() {
        let canvas = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(canvas);
        render_braille(pos(0.0, 0.0), pos(16.0, 8.0), canvas, &mut buf);

        let cells = braille_cells(&buf, canvas);
        assert!(!cells.is_empty(), "diagonal produced no braille");

        // Every row the stroke touches must be present — the staircase gaps that
        // whole-cell rendering leaves are exactly what braille exists to remove.
        let rows: std::collections::HashSet<u16> = cells.iter().map(|(_, y, _)| *y).collect();
        for y in 0..8u16 {
            assert!(rows.contains(&y), "row {y} has no stroke");
        }
    }

    #[test]
    fn braille_uses_subcell_resolution_not_one_dot_per_cell() {
        let canvas = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(canvas);
        // A shallow slope crosses several sub-rows within one cell row.
        render_braille(pos(0.0, 0.0), pos(16.0, 2.0), canvas, &mut buf);

        let cells = braille_cells(&buf, canvas);
        assert!(
            cells.iter().any(|(_, _, mask)| mask.count_ones() > 1),
            "no cell carries more than one dot — resolution is still whole-cell"
        );
    }

    #[test]
    fn crossing_braille_edges_merge_instead_of_overwriting() {
        let canvas = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(canvas);

        render_braille(pos(0.0, 4.0), pos(16.0, 4.0), canvas, &mut buf);
        let after_first = braille_cells(&buf, canvas);

        render_braille(pos(8.0, 0.0), pos(8.0, 8.0), canvas, &mut buf);
        let after_second = braille_cells(&buf, canvas);

        // The horizontal stroke's dots must survive under the vertical one.
        for (x, y, mask) in &after_first {
            let found = after_second
                .iter()
                .find(|(bx, by, _)| bx == x && by == y)
                .map(|(_, _, m)| *m);
            let found = found.unwrap_or_else(|| panic!("cell ({x},{y}) was erased"));
            assert_eq!(
                found & mask,
                *mask,
                "cell ({x},{y}) lost dots from the first edge"
            );
        }
        // And at least one cell gained dots from the second.
        assert!(
            after_second.iter().any(|(x, y, m)| after_first
                .iter()
                .any(|(fx, fy, fm)| fx == x && fy == y && m.count_ones() > fm.count_ones())),
            "the two strokes never actually overlapped"
        );
    }

    #[test]
    fn braille_clips_to_the_canvas_area() {
        // Canvas is offset inside a larger buffer; nothing may land outside it.
        let canvas = Rect::new(5, 2, 6, 4);
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 10));
        render_braille(pos(-40.0, -40.0), pos(40.0, 40.0), canvas, &mut buf);

        for (x, y, _) in braille_cells(&buf, Rect::new(0, 0, 20, 10)) {
            assert!(
                x >= canvas.x && x < canvas.x + canvas.width,
                "dot at x={x} escaped the canvas"
            );
            assert!(
                y >= canvas.y && y < canvas.y + canvas.height,
                "dot at y={y} escaped the canvas"
            );
        }
    }

    #[test]
    fn braille_markers_still_render() {
        let canvas = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(canvas);
        let path = compute_straight_path(
            pos(0.0, 4.0),
            pos(12.0, 4.0),
            HandlePosition::Right,
            HandlePosition::Left,
        );
        let (viewport, render_ctx) = test_context(canvas);
        render_path(
            &path,
            &EdgeStyle::braille(),
            None,
            &viewport,
            &render_ctx,
            (0, 0),
            (0, 0),
            None,
            &mut buf,
        );
        let end = buf[(12, 4)].symbol().chars().next().unwrap();
        assert!(
            !is_braille(end),
            "target marker should overwrite the braille cell, got {end:?}"
        );
    }

    #[test]
    fn world_to_terminal_f64_floors_to_world_to_terminal() {
        let render_ctx = RenderContext::new(Rect::new(5, 3, 40, 20));
        let viewport = Viewport::new(1.25, -2.75, 1.5);
        for p in [pos(0.0, 0.0), pos(7.3, -4.8), pos(-13.1, 22.9)] {
            let f = render_ctx.world_to_terminal_f64(&viewport, p);
            let i = render_ctx.world_to_terminal(&viewport, p);
            assert_eq!(
                (f.0.floor() as i32, f.1.floor() as i32),
                i,
                "mismatch at {p:?}"
            );
        }
    }
}
