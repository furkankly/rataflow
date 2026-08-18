//! Key binding documentation constants aligned with library defaults.
//!
//! - [`FLOW_KEY_BINDINGS`] — graph interaction keys ([`rataflow::default_flow_key_binding`])
//! - [`CONTROLS_KEY_BINDINGS`] — viewport/controls keys ([`rataflow::default_controls_key_binding`])

/// Bindings handled by [`rataflow::default_flow_key_binding`].
///
/// Graph interaction: selection, panning, editing.
pub const FLOW_KEY_BINDINGS: &[(&str, &str)] = &[
    ("↑↓", "select next/prev"),
    ("hjkl", "pan viewport"),
    ("Del", "delete selected"),
    ("Esc", "cancel connection"),
    ("c", "center selection"),
    ("m", "toggle multi select"),
];

/// Bindings handled by [`rataflow::default_controls_key_binding`].
///
/// Viewport manipulation: zoom, fit, lock.
pub const CONTROLS_KEY_BINDINGS: &[(&str, &str)] = &[
    ("+/-", "zoom in/out"),
    ("0", "reset zoom"),
    ("f", "fit to view"),
    ("i", "toggle lock"),
];

/// Returns merged flow + controls bindings.
pub fn default_keys() -> Vec<(&'static str, &'static str)> {
    FLOW_KEY_BINDINGS
        .iter()
        .chain(CONTROLS_KEY_BINDINGS.iter())
        .copied()
        .collect()
}
