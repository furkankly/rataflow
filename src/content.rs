//! Traits for custom node and edge rendering.
//!
//! - [`NodeContent`] — controls how nodes render
//! - [`EdgeContent`] — controls how edges render and route
//!
//! # Example
//!
//! ```no_run
//! use ratatui::buffer::Buffer;
//! use ratatui::style::{Color, Modifier, Style};
//! use ratatui::widgets::{Block, Paragraph, Widget};
//! use rataflow::{NodeContent, NodeRenderContext};
//!
//! #[derive(Debug)]
//! struct MyContent {
//!     label: String,
//!     priority: u8,
//! }
//!
//! impl NodeContent for MyContent {
//!     fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {
//!         // Use ratatui widgets directly - no library helpers needed
//!         let border_style = if ctx.selected {
//!             Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
//!         } else {
//!             Style::default().fg(Color::White)
//!         };
//!         let block = Block::bordered().border_style(border_style);
//!         let paragraph = Paragraph::new(&*self.label).block(block);
//!         paragraph.render(ctx.area, buf);
//!     }
//! }
//! ```

use std::fmt::Debug;

use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::Text};

use crate::state::RenderContext;
use crate::theme::Theme;
use crate::types::{HandlePosition, Position, Viewport};
use crate::ui::{EdgeStyle, Path, edge_render};

/// Context passed to [`NodeContent::render`].
///
/// Provides render metadata for a node. Your content is available as `self`
/// in the render method — use ratatui widgets directly to render into `ctx.area`.
///
/// # Example
///
/// ```no_run
/// # #![allow(unused)]
/// # use ratatui::buffer::Buffer;
/// # use ratatui::style::{Color, Modifier, Style};
/// # use rataflow::{NodeContent, NodeRenderContext};
/// # #[derive(Debug)]
/// # struct MyContent { my_field: String }
/// # impl NodeContent for MyContent {
/// fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {
///     let border_color = if ctx.selected { Color::Cyan } else { Color::White };
///     let mut style = Style::default().fg(border_color);
///     if ctx.dragging {
///         style = style.add_modifier(Modifier::DIM);
///     }
///     // self.my_field is your content data
///     buf.set_style(ctx.area, style);
/// }
/// # }
/// ```
pub struct NodeRenderContext<'a> {
    /// Node ID.
    pub id: &'a str,
    /// The rectangle where the node should be rendered.
    ///
    /// Local coordinates starting at `(0, 0)` with the node's full dimensions.
    /// Always receives the complete area regardless of viewport clipping.
    pub area: Rect,
    /// Whether this node is currently selected.
    pub selected: bool,
    /// Whether this node is currently being dragged.
    pub dragging: bool,
    /// Absolute position of the node's top-left corner in world coordinates.
    /// For nodes with parents, this includes the parent's position.
    pub position_absolute: Position,
    /// The current theme.
    pub theme: Theme,
    /// Animation clock phase: advances by 1 every
    /// [`animation_speed_ms`](crate::Flow::animation_speed_ms) milliseconds —
    /// the same clock that drives edge marching ants. Drive subtle content
    /// animations from it (e.g. a status pulse via `phase / 8 % 2`). Stays `0`
    /// until the app calls [`tick_animation`](crate::Flow::tick_animation).
    pub animation_phase: usize,
}

/// What an edge may consult when computing its path, in world coordinates.
///
/// `from`/`to` are the endpoints the flow resolved from the edge's handles — the
/// answer most edges want. `source_bounds`/`target_bounds` are the node rectangles
/// those handles sit on, for edges that derive their own endpoints instead: an edge
/// that attaches wherever the two nodes happen to face, or anywhere on the
/// perimeter, needs the shapes rather than two points.
///
/// The rectangles are owned copies, not borrows of the layout — an edge is being
/// asked where to draw, not handed the graph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgePathContext {
    /// Start position, resolved from the source handle.
    pub from: Position,
    /// End position, resolved from the target handle.
    pub to: Position,
    /// Side the source handle sits on.
    pub source_position: HandlePosition,
    /// Side the target handle sits on.
    pub target_position: HandlePosition,
    /// Bounds of the source node.
    pub source_bounds: crate::types::Rect,
    /// Bounds of the target node.
    ///
    /// `None` while a connection preview follows the cursor: `to` is the cursor and
    /// there is no target node yet.
    pub target_bounds: Option<crate::types::Rect>,
}

