//! Built-in node and edge content types provided by the library.
//!
//! These types implement [`NodeContent`] and [`EdgeContent`] traits and can be used
//! directly for simple use cases, or as references for implementing custom types.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Text;
use ratatui::widgets::{Block, BorderType, Widget};

use crate::content::{
    EdgeContent, EdgePathContext, EdgeRenderContext, NodeContent, NodeRenderContext,
};
use crate::theme::Theme;
use crate::types::{HandlePosition, Node, Position};

use super::edge_path::{Path, compute_step_path, compute_straight_path};
use super::edge_render::EdgeStyle;

/// Renders centered text inside a node frame.
///
/// This is an internal helper for [`TextContent`]. The text is centered both
/// horizontally and vertically within the inner area (accounting for borders).
fn render_node_text(area: Rect, lines: &[String], text_style: Style, buf: &mut Buffer) {
    if area.height < 3 || area.width < 3 || lines.is_empty() {
        return;
    }

    let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);
    let max_lines = inner.height as usize;
    let lines_to_draw = lines.len().min(max_lines);

    // Calculate starting y to center the lines vertically
    let start_y = inner.y + (inner.height.saturating_sub(lines_to_draw as u16)) / 2;

    for (i, line) in lines.iter().take(lines_to_draw).enumerate() {
        let y = start_y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }

        let label_x = inner.x + (inner.width.saturating_sub(line.len() as u16)) / 2;
        let truncated: String = line.chars().take(inner.width as usize).collect();
        buf.set_string(label_x, y, &truncated, text_style);
    }
}

/// Serde helper for `Text<'static>` — serializes as a plain string.
///
/// `Text` is a ratatui display type with styled spans, but for serialization
/// we only need the raw text content.
#[cfg(feature = "serde")]
mod text_serde {
    use ratatui::text::Text;

    pub fn serialize<S: serde::Serializer>(
        text: &Text<'static>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&text.to_string())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Text<'static>, D::Error> {
        use serde::Deserialize;
        let s = String::deserialize(de)?;
        Ok(Text::from(s))
    }
}

/// Built-in node type that displays bordered text.
///
/// Renders as a bordered box with centered text.
/// Use [`Node::from_text`] to create nodes with auto-calculated dimensions.
///
/// # Styling
///
/// Style fields are `Option` — `None` uses the current theme, `Some` overrides.
///
/// # Example
///
/// ```no_run
/// use rataflow::{Node, TextContent, Position};
/// use ratatui::style::{Color, Style};
///
/// // Single line
/// let node = Node::from_text("n1", (10.0, 20.0), "Hello");
///
/// // Multiple lines
/// let node = Node::from_text("n2", (10.0, 30.0), "Line 1\nLine 2");
///
/// // With custom style override
/// let content = TextContent::from("Custom")
///     .with_border_style(Style::default().fg(Color::Green));
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextContent {
    /// The text to display.
    #[cfg_attr(feature = "serde", serde(with = "text_serde"))]
    pub text: Text<'static>,
    /// Optional title shown in the border.
    pub title: Option<String>,

    /// Style for the node border.
    pub border_style: Option<Style>,
    /// Style for the text content.
    pub text_style: Option<Style>,
    /// Background color. `Some(None)` removes background.
    pub background: Option<Option<Color>>,

    /// Style for the node border when selected.
    pub selected_border_style: Option<Style>,
    /// Style for the text content when selected.
    pub selected_text_style: Option<Style>,
    /// Background color when selected. `Some(None)` removes background.
    pub selected_background: Option<Option<Color>>,
}

impl TextContent {
    /// Creates new text content from anything convertible to `Text`.
    pub fn new(text: impl Into<Text<'static>>) -> Self {
        Self {
            text: text.into(),
            title: None,
            border_style: None,
            text_style: None,
            background: None,
            selected_border_style: None,
            selected_text_style: None,
            selected_background: None,
        }
    }

    /// Resolves styles for the current state, using theme defaults for `None` fields.
    fn resolve_styles(&self, theme: Theme, selected: bool) -> (Style, Style, Option<Color>) {
        let palette = theme.palette();
        if selected {
            (
                self.selected_border_style.unwrap_or_else(|| {
                    Style::default()
                        .fg(palette.accent)
                        .add_modifier(Modifier::BOLD)
                }),
                self.selected_text_style
                    .unwrap_or_else(|| Style::default().fg(palette.text)),
                self.selected_background.unwrap_or(Some(palette.surface)),
            )
        } else {
            (
                self.border_style
                    .unwrap_or_else(|| Style::default().fg(palette.muted)),
                self.text_style
                    .unwrap_or_else(|| Style::default().fg(palette.text)),
                self.background.unwrap_or(Some(palette.surface)),
            )
        }
    }

