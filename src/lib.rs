// The mascot, in the docs.rs sidebar and its tab. Absolute URLs because rustdoc
// does not copy local files into the generated output — docs.rs would serve a
// dead link to a path that only exists in this checkout.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/furkankly/rataflow/main/assets/icon.svg",
    html_favicon_url = "https://raw.githubusercontent.com/furkankly/rataflow/main/assets/icon.svg"
)]
#![doc = include_str!("../README.md")]

pub mod actions;
pub mod content;
pub mod error;
pub mod input;
pub mod layout;
pub mod state;
pub mod theme;
pub mod types;
pub mod ui;

pub use actions::{
    ControlsAction, EventResponse, FlowAction, FlowEvent, default_controls_key_binding,
    default_flow_key_binding,
};
pub use content::{
    EdgeContent, EdgePathContext, EdgeRenderContext, NodeContent, NodeRenderContext,
};
pub use error::Error;
#[cfg(feature = "termwiz")]
pub use input::termwiz_helpers;
pub use input::{KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, MouseEventKind};
#[cfg(feature = "sugiyama")]
pub use layout::{IntoEdge, LayoutDirection, Sugiyama};
pub use state::Direction;
pub use state::EdgePreview;
pub use state::Flow;
pub use state::FlowSnapshot;
pub use state::Pick;
pub use state::flow_ops::FlowOps;
pub use theme::{Palette, Theme};
pub use types::*;
pub use ui::*;
