//! UI widgets and rendering helpers for rataflow.
//!
//! # Widgets
//!
//! - [`Background`] - Patterned background for the canvas
//! - [`Controls`] - Viewport control panel
//! - [`MiniMap`] - Scaled-down graph overview
//!
//! # Built-in Content Types
//!
//! - [`TextContent`] - Bordered text node
//! - [`StepEdge`] - Orthogonal (step) edge routing
//! - [`StraightEdge`] - Straight line edge routing
//! - [`FloatingEdge`] - Attaches to whichever sides the nodes currently face
//!
//! # Edge Path Functions
//!
//! - [`compute_step_path`] - Orthogonal path between two points
//! - [`compute_straight_path`] - Straight path between two points
//!
//! # Styles
//!
//! - [`EdgeStyle`] - Edge visual configuration (characters, corners, markers, style)
//! - [`EdgePreviewStyle`] - Edge preview color configuration (valid, invalid, no-target)
//! - [`HandleStyle`] - Handle visual configuration (character, style)

mod background;
mod builtins;
mod canvas;
mod controls;
mod edge_path;
mod edge_preview;
pub(crate) mod edge_render;
mod handle_render;
mod minimap;

pub use background::{Background, BackgroundStyle, BackgroundVariant};
pub use builtins::{
    FloatingAttachment, FloatingEdge, FloatingRoute, StepEdge, StraightEdge, TextContent,
};
pub use controls::{Controls, ControlsOrientation, ControlsPosition, ControlsStyle};
pub use edge_path::{Path, compute_step_path, compute_straight_path};
pub use edge_preview::EdgePreviewStyle;
pub(crate) use edge_render::ANIMATION_PATTERN_LENGTH;
pub use edge_render::{EdgeStroke, EdgeStyle};
pub use handle_render::HandleStyle;
pub use minimap::{MiniMap, MiniMapPosition, MiniMapStyle};