    /// Sets the title shown in the border.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Returns the first line of text as a string, or an empty string if there are no lines.
    pub fn label(&self) -> String {
        self.text
            .lines
            .first()
            .map(|l| l.to_string())
            .unwrap_or_default()
    }

    /// Returns the width needed to display this text (max line width).
    pub fn width(&self) -> usize {
        self.text.width()
    }

    /// Returns the height needed to display this text (number of lines).
    pub fn height(&self) -> usize {
        self.text.height().max(1) // At least 1 for empty content
    }

    /// Sets the border style for normal state.
    pub fn with_border_style(mut self, style: Style) -> Self {
        self.border_style = Some(style);
        self
    }

    /// Sets the text style for normal state.
    pub fn with_text_style(mut self, style: Style) -> Self {
        self.text_style = Some(style);
        self
    }

    /// Sets the background color for normal state.
    ///
    /// Pass `None` to remove the background — use this on parent/group nodes
    /// together with [`Node::with_opaque(false)`](crate::Node::with_opaque)
    /// so edges and children remain visible inside the node.
    pub fn with_background(mut self, color: impl Into<Option<Color>>) -> Self {
        self.background = Some(color.into());
        self
    }

    /// Sets the border style for selected state.
    pub fn with_selected_border_style(mut self, style: Style) -> Self {
        self.selected_border_style = Some(style);
        self
    }

    /// Sets the text style for selected state.
    pub fn with_selected_text_style(mut self, style: Style) -> Self {
        self.selected_text_style = Some(style);
        self
    }

    /// Sets the background color for selected state.
    ///
    /// Pass `None` to remove the background — use this on parent/group nodes
    /// together with [`Node::with_opaque(false)`](crate::Node::with_opaque)
    /// so edges and children remain visible inside the node.
    pub fn with_selected_background(mut self, color: impl Into<Option<Color>>) -> Self {
        self.selected_background = Some(color.into());
        self
    }
}

impl Default for TextContent {
    fn default() -> Self {
        Self::new(Text::default())
    }
}

impl From<&str> for TextContent {
    fn from(s: &str) -> Self {
        Self::new(s.to_owned())
    }
}

impl From<String> for TextContent {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<Text<'static>> for TextContent {
    fn from(text: Text<'static>) -> Self {
        Self::new(text)
    }
}

impl NodeContent for TextContent {
    fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {
        let (border_style, text_style, background) = self.resolve_styles(ctx.theme, ctx.selected);

        let mut block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(border_style);
        if let Some(ref title) = self.title {
            block = block.title(title.as_str());
        }
        if let Some(bg) = background {
            block = block.bg(bg);
        }
        block.render(ctx.area, buf);
        let lines: Vec<String> = self.text.lines.iter().map(|l| l.to_string()).collect();
        render_node_text(ctx.area, &lines, text_style, buf);
    }
}

impl Node<TextContent> {
    /// Creates a node with auto-calculated dimensions based on the text content.
    ///
    /// Dimensions are calculated from the text using ratatui's measurement:
    /// - Width: widest line + 2 (for borders)
    /// - Height: number of lines + 2 (for borders)
    ///
    /// For uniform sizing across a graph, use [`Flow::with_uniform_width`](crate::Flow::with_uniform_width)
    /// and [`Flow::with_uniform_height`](crate::Flow::with_uniform_height) after creating the flow.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rataflow::{Node, Position};
    /// use ratatui::text::Text;
    ///
    /// // Single line
    /// let node = Node::from_text("n1", (10.0, 20.0), "Hello World");
    ///
    /// // Multiple lines
    /// let node = Node::from_text("n2", (10.0, 30.0), "Hello\nWorld");
    ///
    /// // From ratatui Text
    /// let node = Node::from_text("n3", (10.0, 40.0), Text::from("Styled text"));
    ///
    /// // With padding
    /// let node = Node::from_text("n4", (10.0, 50.0), "Hello").with_padding(1.0, 0.0);
    /// ```
    pub fn from_text<'a>(
        id: impl Into<String>,
        position: impl Into<Position>,
        text: impl Into<Text<'a>>,
    ) -> Self {
        let text = text.into();
        let width = text.width() + 2;
        let height = text.height().max(1) + 2; // At least height 3 for empty text

        // Convert to owned Text<'static>
        let owned_text: Text<'static> = text.to_string().into();

        Node::new(
            id,
            position,
            (width as f64, height as f64),
            TextContent::new(owned_text),
        )
    }
}

