/// Convenience result type for examples.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub mod animating_edges;
pub mod autopilot;
pub mod basic;
pub mod context_menu;
pub mod custom_bindings;
pub mod custom_edges;
pub mod custom_layout;
pub mod custom_nodes;
pub mod default_bindings;
pub mod edge_routing;
pub mod floating_edges;
pub mod hierarchy;
pub mod history;
pub mod meta;
pub mod mutations;
pub mod node_flags;
pub mod reconnection;
pub mod save_restore;
pub mod shell;
pub mod theming;
pub mod validation;
pub mod view_only;

pub use animating_edges::animating_edges;
pub use custom_bindings::{
    CUSTOM_CONTROLS_KEY_BINDINGS, CUSTOM_FLOW_KEY_BINDINGS, custom_controls_bindings,
    custom_flow_bindings, custom_keys,
};
pub use custom_edges::MyEdge;
pub use custom_layout::{compute_layout, layout_tree};
pub use custom_nodes::MyNode;
pub use default_bindings::{CONTROLS_KEY_BINDINGS, FLOW_KEY_BINDINGS, default_keys};
pub use history::History;
pub use shell::{
    ACCENT, ExampleMeta, accent_style, muted_style, render_indicator, render_shell, render_status,
};
