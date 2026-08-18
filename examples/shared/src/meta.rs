//! One definition of each example's title, description and key list.
//!
//! These used to live twice: once in `examples/<name>.rs` and once in
//! `web/wasm/src/demos.rs`. The flows were already shared, so the graphs
//! could not drift — but the prose could, and did. Eight of twenty-one
//! descriptions had diverged, the wasm copies generally describing an older
//! version of the example ("Enum-based edge types with different path routing"
//! for what is now "five edges, five ways to draw one"), and two keys were
//! missing from the wasm sidebars entirely.
//!
//! The one difference that is real: a native example can quit and a browser tab
//! cannot, so `q` is not in these lists. Native binaries add it back with
//! [`ExampleMeta::with_quit`].

use crate::{ExampleMeta, default_keys};

// `description` is optional because ExampleMeta's is: `None` skips the sidebar
// entirely, which `events` relies on — it draws its own event-log panel instead.
fn meta(
    title: &'static str,
    description: Option<&'static str>,
    extra: Vec<(&'static str, &'static str)>,
) -> ExampleMeta<'static> {
    let mut keys = extra;
    keys.extend(default_keys());
    ExampleMeta {
        title,
        description,
        keys,
    }
}

pub fn animating_edges() -> ExampleMeta<'static> {
    meta(
        "Animating Edges",
        Some(
            "Marching ants animation on edges.\nDriven by tick_animation() with adjustable speed.",
        ),
        vec![("</>", "slower/faster")],
    )
}

pub fn basic() -> ExampleMeta<'static> {
    meta(
        "Basic",
        Some(
            "Simple flow graph with nodes and edges.\nDrag nodes to move, drag handles to connect.",
        ),
        vec![],
    )
}

pub fn basic_async() -> ExampleMeta<'static> {
    meta(
        "Basic (Async)",
        Some(
            "Async event handling with tokio.\nSame graph as basic, but uses channels for non-blocking event processing.",
        ),
        vec![],
    )
}

pub fn companion_widgets() -> ExampleMeta<'static> {
    meta(
        "Companion Widgets",
        Some(
            "Layout, structure, and style customization.\nCross background, horizontal controls, bordered minimap.",
        ),
        vec![],
    )
}

pub fn context_menu() -> ExampleMeta<'static> {
    meta(
        "Context Menu",
        Some(
            "A different menu for each target.\nRight-click a node, an edge, or the empty canvas. Each opens its own menu, and every item does something you can see.\nClick an item to run it, or move with j/k and press Enter. Esc closes.\nNothing happening? Your terminal is keeping the right button for itself. Press Space for the menu under the cursor.",
        ),
        vec![
            ("Space", "menu at cursor"),
            ("j/k", "menu item"),
            ("Enter", "run item"),
        ],
    )
}

pub fn custom_bindings() -> ExampleMeta<'static> {
    // The one example whose subject IS the key map: it rebinds panning to WASD,
    // selection to Tab and zoom to z/x, then falls through to the defaults. So
    // its extras are `custom_keys()` rather than a literal list — passing
    // `vec![]` here silently showed the default hjkl bindings instead, which is
    // the opposite of what the example demonstrates.
    meta(
        "Custom Bindings",
        Some(
            "Custom bindings with default fallthrough.\nWASD panning, Tab selection, z/x zoom, space fit.",
        ),
        crate::custom_bindings::custom_keys(),
    )
}

pub fn custom_edges() -> ExampleMeta<'static> {
    meta(
        "Custom Edges",
        Some(
            "Five edges, five ways to draw one.\nTop to bottom: a dotted line with end markers, the same diagonal drawn with braille dots for sub-cell smoothness, a labelled step route, a line with badges painted straight into the buffer, and a hand-built zigzag.\nClick any edge to see its selected styling.",
        ),
        vec![],
    )
}

pub fn custom_layout() -> ExampleMeta<'static> {
    meta(
        "Custom Layout",
        Some(
            "Custom tree layout algorithm.\nPositions computed externally and applied via set_node_positions().",
        ),
        vec![],
    )
}

pub fn custom_nodes() -> ExampleMeta<'static> {
    meta(
        "Custom Nodes",
        Some(
            "NodeContent trait with four approaches.\nTextContent, Paragraph, Canvas, raw buffer.",
        ),
        vec![],
    )
}

pub fn edge_routing() -> ExampleMeta<'static> {
    meta(
        "Edge Routing",
        Some("All 16 handle position combos.\nTop: Step edges, Bottom: Straight."),
        vec![],
    )
}

pub fn events() -> ExampleMeta<'static> {
    meta("Events", None, vec![])
}

pub fn floating_edges() -> ExampleMeta<'static> {
    meta(
        "Floating Edges",
        Some(
            "Edges that follow their nodes.\nPress a to cycle four ways an edge can meet a node. The bar along the bottom names the one you are on.\nThen drag a node around: the first three re-attach as it moves, the fourth stays where it started.\nTwo and three are the same edge with one field changed. One slides along the side, the other snaps to its middle.\nThe ● marks are handles. Only the fourth one uses them.",
        ),
        vec![("a", "cycle edge modes")],
    )
}