/// Built-in edge type that routes with orthogonal (step) segments.
///
/// Uses only horizontal and vertical segments with corners at turning points.
/// This is the most common edge style for flow diagrams.
///
/// # Styling
///
/// Style fields are public on the struct, matching [`StraightEdge`] and [`TextContent`].
/// Use `style` and `selected_style` to customize edge appearance.
///
/// # Stem Length
///
/// The `stem_length` field controls the **minimum distance** the edge must travel
/// in the handle's direction before any routing turns can occur. This prevents
/// edges from turning immediately after leaving a handle, which creates cleaner
/// layouts especially when nodes are close together.
///
/// For example, with `stem_length = 3.0` on a node with a Right handle:
/// - The edge travels at least 3 world units to the right before turning
/// - Only then does the routing algorithm determine the path to the target
///
/// Default is 1.0. Set to 0.0 for immediate routing (edges can turn right away).
///
/// Since stem_length is in world units, it scales proportionally with zoom.
///
/// # Example
///
/// ```
/// # #![allow(unused)]
/// use rataflow::{Edge, EdgeStyle, StepEdge};
/// use ratatui::style::{Color, Style};
///
/// // Default step edge (stem_length = 1.0)
/// let edge: Edge<StepEdge> = Edge::new("e1", "node1", "node2");
///
/// // With larger stem length for cleaner routing
/// let edge = Edge::new("e2", "node1", "node2")
///     .with_content(StepEdge::default().with_stem_length(3.0));
///
/// // With label (set on Edge, rendered automatically)
/// let edge: Edge<StepEdge> = Edge::new("e3", "node1", "node2").with_label("connects");
///
/// // With custom style
/// let custom_edge = StepEdge::default()
///     .with_style(EdgeStyle::default().with_stroke_style(Style::default().fg(Color::Green)));
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StepEdge {
    /// Style for rendering the edge.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub style: Option<EdgeStyle>,
    /// Style when the edge is selected.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub selected_style: Option<EdgeStyle>,
    /// The length of the "stem" extending straight from each handle before routing.
    /// This is the minimum distance the edge travels in the handle's direction
    /// before any turns occur. Default is 1.0. Set to 0.0 for immediate routing.
    #[cfg_attr(
        feature = "serde",
        serde(default = "crate::types::serde_defaults::f64_one")
    )]
    pub stem_length: f64,
}

impl Default for StepEdge {
    fn default() -> Self {
        Self {
            style: None,
            selected_style: None,
            stem_length: 1.0,
        }
    }
}

impl StepEdge {
    /// Sets the style for this edge.
    pub fn with_style(mut self, style: EdgeStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the selected style for this edge.
    pub fn with_selected_style(mut self, style: EdgeStyle) -> Self {
        self.selected_style = Some(style);
        self
    }

    /// Sets the minimum travel distance before routing turns.
    ///
    /// The edge must travel at least this distance in the handle's direction
    /// before any routing turns can occur. This prevents cramped-looking edges
    /// when nodes are close together.
    ///
    /// Since stem_length is in world units, it scales proportionally with zoom.
    /// Set to 0.0 for immediate routing (edges can turn right away).
    pub fn with_stem_length(mut self, stem_length: f64) -> Self {
        self.stem_length = stem_length;
        self
    }
}

impl EdgeContent for StepEdge {
    fn compute_path(&self, ctx: &EdgePathContext) -> Path {
        compute_step_path(
            ctx.from,
            ctx.to,
            ctx.source_position,
            ctx.target_position,
            self.stem_length,
        )
    }

    fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
        let default_style = EdgeStyle::default();
        let style = if ctx.selected {
            self.selected_style.as_ref().unwrap_or(&default_style)
        } else {
            self.style.as_ref().unwrap_or(&default_style)
        };
        let label = ctx.label.map(Text::raw);
        ctx.render_path(style, label.as_ref(), buf);
    }
}

