//! Core type definitions.
//!
//! - [`Node`], [`Edge`] — graph elements with generic content types
//! - [`Position`], [`Dimensions`], [`Rect`] — f64-based geometry
//! - [`Viewport`] — pan/zoom state with coordinate transforms
//! - [`Handle`], [`HandlePosition`] — connection points on nodes
//!
//! All positions use f64 world coordinates. The viewport transforms these
//! through pan/zoom to terminal cell positions at render time.

mod connection;
mod edge;
mod geometry;
mod handle;
mod node;
mod viewport;

/// Default value helpers for `#[serde(default = "...")]` on fields whose
/// defaults differ from `Default::default()` (e.g., `true` instead of `false`).
#[cfg(feature = "serde")]
pub(crate) mod serde_defaults {
    pub const fn bool_true() -> bool {
        true
    }
    pub const fn f64_half() -> f64 {
        0.5
    }
    pub const fn f64_one() -> f64 {
        1.0
    }
    pub fn handle_position_right() -> super::HandlePosition {
        super::HandlePosition::Right
    }
    pub fn handle_position_left() -> super::HandlePosition {
        super::HandlePosition::Left
    }
}

// Re-export all public types
pub use connection::*;
pub use edge::*;
pub use geometry::*;
pub use handle::*;
pub use node::*;
pub use viewport::*;
