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

/// The BROWSER stress test's metadata.
///
/// `examples/stress_test.rs` is a different program with its own inline meta:
/// the native one is driven by CLI flags, this one by URL params, and their key
/// lists differ. So this is not a shared definition, it is the web port's own —
/// named like [`overview_web`] for that reason. It lives here rather than
/// inline in the wasm crate because this module is where example prose lives,
/// and prose inline in a source file is what drifted last time.
pub fn stress_test_web() -> ExampleMeta<'static> {
    meta(
        "Stress Test",
        Some(
            "Default: 25x25 grid (625 nodes, 624 edges).\nResize via URL, e.g. ?size=50#stress-test or ?cols=30&rows=20#stress-test",
        ),
        vec![
            ("t", "drag test"),
            ("s", "select test"),
            ("r", "remount test"),
            ("a", "run all"),
            ("l", "log frames"),
        ],
    )
}

/// The BROWSER overview's metadata.
///
/// `web/wasm/src/overview.rs` is a port, not a build of `examples/overview.rs`:
/// the terminal-only nodes (ratatui-image, tachyonfx) are a bar chart there,
/// because a browser has no image protocol to draw them with.
///
/// That makes [`overview`]'s description actively wrong on the web — it asks
/// for Kitty/iTerm2/Sixel, none of which mean anything in a tab — and its two
/// extra keys dead: the port forwards to the default bindings only, so `p`
/// (image mode) and `Enter` (toggle input) would be advertised and do nothing.
/// Hence a second definition rather than a shared one. The two programs differ,
/// so their descriptions are allowed to.
pub fn overview_web() -> ExampleMeta<'static> {
    meta(
        "Overview",
        Some(
            "Custom nodes, edges, and handles. Nodes via ratatui widgets, a raw buffer, and a sparkline.\nThe native example's terminal-only nodes (ratatui-image, tachyonfx) are a bar chart here.",
        ),
        vec![],
    )
}

// ============================================================================
// The website's list
// ============================================================================

/// One example as the website publishes it.
///
/// The web build turns each of these into a real route (`/examples/<slug>/`)
/// instead of the hash fragment it used to be, so `source` is here to let the
/// page render the example's module doc and its code — the two things a reader
/// (and a crawler) cannot get out of a WebGl2 canvas.
pub struct WebExample {
    /// Last segment of the URL, and the hash the app has always accepted.
    pub slug: &'static str,
    /// Path to the example, relative to the repository root.
    ///
    /// The NATIVE one, always, even for the two that the browser runs a port
    /// of. rataflow builds terminal UIs; a reader of the site is deciding
    /// whether to use it in one, and the code they would copy is this. The
    /// browser build is how they watch it move without `cargo run` first.
    pub source: &'static str,
    /// The port the browser actually runs, where it is a different program.
    ///
    /// `None` for the twenty-one examples the wasm build compiles as they are.
    /// For the two it does not, the page shows a short note saying what changed,
    /// taken from this file's own module doc — which already explains it,
    /// because the file doing the diverging is the one that knows.
    pub web_source: Option<&'static str>,
    pub meta: ExampleMeta<'static>,
}

/// Every example the website ships, in sidebar order.
///
/// Not every example in `examples/` is here: `basic_async` and `termion_basic`
/// are native-only (a tokio runtime and a termion backend, neither of which the
/// browser build has), so they keep their metadata above without a route.
///
/// `web/wasm/src/main.rs` maps these same slugs to the demo each one builds.
/// That mapping is behaviour and has to live with the constructors; this one is
/// text. `build.sh check` compares the two so a slug added to one and not the
/// other is caught rather than silently serving the overview.
pub fn web_examples() -> Vec<WebExample> {
    fn e(slug: &'static str, source: &'static str, meta: ExampleMeta<'static>) -> WebExample {
        WebExample {
            slug,
            source,
            web_source: None,
            meta,
        }
    }
    /// An example the browser runs a port of, not the example itself.
    fn ported(
        slug: &'static str,
        source: &'static str,
        web_source: &'static str,
        meta: ExampleMeta<'static>,
    ) -> WebExample {
        WebExample {
            slug,
            source,
            web_source: Some(web_source),
            meta,
        }
    }
    vec![
        ported(
            "overview",
            "examples/overview.rs",
            "web/wasm/src/overview.rs",
            overview_web(),
        ),
        e("basic", "examples/basic.rs", basic()),
        e("view-only", "examples/view_only.rs", view_only()),
        e("custom-nodes", "examples/custom_nodes.rs", custom_nodes()),
        e("node-flags", "examples/node_flags.rs", node_flags()),
        e("hierarchy", "examples/hierarchy.rs", hierarchy()),
        e("custom-edges", "examples/custom_edges.rs", custom_edges()),
        e("edge-routing", "examples/edge_routing.rs", edge_routing()),
        e(
            "floating-edges",
            "examples/floating_edges.rs",
            floating_edges(),
        ),
        e(
            "animating-edges",
            "examples/animating_edges.rs",
            animating_edges(),
        ),
        e("reconnection", "examples/reconnection.rs", reconnection()),
        e("multi-select", "examples/multi_select.rs", multi_select()),
        e("context-menu", "examples/context_menu.rs", context_menu()),
        e(
            "custom-bindings",
            "examples/custom_bindings.rs",
            custom_bindings(),
        ),
        e("events", "examples/events.rs", events()),
        e("validation", "examples/validation.rs", validation()),
        e(
            "companion-widgets",
            "examples/companion_widgets.rs",
            companion_widgets(),
        ),
        e(
            "custom-layout",
            "examples/custom_layout.rs",
            custom_layout(),
        ),
        e("undo-redo", "examples/undo_redo.rs", undo_redo()),
        e("mutations", "examples/mutations.rs", mutations()),
        e("theming", "examples/theming.rs", theming()),
        e(
            "save-restore",
            "examples/save_restore.rs",
            save_restore(false),
        ),
        ported(
            "stress-test",
            "examples/stress_test.rs",
            "web/wasm/src/stress_test.rs",
            stress_test_web(),
        ),
    ]
}