/// Built-in edge type that draws a straight line between source and target.
///
/// Uses a direct point-to-point path with no routing. Simpler than [`StepEdge`]
/// but may overlap with nodes when the source and target are not aligned.
///
/// # Styling
///
/// Style fields are public on the struct, matching [`StepEdge`] and [`TextContent`].
/// Use `style` and `selected_style` to customize edge appearance.
///
/// Strokes with [`EdgeStyle::braille`] by default, so a slope renders continuously
/// instead of as a staircase of line characters — the common case here, since any
/// node drag takes an edge off exact alignment. The tradeoff is at exact alignment:
/// a horizontal or vertical edge is a lighter stroke on the cell's top or left edge
/// rather than a centered `─`/`│`, and braille does not merge with the box-drawing
/// characters a crossing [`StepEdge`] uses. Set `style` to
/// [`EdgeStyle::default()`](EdgeStyle::default) for the character rendering.
///
/// # Example
///
/// ```
/// # #![allow(unused)]
/// use rataflow::{Edge, StraightEdge};
///
/// // Default straight edge
/// let edge: Edge<StraightEdge> = Edge::new("e1", "node1", "node2");
///
/// // With label (set on Edge, rendered automatically)
/// let edge: Edge<StraightEdge> = Edge::new("e2", "node1", "node2").with_label("connects");
/// ```
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StraightEdge {
    /// Style for rendering the edge.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub style: Option<EdgeStyle>,
    /// Style when the edge is selected.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub selected_style: Option<EdgeStyle>,
}

impl StraightEdge {
    /// Sets the style for this edge.
    pub fn with_style(mut self, style: EdgeStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the selected style for this edge.
    pub fn with_selected_style(mut self, style: EdgeStyle) -> Self {
        self.selected_style = Some(style);
        self
    }
}

impl EdgeContent for StraightEdge {
    fn compute_path(&self, ctx: &EdgePathContext) -> Path {
        compute_straight_path(ctx.from, ctx.to, ctx.source_position, ctx.target_position)
    }

    fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
        // Straight edges run at arbitrary angles, where one character per cell can
        // only approximate the slope; braille strokes it at sub-cell resolution.
        let default_style = EdgeStyle::braille();
        let style = if ctx.selected {
            self.selected_style.as_ref().unwrap_or(&default_style)
        } else {
            self.style.as_ref().unwrap_or(&default_style)
        };
        let label = ctx.label.map(Text::raw);
        ctx.render_path(style, label.as_ref(), buf);
    }
}

/// Built-in edge type that attaches to whichever sides the two nodes currently face.
///
/// A [`StepEdge`] or [`StraightEdge`] leaves from the handle the edge names and stays
/// there however the nodes are arranged. This one ignores the resolved endpoints and
/// derives its own from the node bounds, so dragging a node re-attaches its edges
/// instead of leaving them entering from behind.
///
/// The side is chosen by [`HandlePosition::facing`]. Where on that side the edge
/// lands depends on the route, because the two have different resolution to spend:
/// a [`Step`](FloatingRoute::Step) route takes the middle of the side, while a
/// [`Straight`](FloatingRoute::Straight) route takes the exact point where the line
/// between the two centers crosses the outline, so its endpoint slides as the node
/// moves rather than jumping from one midpoint to the next.
///
/// Nothing has to be declared on the node for any of this — no handles on the
/// relevant sides, no flag on the edge. It is computed per frame from the two
/// rectangles the flow hands to [`EdgeContent::compute_path`], which is also why
/// rendering and hit testing cannot disagree: both go through that one call.
///
/// # Example
///
/// ```no_run
/// use rataflow::{Edge, FloatingEdge};
///
/// let edge: Edge<FloatingEdge> = Edge::new("e1", "a", "b");
///
/// // Straight rather than stepped, and it still re-attaches as the nodes move.
/// let edge = Edge::new("e2", "a", "b").with_content(FloatingEdge::straight());
/// ```
/// How a [`FloatingEdge`] routes between the two sides it picked.
///
/// The stem length belongs to [`Step`](FloatingRoute::Step) rather than to the edge,
/// so a straight route carries no setting it would ignore.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FloatingRoute {
    /// Orthogonal, leaving each side by `stem_length` before turning.
    Step {
        /// Length of the straight stem leaving each side.
        stem_length: f64,
    },
    /// A direct line between the two sides.
    Straight,
}