/// Context passed to [`EdgeContent::render`].
///
/// Provides render metadata and coordinate helpers for an edge.
/// Your content is available as `self` in the render method.
///
/// For most cases, call [`render_path`](Self::render_path) with an [`EdgeStyle`]
/// and optional label — it handles coordinate transforms, clipping, and drawing.
///
/// # Example
///
/// ```no_run
/// # #![allow(unused)]
/// # use ratatui::buffer::Buffer;
/// # use ratatui::text::Text;
/// # use rataflow::{EdgeContent, EdgePathContext, EdgeRenderContext, EdgeStyle, Path};
/// # #[derive(Debug, Default)]
/// # struct MyEdge { style: EdgeStyle, selected_style: EdgeStyle }
/// # impl EdgeContent for MyEdge {
/// #     fn compute_path(&self, ctx: &EdgePathContext) -> Path { Path::straight(ctx.from, ctx.to) }
/// fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
///     let style = if ctx.selected { &self.selected_style } else { &self.style };
///     let label = ctx.label.map(Text::raw);
///     ctx.render_path(style, label.as_ref(), buf);
/// }
/// # }
/// ```
pub struct EdgeRenderContext<'a> {
    /// Edge ID.
    pub id: &'a str,
    /// Whether this edge is currently selected.
    pub selected: bool,
    /// Label from [`Edge::label`](crate::Edge::label), if set.
    pub label: Option<&'a str>,
    /// Pre-computed path in world coordinates.
    pub path: Path,
    /// The current theme.
    pub theme: Theme,
    // Internal: coordinate transformation state
    pub(crate) viewport: &'a Viewport,
    pub(crate) render_ctx: &'a RenderContext,
    pub(crate) source_endpoint_offset: (i32, i32),
    pub(crate) target_endpoint_offset: (i32, i32),
    pub(crate) animation_phase: Option<usize>,
}

impl<'a> EdgeRenderContext<'a> {
    /// Renders the edge path with the given style and optional label.
    ///
    /// - Transforms world coordinates to terminal coordinates
    /// - Clips to the visible canvas area
    /// - Draws markers (arrows/dots) at endpoints
    /// - Applies animation phase if enabled
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #![allow(unused)]
    /// # use ratatui::buffer::Buffer;
    /// # use ratatui::text::Text;
    /// # use rataflow::{EdgeContent, EdgePathContext, EdgeRenderContext, EdgeStyle, Path};
    /// # #[derive(Debug, Default)]
    /// # struct MyEdge { style: EdgeStyle, selected_style: EdgeStyle }
    /// # impl EdgeContent for MyEdge {
    /// #     fn compute_path(&self, ctx: &EdgePathContext) -> Path { Path::straight(ctx.from, ctx.to) }
    /// fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
    ///     let style = if ctx.selected { &self.selected_style } else { &self.style };
    ///     let label = ctx.label.map(Text::raw);
    ///     ctx.render_path(style, label.as_ref(), buf);
    /// }
    /// # }
    /// ```
    pub fn render_path(&self, style: &EdgeStyle, label: Option<&Text<'_>>, buf: &mut Buffer) {
        let palette = self.theme.palette();
        let stroke_color = if self.selected {
            palette.accent
        } else {
            palette.muted
        };
        let resolved_style = style.resolved_style(
            Style::default().fg(stroke_color),
            Style::default().fg(palette.text),
        );
        edge_render::render_path(
            &self.path,
            &resolved_style,
            label,
            self.viewport,
            self.render_ctx,
            self.source_endpoint_offset,
            self.target_endpoint_offset,
            self.animation_phase,
            buf,
        );
    }

