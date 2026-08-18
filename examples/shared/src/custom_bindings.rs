//! Custom key bindings for examples.
//!
//! Demonstrates custom bindings using the action pattern.
//! WASD for panning, Tab for selection, z/x for zoom.

use rataflow::{ControlsAction, FlowAction, KeyCode, KeyEvent};

/// Custom flow key bindings for sidebar display.
pub const CUSTOM_FLOW_KEY_BINDINGS: &[(&str, &str)] =
    &[("tab", "select next/prev"), ("wasd", "pan viewport")];

/// Custom controls key bindings for sidebar display.
pub const CUSTOM_CONTROLS_KEY_BINDINGS: &[(&str, &str)] = &[
    ("z/x", "zoom in/out"),
    ("0", "reset zoom"),
    ("space", "fit to view"),
];

/// Returns merged custom flow + controls bindings.
pub fn custom_keys() -> Vec<(&'static str, &'static str)> {
    CUSTOM_FLOW_KEY_BINDINGS
        .iter()
        .chain(CUSTOM_CONTROLS_KEY_BINDINGS.iter())
        .copied()
        .collect()
}

/// Custom flow bindings: WASD panning, Tab selection.
pub fn custom_flow_bindings(key: &KeyEvent) -> Option<FlowAction> {
    match key.code {
        KeyCode::Char('w') => Some(FlowAction::PanUp),
        KeyCode::Char('a') => Some(FlowAction::PanLeft),
        KeyCode::Char('s') => Some(FlowAction::PanDown),
        KeyCode::Char('d') => Some(FlowAction::PanRight),
        KeyCode::Tab => {
            if key.modifiers.shift {
                Some(FlowAction::SelectPrev)
            } else {
                Some(FlowAction::SelectNext)
            }
        }
        _ => None,
    }
}

/// Custom controls bindings: z/x zoom, space fit.
pub fn custom_controls_bindings(key: &KeyEvent) -> Option<ControlsAction> {
    match key.code {
        KeyCode::Char('z') => Some(ControlsAction::ZoomIn),
        KeyCode::Char('x') => Some(ControlsAction::ZoomOut),
        KeyCode::Char('0') => Some(ControlsAction::ResetZoom),
        KeyCode::Char(' ') => Some(ControlsAction::FitView),
        _ => None,
    }
}