impl Default for FloatingRoute {
    fn default() -> Self {
        // Matches `StepEdge`, so the two agree on shape where they overlap.
        Self::Step { stem_length: 1.0 }
    }
}

/// Where on the chosen side a [`FloatingEdge`] lands.
///
/// The side comes from [`HandlePosition::facing`] either way; this decides the
/// point on it. Left unset, it follows the route, because the two routes have
/// different resolution to spend — see [`FloatingEdge::with_attachment`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FloatingAttachment {
    /// The middle of the side. The endpoint holds still until the facing side
    /// changes, then jumps to the next midpoint.
    Midpoint,
    /// The point where the line between the two node centers crosses the outline.
    /// The endpoint slides along the side as the node moves.
    Perimeter,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FloatingEdge {
    /// How the edge routes between the sides it picked.
    pub route: FloatingRoute,
    /// Where on those sides it lands. `None` follows the route.
    #[cfg_attr(feature = "serde", serde(default))]
    pub attachment: Option<FloatingAttachment>,
    /// Style for rendering the edge.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub style: Option<EdgeStyle>,
    /// Style when the edge is selected.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub selected_style: Option<EdgeStyle>,
}

impl FloatingEdge {
    /// A floating edge routed orthogonally. Same as [`Default`].
    pub fn stepped() -> Self {
        Self::default()
    }

    /// A floating edge drawn as a direct line between the two sides.
    pub fn straight() -> Self {
        Self {
            route: FloatingRoute::Straight,
            ..Self::default()
        }
    }

    /// Sets the routing.
    pub fn with_route(mut self, route: FloatingRoute) -> Self {
        self.route = route;
        self
    }

    /// Pins where on the chosen side the edge lands, overriding the route's default.
    ///
    /// Unset, a [`Straight`](FloatingRoute::Straight) route uses
    /// [`Perimeter`](FloatingAttachment::Perimeter) and a
    /// [`Step`](FloatingRoute::Step) route uses
    /// [`Midpoint`](FloatingAttachment::Midpoint), because they have different
    /// resolution to spend: a straight edge is drawn in braille, whose sub-cell dots
    /// can show an endpoint moving continuously, while a stepped edge is drawn in
    /// box-drawing characters, where an off-center endpoint only shifts the elbow by
    /// a whole cell.
    ///
    /// Both are available to both routes. A stepped edge can take `Perimeter` if you
    /// want the stem to leave from wherever the nodes line up, and a straight edge
    /// can take `Midpoint` if you would rather it snapped.
    ///
    /// ```
    /// use rataflow::{FloatingAttachment, FloatingEdge};
    ///
    /// let snapping = FloatingEdge::straight().with_attachment(FloatingAttachment::Midpoint);
    /// let sliding = FloatingEdge::stepped().with_attachment(FloatingAttachment::Perimeter);
    /// ```
    pub fn with_attachment(mut self, attachment: FloatingAttachment) -> Self {
        self.attachment = Some(attachment);
        self
    }