    /// Transforms a world-coordinate point to terminal coordinates.
    ///
    /// **Escape hatch** for custom rendering beyond what [`render_path`](Self::render_path) provides.
    /// Use this when you need to draw additional decorations at specific positions.
    /// Identical to [`Flow::world_to_terminal`](crate::Flow::world_to_terminal);
    /// this context-bound version exists because content renders without access
    /// to the `Flow`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #![allow(unused)]
    /// # use ratatui::buffer::Buffer;
    /// # use ratatui::style::Style;
    /// # use rataflow::{EdgeContent, EdgePathContext, EdgeRenderContext, EdgeStyle, Path};
    /// # #[derive(Debug, Default)]
    /// # struct MyEdge { style: EdgeStyle }
    /// # impl EdgeContent for MyEdge {
    /// #     fn compute_path(&self, ctx: &EdgePathContext) -> Path { Path::straight(ctx.from, ctx.to) }
    /// fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
    ///     ctx.render_path(&self.style, None, buf);
    ///
    ///     // Draw a custom marker at the path midpoint
    ///     let (x, y) = ctx.world_to_terminal(ctx.path.label_position);
    ///     if ctx.is_in_bounds(x, y) {
    ///         buf.set_string(x as u16, y as u16, "●", Style::default());
    ///     }
    /// }
    /// # }
    /// ```
    pub fn world_to_terminal(&self, pos: Position) -> (i32, i32) {
        self.render_ctx.world_to_terminal(self.viewport, pos)
    }

    /// Checks if terminal coordinates are within the drawable canvas area.
    ///
    /// **Escape hatch** for custom rendering. Use before drawing to avoid
    /// writing outside canvas bounds. Identical to
    /// [`Flow::is_in_bounds`](crate::Flow::is_in_bounds).
    pub fn is_in_bounds(&self, x: i32, y: i32) -> bool {
        self.render_ctx.is_in_canvas(x, y)
    }
}

/// Trait for custom node rendering.
///
/// Implement this to control how nodes look. There is no library-level node style
/// primitive — use ratatui widgets directly (e.g., `Block`, `Paragraph`) to render
/// into `ctx.area`. The library provides [`TextContent`](crate::TextContent) as
/// a built-in implementation.
///
/// # Example
///
/// ```no_run
/// use ratatui::buffer::Buffer;
/// use ratatui::style::{Color, Modifier, Style};
/// use ratatui::widgets::{Block, Paragraph, Widget};
/// use rataflow::{NodeContent, NodeRenderContext};
///
/// #[derive(Debug)]
/// enum MyContent {
///     Input { name: String },
///     Process { name: String, code: String },
/// }
///
/// impl NodeContent for MyContent {
///     fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {
///         let border_style = if ctx.selected {
///             Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
///         } else {
///             Style::default().fg(Color::White)
///         };
///
///         let text = match self {
///             MyContent::Input { name } => name.clone(),
///             MyContent::Process { name, code } => format!("{}\n{}", name, code),
///         };
///
///         let block = Block::bordered().border_style(border_style);
///         let paragraph = Paragraph::new(text).block(block);
///         paragraph.render(ctx.area, buf);
///     }
/// }
/// ```
pub trait NodeContent: Debug + Sized {
    /// Render the node to the buffer.
    ///
    /// The context provides the render area and selection state.
    /// Your content data is available as `self`.
    /// Use ratatui primitives directly (e.g., `Block`, `Paragraph`).
    fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer);
}