pub fn hierarchy() -> ExampleMeta<'static> {
    meta(
        "Hierarchy",
        Some(
            "Nodes that live inside other nodes.\nA child's position is measured from its parent's corner, so dragging a parent takes its children along.\nSelect any node to compare the two numbers at the bottom: the position it stores, and where it actually sits.\nOverflows and Nested grow their parent. Bounded cannot leave it. Regular moves freely.",
        ),
        vec![],
    )
}

pub fn multi_select() -> ExampleMeta<'static> {
    meta(
        "Multi Select",
        Some(
            "Three ways to build a selection.\nPress m, then click nodes and edges to add them one at a time.\nOr right-drag a box to take everything it touches. If nothing happens, your terminal is keeping that button — press b and left-drag draws the box instead.\nv selects everything on screen. d deletes the selection, s zooms to fit it.",
        ),
        vec![
            ("b", "box on left-drag"),
            ("v", "select in view"),
            ("d", "delete selected"),
            ("s", "fit to selection"),
        ],
    )
}

pub fn mutations() -> ExampleMeta<'static> {
    meta(
        "Mutations",
        Some(
            "Runtime graph mutation showcase.\nAdd, remove, resize, and reconfigure nodes and edges.",
        ),
        vec![
            ("a", "add node"),
            ("x", "delete selected"),
            ("r", "cycle size"),
            ("g", "nudge right"),
            ("n", "rename node"),
            ("m", "label all with id"),
            ("e", "select edge"),
            ("b", "cycle edge label"),
            ("w", "toggle animated"),
            ("t", "toggle lock"),
        ],
    )
}

pub fn node_flags() -> ExampleMeta<'static> {
    meta(
        "Node Flags",
        Some(
            "What each node will let you do.\nOne node per flag: try dragging, selecting or deleting each and see which refuse.\nToggle a flag on the selected node with d/s/p/o/v/z, and watch its label update.\nPress r to make a node resizable, then drag the ◢ grip at its bottom-right corner.",
        ),
        vec![
            ("d", "toggle draggable"),
            ("s", "toggle selectable"),
            ("p", "toggle deletable"),
            ("o", "toggle connectable"),
            ("v", "toggle hidden"),
            ("z", "cycle z-index"),
            ("r", "toggle resizable"),
        ],
    )
}

pub fn overview() -> ExampleMeta<'static> {
    meta(
        "Overview",
        Some(
            "Custom nodes, edges, and handles. Nodes via ratatui widgets, raw buffer, or third-party crates (ratatui-image, tachyonfx).\nRequires: image protocol (Kitty/iTerm2/Sixel). Image resize blocks render thread; stable sizes cached.",
        ),
        vec![("p", "image mode"), ("Enter", "toggle input")],
    )
}

pub fn reconnection() -> ExampleMeta<'static> {
    meta(
        "Reconnection",
        Some(
            "Edge reconnection modes: Both, Target-only, None.\nSelect an edge, then drag near its endpoint to reconnect.",
        ),
        vec![],
    )
}

pub fn termion_basic() -> ExampleMeta<'static> {
    meta(
        "Termion Backend",
        Some("Same library, different terminal backend.\nUses termion instead of crossterm."),
        vec![],
    )
}

pub fn theming() -> ExampleMeta<'static> {
    meta(
        "Theme",
        Some("Runtime theme switching.\nPress 't' to cycle Dark / Light / Sakura (custom)."),
        vec![("t", "cycle theme")],
    )
}

pub fn undo_redo() -> ExampleMeta<'static> {
    meta(
        "Undo / Redo",
        Some(
            "Snapshot-based undo/redo history.\nGraph mutations tracked and reversible with u / U.",
        ),
        vec![("u", "undo"), ("U", "redo"), ("a", "add node")],
    )
}

pub fn validation() -> ExampleMeta<'static> {
    meta(
        "Validation",
        Some(
            "Connection validation via ConnectionMode, validators, and handle flags.\nToggle Strict/Loose mode with 'o'.",
        ),
        vec![("o", "toggle conn mode")],
    )
}

pub fn view_only() -> ExampleMeta<'static> {
    meta(
        "View Only",
        Some(
            "Static read-only graph display.\nMinimal setup with from_edges() and request_fit_view().",
        ),
        vec![],
    )
}

/// Takes the saved flag: this example reports state in its sidebar, so its
/// description is the one here that genuinely cannot be a constant.
pub fn save_restore(saved: bool) -> ExampleMeta<'static> {
    meta(
        "Save / Restore",
        Some(if saved {
            "Graph saved! Move nodes and press 'r' to restore."
        } else {
            "Press 's' to save, move nodes, then 'r' to restore."
        }),
        vec![("s", "save"), ("r", "restore")],
    )
}