    /// Sets the style for this edge.
    pub fn with_style(mut self, style: EdgeStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the selected style for this edge.
    pub fn with_selected_style(mut self, style: EdgeStyle) -> Self {
        self.selected_style = Some(style);
        self
    }

    /// Applies the configured routing between two resolved endpoints.
    fn route_between(
        &self,
        from: Position,
        to: Position,
        source_side: HandlePosition,
        target_side: HandlePosition,
    ) -> Path {
        match self.route {
            FloatingRoute::Step { stem_length } => {
                compute_step_path(from, to, source_side, target_side, stem_length)
            }
            FloatingRoute::Straight => compute_straight_path(from, to, source_side, target_side),
        }
    }

    /// The point at the middle of `side` on `bounds`.
    fn side_midpoint(bounds: &crate::types::Rect, side: HandlePosition) -> Position {
        let center = bounds.center();
        match side {
            HandlePosition::Top => Position::new(center.x, bounds.y()),
            HandlePosition::Bottom => Position::new(center.x, bounds.bottom()),
            HandlePosition::Left => Position::new(bounds.x(), center.y),
            HandlePosition::Right => Position::new(bounds.right(), center.y),
        }
    }

    /// The point where a ray from `bounds`' center toward `toward` leaves `bounds`.
    ///
    /// Scales the ray until it meets the nearer pair of edges: the smaller of the
    /// two axis ratios is the one that limits it.
    fn exit_point(bounds: &crate::types::Rect, toward: Position) -> Position {
        let center = bounds.center();
        let (dx, dy) = (toward.x - center.x, toward.y - center.y);
        let (half_w, half_h) = (bounds.width() / 2.0, bounds.height() / 2.0);

        let scale_x = if dx == 0.0 {
            f64::INFINITY
        } else {
            half_w / dx.abs()
        };
        let scale_y = if dy == 0.0 {
            f64::INFINITY
        } else {
            half_h / dy.abs()
        };
        let scale = scale_x.min(scale_y);
        if !scale.is_finite() {
            // Coincident centers — there is no ray to follow.
            return center;
        }
        Position::new(center.x + dx * scale, center.y + dy * scale)
    }

    /// Where this edge meets `bounds`, given the other end lies toward `toward`.
    ///
    /// Resolves [`attachment`](Self::attachment) against the route when it is unset;
    /// see [`with_attachment`](Self::with_attachment) for why the routes default
    /// differently.
    fn attachment_point(&self, bounds: &crate::types::Rect, toward: Position) -> Position {
        let attachment = self.attachment.unwrap_or(match self.route {
            FloatingRoute::Step { .. } => FloatingAttachment::Midpoint,
            FloatingRoute::Straight => FloatingAttachment::Perimeter,
        });
        match attachment {
            FloatingAttachment::Midpoint => {
                Self::side_midpoint(bounds, HandlePosition::facing(bounds, toward))
            }
            FloatingAttachment::Perimeter => Self::exit_point(bounds, toward),
        }
    }
}

impl EdgeContent for FloatingEdge {
    fn compute_path(&self, ctx: &EdgePathContext) -> Path {
        // No target node yet — a preview still following the cursor. Leave from the
        // side facing the cursor and arrive at the cursor itself.
        let Some(target_bounds) = ctx.target_bounds else {
            let source_side = HandlePosition::facing(&ctx.source_bounds, ctx.to);
            let from = self.attachment_point(&ctx.source_bounds, ctx.to);
            return self.route_between(from, ctx.to, source_side, source_side.opposite());
        };

        // The sides still classify the endpoints even when the attachment is solved
        // rather than snapped: they are what orients the markers.
        let source_side = HandlePosition::facing(&ctx.source_bounds, target_bounds.center());
        let target_side = HandlePosition::facing(&target_bounds, ctx.source_bounds.center());
        let from = self.attachment_point(&ctx.source_bounds, target_bounds.center());
        let to = self.attachment_point(&target_bounds, ctx.source_bounds.center());

        self.route_between(from, to, source_side, target_side)
    }

    fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
        let default_style = match self.route {
            FloatingRoute::Step { .. } => EdgeStyle::default(),
            // Same reasoning as `StraightEdge`: an arbitrary slope needs sub-cell
            // resolution, and a straight floating edge is at one by nature.
            FloatingRoute::Straight => EdgeStyle::braille(),
        };
        let style = if ctx.selected {
            self.selected_style.as_ref().unwrap_or(&default_style)
        } else {
            self.style.as_ref().unwrap_or(&default_style)
        };
        let label = ctx.label.map(Text::raw);
        ctx.render_path(style, label.as_ref(), buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Rect;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect::from_coords(x, y, w, h)
    }

    /// A context whose resolved endpoints deliberately disagree with the geometry,
    /// so a test can tell whether `FloatingEdge` used them or the bounds.
    fn ctx(source: Rect, target: Option<Rect>) -> EdgePathContext {
        EdgePathContext {
            from: Position::new(-999.0, -999.0),
            to: target.map_or(Position::new(500.0, 0.0), |t| t.center()),
            source_position: HandlePosition::Top,
            target_position: HandlePosition::Top,
            source_bounds: source,
            target_bounds: target,
        }
    }