/// Trait for custom edge routing and rendering.
///
/// Implement this to control how edges are routed and drawn. Unlike nodes,
/// the library provides [`EdgeStyle`] for configuring edge visuals (characters,
/// markers, style) and [`EdgeRenderContext::render_path`] for standard rendering.
/// Built-in implementations: [`StepEdge`](crate::StepEdge) and
/// [`StraightEdge`](crate::StraightEdge).
///
/// # Coordinate System
///
/// All path computation happens in **world coordinates** (f64). This ensures:
/// - Zoom-independent geometry (proportions stay the same at any zoom level)
/// - Consistent behavior for hit-testing and rendering
/// - Clean separation between geometry and screen-space concerns
///
/// The library transforms paths to terminal coordinates at render time.
///
/// # `Default` Requirement
///
/// This trait requires `Default` because the library uses `E::default().compute_path()`
/// to render the edge preview when users drag from a handle to create new edges.
/// The preview needs to show the same path shape that the actual edge will have.
///
/// For enum edge types with multiple variants, the `Default` implementation determines
/// which variant's path shape is used for the preview:
///
/// ```no_run
/// # #![allow(unused)]
/// # use rataflow::{StepEdge, StraightEdge};
/// #[derive(Debug)]
/// enum MyEdge {
///     Step(StepEdge),
///     Straight(StraightEdge),
/// }
///
/// impl Default for MyEdge {
///     fn default() -> Self {
///         // Preview will use Step path shape
///         MyEdge::Step(StepEdge::default())
///     }
/// }
/// ```
///
/// # Edge Rendering Architecture
///
/// The library computes the path and passes it via [`EdgeRenderContext`], similar to
/// how [`NodeRenderContext`] provides the render area. Your `render` method receives
/// the pre-computed path and draws it. This ensures consistency between rendering and
/// hit-testing (both use the same path).
///
/// # Example
///
/// ```
/// use ratatui::buffer::Buffer;
/// use rataflow::{EdgeContent, EdgePathContext, EdgeRenderContext, EdgeStyle, Path};
/// use rataflow::compute_step_path;
///
/// #[derive(Debug, Default, Clone)]
/// struct MyEdge {
///     style: EdgeStyle,
///     selected_style: EdgeStyle,
///     stem_length: f64,
/// }
///
/// impl EdgeContent for MyEdge {
///     fn compute_path(&self, ctx: &EdgePathContext) -> Path {
///         compute_step_path(
///             ctx.from,
///             ctx.to,
///             ctx.source_position,
///             ctx.target_position,
///             self.stem_length,
///         )
///     }
///
///     fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
///         let style = if ctx.selected { &self.selected_style } else { &self.style };
///         ctx.render_path(style, None, buf);
///     }
/// }
/// ```
pub trait EdgeContent: Debug + Default {
    /// Compute the path for this edge in world coordinates.
    ///
    /// Defines the edge's shape, used for both rendering and hit testing.
    ///
    /// The returned [`Path`] declares the sides it leaves and arrives on, which is
    /// what orients the end markers. An edge that ignores `ctx.from`/`ctx.to` and
    /// derives its own endpoints from the node bounds must say so there — see
    /// [`FloatingEdge`](crate::FloatingEdge), which does exactly that.
    fn compute_path(&self, ctx: &EdgePathContext) -> Path;

    /// Render the edge to the buffer.
    ///
    /// The context provides:
    /// - `ctx.path` - The pre-computed path in world coordinates (from `compute_path`)
    /// - `ctx.selected` - Whether this edge is selected
    ///
    /// Your content data is available as `self`.
    /// Use [`EdgeRenderContext::render_path`] for standard rendering.
    fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer);

    /// Tests if a point is on or near this edge in world coordinates.
    ///
    /// The default implementation uses `compute_path` and tests against the path.
    /// To adjust hit area size, set [`Flow::edge_hit_threshold`](crate::Flow::edge_hit_threshold)
    /// instead of overriding this method.
    fn hit_test(&self, point: Position, ctx: &EdgePathContext, threshold: f64) -> bool {
        let path = self.compute_path(ctx);
        path.hit_test(point, threshold)
    }
}