    #[test]
    fn floating_edge_attaches_to_the_facing_sides() {
        let source = rect(0.0, 0.0, 20.0, 10.0);

        // Target to the right: leave from the right side, arrive on the left.
        let path =
            FloatingEdge::default().compute_path(&ctx(source, Some(rect(200.0, 0.0, 20.0, 10.0))));
        assert_eq!(path.source_position, HandlePosition::Right);
        assert_eq!(path.target_position, HandlePosition::Left);

        // Target below: the same edge now leaves from the bottom.
        let path =
            FloatingEdge::default().compute_path(&ctx(source, Some(rect(0.0, 200.0, 20.0, 10.0))));
        assert_eq!(path.source_position, HandlePosition::Bottom);
        assert_eq!(path.target_position, HandlePosition::Top);
    }

    #[test]
    fn floating_edge_ignores_the_resolved_handles() {
        // `ctx` puts `from` far off in the corner and names Top for both sides. A
        // floating edge must derive its own endpoints from the bounds instead.
        let source = rect(0.0, 0.0, 20.0, 10.0);
        let path =
            FloatingEdge::default().compute_path(&ctx(source, Some(rect(200.0, 0.0, 20.0, 10.0))));

        let start = path.points.first().expect("path has points");
        assert_eq!(
            *start,
            Position::new(20.0, 5.0),
            "middle of the source's right side"
        );
    }

    #[test]
    fn floating_edge_previews_toward_the_cursor() {
        // No target node yet: leave from the side facing the cursor, end on it.
        let source = rect(0.0, 0.0, 20.0, 10.0);
        let mut context = ctx(source, None);
        context.to = Position::new(10.0, 200.0);

        let path = FloatingEdge::default().compute_path(&context);
        assert_eq!(path.source_position, HandlePosition::Bottom);
        assert_eq!(*path.points.last().expect("path has points"), context.to);
    }

    #[test]
    fn floating_edge_straight_and_stepped_agree_on_sides() {
        let source = rect(0.0, 0.0, 20.0, 10.0);
        let target = Some(rect(200.0, 60.0, 20.0, 10.0));
        let stepped = FloatingEdge::stepped().compute_path(&ctx(source, target));
        let straight = FloatingEdge::straight().compute_path(&ctx(source, target));

        // The sides agree, because both classify the endpoints the same way.
        assert_eq!(stepped.source_position, straight.source_position);
        assert_eq!(stepped.target_position, straight.target_position);
        // The endpoints do NOT: a stepped edge takes the middle of that side, a
        // straight one takes the exact point the center-to-center line crosses.
        assert_ne!(stepped.points.first(), straight.points.first());
        assert!(stepped.points.len() > straight.points.len());
    }

    #[test]
    fn floating_attachment_overrides_the_route_default_either_way() {
        let source = rect(0.0, 0.0, 20.0, 10.0);
        let target = Some(rect(200.0, 60.0, 20.0, 10.0));

        // A straight edge told to snap lands where a stepped one does.
        let snapping = FloatingEdge::straight()
            .with_attachment(FloatingAttachment::Midpoint)
            .compute_path(&ctx(source, target));
        let stepped = FloatingEdge::stepped().compute_path(&ctx(source, target));
        assert_eq!(snapping.points.first(), stepped.points.first());

        // A stepped edge told to slide lands where a straight one does.
        let sliding = FloatingEdge::stepped()
            .with_attachment(FloatingAttachment::Perimeter)
            .compute_path(&ctx(source, target));
        let straight = FloatingEdge::straight().compute_path(&ctx(source, target));
        assert_eq!(sliding.points.first(), straight.points.first());
    }

    #[test]
    fn floating_straight_slides_along_the_side_instead_of_snapping() {
        // A target offset on both axes: the crossing point is off-center, which is
        // the whole difference from a midpoint attachment.
        let source = rect(0.0, 0.0, 20.0, 10.0);
        let straight = FloatingEdge::straight()
            .compute_path(&ctx(source, Some(rect(200.0, 60.0, 20.0, 10.0))));
        let start = *straight.points.first().expect("path has points");

        assert_eq!(start.x, 20.0, "leaves through the right side");
        assert!(
            start.y > 5.0,
            "and below that side's midpoint, because the target sits lower: {start:?}"
        );

        // Move the target further down and the endpoint slides further down with
        // it, rather than staying put until the facing side flips.
        let lower = FloatingEdge::straight()
            .compute_path(&ctx(source, Some(rect(200.0, 120.0, 20.0, 10.0))));
        assert!(lower.points.first().expect("path has points").y > start.y);
    }
}
