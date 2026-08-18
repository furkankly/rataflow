# rataflow Architecture

Library for building node-based UIs in the terminal — from a static diagram to a fully interactive editor. Built on [ratatui](https://github.com/ratatui/ratatui), inspired by [xyflow](https://github.com/xyflow/xyflow).

## Layer Overview

```
┌─────────────────────────────────────────────────────────────┐
│  User Code                                                  │
│  Creates Node<N>, Edge<E> with positions and content        │
│  Uses Flow for rendering                                    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Types Layer (src/types/)                                   │
│  Node, Edge, Handle, Position, Dimensions, Rect, Viewport   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  State Layer (src/state/)                                   │
│  Flow, DragState, EdgePreview, RenderContext                │
│  Event handlers, viewport/zoom methods                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  UI Layer (src/ui/)                                         │
│  Path computation, EdgeStyle, HandleStyle                   │
│  Built-in content: TextContent, StepEdge, StraightEdge      │
└─────────────────────────────────────────────────────────────┘
```

## Coordinate System

Three coordinate spaces:

1. **World** (f64) — logical space where nodes and edges exist
2. **Canvas** — after viewport transform (pan/zoom)
3. **Terminal** — final buffer positions (canvas + offset)

```
world → (Viewport: pan/zoom) → canvas → (RenderContext: offset) → terminal
```

### Transform Pipeline

```
node.position (user-specified)
       ↓
get_position_with_origin() applies NodeOrigin
       ↓
position_absolute (top-left corner in world coords)
       ↓
viewport.world_to_canvas()
       ↓
render_context.canvas_to_terminal()
       ↓
terminal coordinates
```

### NodeOrigin

Defines which point of a node the `position` refers to:
- `TOP_LEFT` (0.0, 0.0), `CENTER` (0.5, 0.5), `BOTTOM_RIGHT` (1.0, 1.0)

### Hierarchy

Child positions are relative to parent's `position_absolute`. Resolved BFS-style.

Because the stored position means something different depending on the parent, changing a parent is a coordinate change too. `set_node_parent` rebases so the node holds still on screen; `node_bounds` reports where a node actually sits, since `Node::position` alone cannot say. See [INTERNALS](INTERNALS.md#re-parenting).

## Selection Model

Per-entity source of truth: `node.selected` / `edge.selected` are the sole authority — not stored centrally in Flow.

Multi-select mode is toggled via `FlowAction::ToggleMultiSelect` (default key: `m`). While active, clicks toggle individual selections without clearing others.

### Keyboard Navigation

Two complementary modes:

| Mode | Actions | Default Keys | Behavior |
|------|---------|-------------|----------|
| **Directional** (spatial) | `SelectUp/Down/Left/Right` | Arrow keys | Weighted nearest-neighbor in the 180° forward hemisphere |
| **Sequential** (insertion order) | `SelectNext/SelectPrev` | Tab / Shift+Tab | Wrapping cycle through nodes in insertion order |

**Directional algorithm:** `score = distance × (1 + k × sin(angle))` where `k = 2.0` (fixed, not configurable). Candidates behind the current node (>90° off-axis) are excluded. The lowest-scoring candidate wins. If no candidate exists in the forward hemisphere, selection stays put (no wrapping — wrapping in 2D is confusing). Hidden nodes are skipped. If nothing is selected, any direction selects the first node (same as `SelectNext` from nothing).

Uses `position_absolute` (node center) from `InternalNode` — the same world-space positions used for rendering. Ancestor nodes (parent, grandparent, ...) of the current selection are skipped — directional navigation moves between peers, not into your own container. Ancestors remain reachable via sequential navigation (Tab), mouse click, or by navigating from outside the container. No unreachable nodes: the four 180° hemispheres cover all 360°, and each direction makes strictly monotonic progress along its primary axis.

**Camera reveal (`SelectionReveal`).** After a *keyboard* selection change — not mouse-click or programmatic `select_node`, which leave the camera alone — the viewport responds per the `Flow::with_selection_reveal` policy: `EnsureVisible` (default — pan the minimum to bring the node fully on-screen, no-op if already visible), `Center` (center it in one move), or `None` (leave the viewport untouched, for consumers driving their own camera such as an eased glide). The reveal is applied *after* the selection moves, so the selection change and the `SelectionChanged` event fire regardless of the policy — only the camera response differs. This keeps the good default (a bare consumer sees the selected node) while letting a consumer with its own camera opt out cleanly instead of fighting a built-in pan.

## Connection Validation

Three-layer model checked in order during interactive edge creation:

| Layer | What | API |
|-------|------|-----|
| 1. Connectability | Can this handle participate? | `Node.connectable`, `Handle.connectable*` |
| 2. Mode | Are these handle types compatible? | `ConnectionMode::Strict` / `Loose` |
| 3. Validator | Custom business logic | `set_connection_validator(\|conn\| bool)` |

Connections are **normalized** based on drag start handle type — if you start from a target handle, source/target are swapped. This affects edge direction and marker placement.

**Edge IDs are deterministic:** `Connection::edge_id()` generates IDs from endpoints (`{source}:{source_handle}<>{target}:{target_handle}`). Same endpoints always produce the same ID, eliminating collision risk. `connection_exists()` provides endpoint-based duplicate detection (checked during drag for visual feedback, and as a safety net in `add_edge_from_connection`).

### Visual Feedback

**During drag:** green (valid), red (invalid), yellow (no target)

**Handles:** gray ● (default for both types), ◐ (partial restriction), ○ (disabled)

## Rendering

### Widget Render Order

Terminal rendering has no z-index — last write wins. Widgets must be rendered in the correct order:

```
1. Background(&flow)  (bottom layer — dot/line pattern)
2. &mut flow          (main content — edges, nodes, handles)
3. Controls / MiniMap (overlays — rendered on top of flow)
```

### Render Order and Z-Index

Within the canvas, rendering order determines layering:

```
1. Edges to separate buffer (symbol merging at intersections)
2. Edge preview to edge buffer
3. Composite edge buffer onto main buffer
4. Nodes + handles in z-order (body then handles per node)
```

Nodes render in z-order: `(effective_z, insertion_index)`. Each node's handles render immediately after its body, so a front node's body naturally occludes a behind node's handles.

**Effective z-index:** `node.z_index + (selected && elevate_nodes_on_select ? DEFAULT_SELECTED_NODE_Z : 0)`, then children are guaranteed above their parent (`parentZ >= childZ ? parentZ + 1 : childZ`). This matches xyflow's basic mode — selecting a parent elevates its children too. `DEFAULT_SELECTED_NODE_Z` is 1000. `elevate_nodes_on_select` defaults to `true`. Insertion index breaks ties (mimicking browser paint order / DOM order).

**Hit testing** uses the same z-order in reverse — front nodes are tested first. Within a single node, handles take priority over body.

**Z-order cache:** Computed lazily with a dirty flag (`z_order_dirty`). Invalidated on node add/remove, selection change, or z-index mutation. Recomputed once per frame via `ensure_z_order()`.

### Terminal Rendering Gaps vs xyflow

Edges render in a separate buffer (required for terminal symbol merging at intersections), making interleaved edge/node rendering infeasible. This creates behavioral differences from xyflow:

| Gap | Consequence |
|-----|-------------|
| Edges always below all nodes | Cannot layer an edge above a node |
| No edge z-index | `Edge.z_index` would be a dead field — omitted entirely |
| No edge elevation on select | Selected edges don't rise above nodes |
| No edge elevation from connected nodes | Selecting a node doesn't elevate its connected edges |
| Edges through node bodies render below | Visually, edges always render below nodes; however, hit testing uses implicit edge z-index (`max(source_z, target_z)`) so edges between children are clickable through a non-opaque parent (`node.opaque = false`) |

| Connection preview renders below nodes | xyflow renders previews at z-index 1001 (above selected nodes) |

These are inherent to terminal rendering — rendering above a node overwrites its content.

### Rendering Model

Three distinct approaches — this asymmetry is intentional:

| Element | Custom rendering? | Library primitive |
|---------|------------------|------------------|
| **Nodes** | Yes (`NodeContent`) | None — use ratatui directly |
| **Edges** | Yes (`EdgeContent`) | `EdgeStyle` + `EdgeRenderContext::render_path()` |
| **Handles** | No — library-rendered | `HandleStyle` on `Handle` instances |

### Node Rendering

No library-level style primitive. `NodeContent::render()` receives a `NodeRenderContext` and renders into `ctx.area` using ratatui widgets directly. The library renders handles separately.

Each visible node is rendered into a per-node scratch buffer at local `(0, 0)` coordinates with full dimensions, then only the visible portion is composited onto the main buffer. This solves two problems: (1) content always sees the complete area regardless of viewport clipping, so borders, layout, and positioning remain correct for partially off-screen nodes; (2) it sidesteps ratatui's u16 coordinate space — nodes extending off the left/top edge have negative terminal positions that can't be represented in `Rect`. The overhead is negligible even at scale — most nodes are culled by the visibility check before any buffer is allocated, each buffer is node-sized (typically ~10x3 cells), and frame time is dominated by edge path computation and the canvas-sized edge buffer, not node rendering.

Nodes are opaque by default (`node.opaque = true`) — the entire node area blocks content behind it, even cells not written by `NodeContent::render()`. Set `opaque: false` on parent nodes in hierarchical graphs so edges and children remain visible inside the parent.

### Accessing App State in Renders

`NodeContent::render()` receives `self` (your content data) and a `NodeRenderContext` (library-provided metadata). Because content lives inside `Flow` — the graph widget needs it for hit testing, dragging, and connection validation, not just rendering — it's persistent rather than constructed fresh each frame like ratatui widgets.

State that affects rendering falls into three categories:

| Category | Example | Pattern |
|----------|---------|---------|
| **Per-node owned state** | Label, color, collapse state | Field on content (`self`) |
| **Global app state** | Mode enum, display config | `Rc<Cell<T>>` shared across all nodes |
| **Per-node derived state** | "Am I being edited?" | Synced field from app |

**Per-node owned state** is what `self` in `NodeContent::render()` is for — data that belongs to the node. No sync needed, it's already there.

**Global app state: `Rc<Cell<T>>`.** State that's the same for every node — a mode enum, a display flag. Clone a shared `Rc<Cell<T>>` into each content struct at construction. All nodes read via `get()`, mutations via `set()` — no per-frame sync, no runtime borrow checking:

```rust
use std::cell::Cell;
use std::rc::Rc;

#[derive(Debug, Clone)]
struct MyContent {
    label: String,
    mode: Rc<Cell<Mode>>,  // shared across all nodes
}

impl NodeContent for MyContent {
    fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {
        match self.mode.get() {
            Mode::Compact => { /* ... */ }
            Mode::Detailed => { /* ... */ }
        }
    }
}

// Construction — clone the Rc into each node
let mode = Rc::new(Cell::new(Mode::Normal));
let nodes = vec![
    Node::new("a", (10.0, 10.0), MyContent {
        label: "Node A".into(),
        mode: mode.clone(),
    }),
];

// Mutation — all nodes see the change immediately
mode.set(Mode::Compact);
```

For non-`Copy` global state (`String`, `Vec`, etc.), use `Rc<RefCell<T>>` instead — same pattern but with `borrow()` / `borrow_mut()`. In practice borrows never overlap since mutations happen during event handling and reads during rendering.

**Per-node derived state: sync loop.** State that depends on both app state and node identity — "am I the node being edited?" is `app.editing_id == node.id`, which the node can't compute on its own. Push the answer down with a sync loop before rendering:

```rust
#[derive(Debug)]
struct MyContent {
    label: String,
    mode: Rc<Cell<Mode>>,   // global — shared via Rc
    is_editing: bool,        // derived — synced from app
}

// Sync per-node derived state before rendering
let node_ids: Vec<_> = flow.nodes().map(|n| n.id.clone()).collect();
for id in &node_ids {
    if let Some(content) = flow.node_content_mut(id) {
        content.is_editing = editing_id.as_deref() == Some(id.as_str());
    }
}
```

The O(n) sync is negligible at TUI scale — setting a bool on each node is orders of magnitude cheaper than the edge path computation and buffer allocation that dominate frame time.

The same patterns apply to `EdgeContent` — edges have the same `self` access in `compute_path()` and `render()`.

### Auto-Pan

When dragging a node or connection toward the canvas edge, the viewport automatically pans in that direction — matching xyflow's `autoPanOnNodeDrag` / `autoPanOnConnect` behavior. The velocity ramps linearly within a 5-cell edge zone.

| Config | Default | What |
|--------|---------|------|
| `auto_pan_on_node_drag` | `true` | Pan during node drag |
| `auto_pan_on_connect` | `true` | Pan during connection/reconnection drag |
| `auto_pan_speed` | `110.0` | Speed in canvas cells/second |

The library doesn't own the event loop, so the app must call `tick_auto_pan(elapsed)` each frame to drive panning. Without the call, auto-pan is inert. Returns `EventResponse` with `ViewportChanged` and `NodeDragged` events when panning occurs.

### App Loop

A ratatui app using rataflow has two phases per frame:

```
loop {
    // 0. Tick — advance time-based state
    let now = Instant::now();
    flow.tick_auto_pan(now - last_tick);
    flow.tick_animation(now - last_tick);
    last_tick = now;

    // 1. Input — mutate app state + Flow
    handle_input(&mut app, &mut flow);

    // 2. Render
    terminal.draw(|frame| {
        frame.render_widget(Background::new(&flow), area);
        frame.render_widget(&mut flow, area);
        frame.render_widget(Controls::new(&flow), area);
        frame.render_widget(MiniMap::new(&flow), area);
    })?;
}
```

If you have per-node derived state to sync, do it before rendering:

```rust
fn sync_state(&self, flow: &mut Flow<MyContent, MyEdge>) {
    // Content sync — per-node derived state, one pass
    for (id, content) in flow.nodes_content_mut() {
        content.is_editing = self.editing_id.as_deref() == Some(id);
    }

    // Library property sync — handle styles, behavioral flags
    for id in &node_ids {
        let selected = flow.node(&id).map_or(false, |n| n.selected);
        let charset = if selected { &THICK } else { &PLAIN };
        flow.set_handle_styles(&id, Some(
            HandleStyle::directional(charset[0], charset[1], charset[2], charset[3],
                Style::default().fg(self.mode_color(selected)))
        ));
        flow.set_node_draggable(&id, self.mode.allows_drag());
    }
}
```

The cost is O(n) per frame, negligible at TUI scale.

### Edge Rendering — 3 Layers

```
┌──────────────────────────────────────────────────────────┐
│  Layer 1: Path Computation (pure geometry)               │
│  compute_step_path(), compute_straight_path()            │
│  Path::hit_test()                                        │
└──────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────┐
│  Layer 2: Path Rendering                                 │
│  ctx.render_path(style, label, buf)                      │
│  EdgeStyle configures characters, markers, resolution    │
└──────────────────────────────────────────────────────────┘
                              ↓
┌──────────────────────────────────────────────────────────┐
│  Layer 3: Builtins                                       │
│  StepEdge, StraightEdge compose layers 1 + 2             │
└──────────────────────────────────────────────────────────┘
```

Users can hook in at any level: use builtins, custom path + standard rendering, or fully custom.

### Handle Rendering

Library-rendered — no custom handle rendering trait. `HandleStyle` configures the marker character and style on each `Handle` instance.

### Render Contexts

Non-generic, flattened views of render metadata. Content is available as `self` in trait methods, so contexts expose only what's needed for rendering:

**NodeRenderContext:**
- `id`, `area`, `selected`, `dragging`, `position_absolute`

**EdgeRenderContext:**
- `id`, `selected`, `label`, `path`, `source_position`, `target_position`
- Helper methods: `render_path()`, `world_to_terminal()`, `is_in_bounds()`

The `world_to_terminal()`/`is_in_bounds()` escape hatches also exist on `Flow` itself (alongside `node_terminal_rect()`) for app-drawn overlays — badges, tooltips, activity indicators drawn right after the flow in the same draw pass. The two surfaces behave identically (both are one-line delegations reading the same viewport and canvas area); two access points exist only because content renders without access to the `Flow`. Same contract everywhere: transforms return raw, possibly off-canvas i32 coordinates; callers clip per cell with `is_in_bounds()`.

## Style Types

All style structs use the same pattern: private fields, `Default` + builders, `Option<Style>`/`Option<Color>` for theme-derived values. `None` means "use the current theme" — resolved at render time via `resolved_style(palette)`. Structural fields (characters, markers, booleans) are concrete values with sensible defaults since they don't vary by theme.

```rust
// EdgeStyle — structure + optional color
EdgeStyle::default()       // Unicode box-drawing + arrow marker, color from theme
EdgeStyle::ascii()         // ASCII characters, color from theme
EdgeStyle::braille()       // braille strokes (2x4 sub-cell), color from theme
EdgeStyle::default()       // Explicit color override
    .with_stroke_style(Style::default().fg(Color::Indexed(248)))

// HandleStyle — structure + optional color
HandleStyle::default()     // ●, color from theme
HandleStyle::ascii()       // o, color from theme
HandleStyle::new('◉', Style::default().fg(Color::Cyan))  // explicit color

// Companion widget styles — all colors optional
MiniMapStyle::default().with_node_color(Color::Red)  // only node color overridden
ControlsStyle::default().with_zoom_in_char('+')      // only char changed, colors from theme
EdgePreviewStyle::default().with_valid_color(Color::Cyan)  // only valid color overridden
EdgePreviewStyle::default().with_stroke(EdgeStroke::Braille)  // rasterization; color from validation
```

### Theming

All default colors come from a [`Theme`] → [`Palette`] system. `Theme` is an enum (`Dark`, `Light`) that maps to a `Palette` with 8 semantic color fields (`canvas_bg`, `surface`, `muted`, `subtle`, `accent`, `text`, `success`, `error`).

Set `Flow::theme` to switch all library-rendered elements at once:

```rust
let flow = Flow::new().with_theme(Theme::Light);
```

Companion widgets take `&Flow` and read the theme directly at render time. Background, Controls, MiniMap, handles, and edge preview all derive their colors from the theme automatically. Style overrides on individual widgets (e.g., `Background::style()`) take precedence when set.

**Content theming:** Both render contexts (`NodeRenderContext`, `EdgeRenderContext`) expose `ctx.theme`, so all content types — built-in and custom — resolve theme colors at render time. Setting `flow.theme` is enough to switch everything:

```rust
flow.theme = Theme::Light; // all elements update on next render
```

Built-in types (`TextContent`, `StepEdge`, `StraightEdge`) use `Option` style fields — `None` means "use theme", `Some(style)` means "explicit override":

```rust
// Theme default — no style fields needed
let content = TextContent::from("Hello");

// Per-instance override
let content = TextContent::from("Custom")
    .with_border_style(Style::default().fg(Color::Green));
```

`EdgeRenderContext::render_path()` is selection-aware: when `stroke_style` is `None`, it resolves to `palette.accent` for selected edges and `palette.muted` for normal edges. Custom edge types get selected highlighting for free.

Custom `NodeContent` implementations can use `ctx.theme.palette()` directly:

```rust
fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {
    let palette = ctx.theme.palette();
    let border_color = if ctx.selected { palette.accent } else { palette.muted };
    // ...
}
```

The `Palette` struct has `pub` fields and `const DARK` / `const LIGHT` values, so users can also construct custom palettes directly.

## API Design

Two rules govern field visibility:

1. **`pub` when users read, private + builders when users only configure.** Types whose fields users query (`node.selected`, `flow.min_zoom`) use pub fields — making them private would just mean a getter for every field. Types users only write during construction and never inspect (style structs, companion widgets) use private fields + builders, which buys non-exhaustive safety for free.
2. **Side effects require operations.** Flow graph state (`pub(crate)`) has side effects on write (hierarchy resolution, index rebuilding), so mutations go through setters and operations. Flow config (`pub`) has no side effects, so users assign directly.

### By type category

| Category | Examples | Fields | Builders | Why |
|----------|----------|--------|----------|-----|
| **DTOs** | Node, Edge, Handle, Position, Viewport | `pub` | `with_*` (construction sugar) | Users read fields back; borrow checker controls mutation via `&T` |
| **Flow config** | min_zoom, locked, connection_mode, etc. | `pub` | `with_*` (construction sugar) | Users read and write; no side effects |
| **Flow graph** | nodes, edges, lookups, drag state, edge preview | `pub(crate)` | `with_*` (triggers computation) | Side effects on write (hierarchy, indexes) |
| **Content widgets** | TextContent, StepEdge, StraightEdge | `pub` | `with_*` (construction sugar) | Users read and write via `content_mut()` |
| **Style structs** | EdgeStyle, HandleStyle, EdgePreviewStyle, ControlsStyle, MiniMapStyle, BackgroundStyle | private | `with_*` (required) | Users only configure, never read back |
| **Companion widgets** | Controls, MiniMap, Background | private | `with_*` (ratatui convention) | Users only configure; take `&Flow` reference |

### DTO Field Categories

Node and Edge fields are `pub` (readable via `&T`), but Flow only exposes `&Node` / `&Edge` — never `&mut`. This is intentional: some fields have side effects on write, and others can break invariants entirely. Fields fall into three categories:

**Identity — set at construction, never mutate at runtime.**

Changing these breaks internal invariants (lookup HashMaps, edge references, hierarchy) that no recomputation can fix.

| Type | Fields |
|------|--------|
| Node | `id`, `parent_id`, `handles` (add/remove/reorder), `source_position`, `target_position` |
| Edge | `id`, `source`, `target`, `source_handle`, `target_handle` |

**Geometry — mutate via dedicated setters that trigger recomputation.**

These affect cached computed state (absolute positions, handle bounds, z-order). Each setter triggers the specific recomputation needed.

| Setter | Fields | Side effect |
|--------|--------|-------------|
| `set_node_position` | `position` | `resolve_hierarchy()` |
| `set_node_dimensions` | `width`, `height` | `resolve_hierarchy()` |
| `set_node_z_index` | `z_index` | `invalidate_z_order()` |

Node fields `origin`, `extent`, `expand_parent` also affect hierarchy resolution but are set at construction — no runtime setter because changing these on a live graph is architecturally suspect.

**Configuration — mutate via setters, no side effects.**

Purely behavioral or visual flags read at interaction/render time. No cached state depends on them. Setters exist because Flow only exposes `&Node` / `&Edge`, not `&mut`.

| Node setters | Edge setters |
|-------------|-------------|
| `set_node_hidden` | `set_edge_hidden` |
| `set_node_selectable` | `set_edge_selectable` |
| `set_node_deletable` | `set_edge_deletable` |
| `set_node_draggable` | `set_edge_animated` |
| `set_node_connectable` | `set_edge_reconnectable` |
| `set_node_opaque` | `set_edge_label` |
| `set_handle_styles` / `set_handle_style` | |
| `set_handle_disabled_styles` / `set_handle_disabled_style` | |
| `set_handles_hidden` / `set_handle_hidden` | |

Content is the fourth category but follows a different pattern: `node_content_mut()` / `edge_content_mut()` return `&mut N` / `&mut E` directly because content is opaque to the library — no invariants to protect. `nodes_content_mut()` / `edges_content_mut()` do the same in bulk, yielding `(&str, &mut N)` / `(&str, &mut E)` so a whole-graph content sync doesn't have to collect IDs to end the read borrow first.

### Flow specifically

Flow has two kinds of fields:

- **`pub` config** — plain settings users read and write with no side effects. Builders (`with_min_zoom`, `with_locked`, etc.) provide construction-time sugar; direct assignment (`flow.min_zoom = 0.3`) works for runtime changes.
- **`pub(crate)` graph state** — nodes, edges, lookups, drag state, edge preview. Writing these has side effects (hierarchy resolution, index rebuilding), so users go through setters (`set_node_position`) and operations (`add_node`, `select_node`, `pan`).

Builders that trigger computation (`with_graph`, `with_uniform_width`) are for construction-time setup, not runtime config.

### FlowOps (object-safe trait)

`Flow<N, E>` is generic over content types, which prevents `dyn` dispatch. `FlowOps` extracts the ~35 methods whose signatures don't mention `N` or `E` into an object-safe trait with a blanket impl for all `Flow<N, E>`.

**When you need it:** type-erased contexts like `Box<dyn Demo>` where concrete `N`/`E` aren't known. The pattern is to expose `&mut dyn FlowOps` from a trait method:

```rust
trait Demo {
    fn flow_ops(&mut self) -> &mut dyn FlowOps;
}

// Callers use flow_ops() for viewport, selection, animation, etc.
demo.flow_ops().zoom_in();
demo.flow_ops().request_fit_view();
demo.flow_ops().tick_animation(elapsed);
```

**When you don't need it:** direct `Flow` usage. Inherent methods take priority over trait methods, so existing code is unaffected. You never need to import `FlowOps` for normal use.

**What's included:** event handling (`handle_key_event`, `handle_mouse_event`, `apply`), viewport (`pan`, `zoom_in`, `request_fit_view`, `center_on`, ...), selection (`select_node`, `select_next_edge`, `clear_selection`, ...), graph mutation (`set_node_position`, `set_node_dimensions`, `clear`, ...), animation (`tick_animation`), and state queries (`is_dragging`, `canvas_area`, ...).

**What's excluded:** methods that mention `N` or `E` in their signature — `add_node`, `add_edge`, `node_content_mut`, `add_edge_from_connection`, iterators over `Node<N>`/`Edge<E>`, etc. These remain inherent on `Flow`.

**`impl Into<Position>` note:** three inherent methods use `impl Into<Position>` (`set_node_position`, `move_node`, `center_on`). The trait versions take `Position` directly for object safety. Both coexist — inherent methods win for direct calls.

## Actions and Events

Input/output separation inspired by xyflow's callback model, adapted for immediate-mode TUI:

| Type | Direction | Purpose |
|------|-----------|---------|
| `FlowAction` | Input | Semantic actions (what to do) |
| `EventResponse` | Output | Routing signal + optional semantic event |
| `FlowEvent` | Output | Semantic events (what happened), wrapped in `EventResponse::Event(Vec<_>)` |

### FlowAction (Input)

Semantic actions for keyboard interaction. Three tiers, from most to least common:

| Tier | Entry point | When to use |
|------|-------------|-------------|
| **Default bindings** | `flow.handle_key_event(key.into())` | Most users — handles key mapping and event emission |
| **Custom bindings** | `flow.apply(FlowAction::...)` | You map your own keys to actions |
| **Direct methods** | `flow.select_node("a")`, `flow.pan(...)` | Programmatic mutation — no events emitted |

```rust
// Tier 1: default bindings (most users)
flow.handle_key_event(key.into());

// Tier 2: custom bindings
fn my_bindings(key: &KeyEvent) -> Option<FlowAction> {
    match key.code {
        KeyCode::Char('x') => Some(FlowAction::Delete),
        _ => default_flow_key_binding(key), // fall back to defaults
    }
}
if let Some(action) = my_bindings(&key.into()) {
    flow.apply(action);
}

// Tier 3: programmatic (setup, scripting, tests)
flow.select_node("a");
flow.pan(10.0, 0.0);
```

### EventResponse (Output)

All event handlers return [`EventResponse`] — a three-variant enum that separates routing signals from semantic events:

| Variant | Meaning |
|---------|---------|
| `NotHandled` | Input was not consumed — fall through to next handler |
| `Handled` | Input was consumed but produced no semantic event |
| `Event(FlowEvent)` | Input produced a semantic event the app may react to |

This is a return-value pattern, not a callback pattern. Unlike xyflow which fires multiple independent callbacks per interaction (`onNodeClick` + `onSelectionChange`), we return a single `EventResponse` containing all events from one interaction.

The `NotHandled` variant enables **handler chaining** — try one handler, fall through to the next if it didn't consume the input:

```rust
let response = flow.handle_controls_key_event(key.into());
if matches!(response, EventResponse::NotHandled) {
    flow.handle_key_event(key.into());
}
```

### FlowEvent (Semantic Events)

The payload inside `EventResponse::Event(vec![...])`. One interaction can produce multiple events (e.g., `NodeClicked` + `SelectionChanged`). Use `into_events()` to iterate, or `events()` for a borrowed slice.

**Naming convention:** events are named after interactions an app would commonly run code in response to. Gesture events use the interaction name (`NodeClicked`, `NodeDragStarted`). State-change events use the outcome name (`ViewportChanged`, `SelectionChanged`, `Deleted`) — these aggregate multiple input sources into a single event. If an app would only "render differently" in response, read state during render instead of adding an event.

**No implicit graph mutations.** Events are purely informational — the library does not add, remove, or modify nodes/edges in response to events. `ConnectionCompleted` means a drag gesture completed; call `add_edge_from_connection` to actually add the edge. Selection is the one implicit side effect: clicking a node/edge selects it before the event is returned.

```rust
for event in flow.handle_mouse_event(mouse.into()).into_events() {
    match event {
        FlowEvent::NodeClicked { node_id } => show_details(&node_id),
        FlowEvent::ConnectionCompleted(conn) => {
            flow.add_edge_from_connection(conn, StepEdge::default());
        }
        FlowEvent::SelectionChanged { node_ids, .. } => update_sidebar(&node_ids),
        _ => {}
    }
}
```

| FlowEvent | Data | When |
|-----------|------|------|
| `NodeClicked` | `{ node_id }` | Node was clicked (and selected) |
| `EdgeClicked` | `{ edge_id }` | Edge was clicked (and selected) |
| `PaneClicked` | `{ x, y }` | Empty canvas clicked (world coords) |
| `NodeContextMenu` | `{ node_id }` | Node right-clicked (selection unchanged) |
| `EdgeContextMenu` | `{ edge_id }` | Edge right-clicked (selection unchanged) |
| `PaneContextMenu` | `{ x, y }` | Empty canvas right-clicked (world coords) |
| `NodeResizeStarted` | `{ node_id }` | Began resizing from the bottom-right grip |
| `NodeResized` | `{ node_id }` | Node is being resized (ongoing) |
| `NodeResizeEnded` | `{ node_id }` | Finished resizing |
| `ConnectionStarted` | `{ node_id, handle_id }` | Began dragging from handle |
| `ConnectionCompleted` | `Connection` | Connection drag completed — call `add_edge_from_connection` to add |
| `ConnectionCancelled` | — | Released without valid target, or cancelled via keyboard |
| `NodeDragStarted` | `{ node_id }` | Began dragging node (threshold exceeded) |
| `NodeDragged` | `{ node_id }` | Node is being dragged (ongoing movement) |
| `NodeDragEnded` | `{ node_id }` | Finished dragging node |
| `ViewportChanged` | `{ x, y, zoom }` | Pan or zoom occurred (keyboard, mouse, scroll) |
| `SelectionChanged` | `{ node_ids, edge_ids }` | Selection changed (any source) — diff-checked, only emitted when different from handler entry |
| `Deleted` | `{ node_ids, edge_ids }` | Elements were deleted |

**Note:** `NodeClicked` fires on mouse up if drag threshold was not exceeded. `NodeDragStarted` only fires when movement exceeds `node_drag_threshold` (default 2.0 world units), distinguishing clicks from drags.

**SelectionChanged diff check:** `SelectionChanged` is only emitted when the selection actually differs from its state at handler entry. Handlers snapshot the selection at the start of `apply()` / `handle_mouse_event()`, then compare after mutations. This suppresses spurious events (e.g., clicking an already-selected node, clearing an empty selection) and works correctly with programmatic mutations — if you call `select_node()` between handler calls, the next handler's snapshot captures that change.

### ControlsAction

Separate action type for the Controls widget. Both `Flow::apply()` and `Flow::apply_controls_action()` return `EventResponse` for consistent event handling.

| Enum | Binding function | Scope |
|------|-----------------|-------|
| `FlowAction` | `default_flow_key_binding` | Graph interaction |
| `ControlsAction` | `default_controls_key_binding` | Controls widget |

### Interaction Lock

The lock lives on `Flow` — `apply()` and `handle_mouse_event()` gate mutations internally, making it self-enforcing. Following xyflow's model:

- **Blocked when locked:** selection (sequential and directional), drag, connect, delete
- **Allowed when locked:** pan, zoom, fit view, center

The Controls widget reads the lock state from `&Flow` at render time.

## Companion Widgets

`Controls`, `MiniMap`, and `Background` are stateless widgets that take `&Flow` as a reference. Each has a corresponding Style type.

| Widget | Style | Purpose |
|--------|-------|---------|
| `Controls` | `ControlsStyle` | Viewport control panel (zoom, fit, lock) |
| `MiniMap` | `MiniMapStyle` | Scaled-down graph overview |
| `Background` | `BackgroundStyle` | Patterned background (dots, lines, crosses) |

Companion widgets read all needed state (viewport, theme, lock status, node positions, etc.) directly from the `&Flow` reference at render time — no sync step needed. Style types follow the same private-field + builder pattern as `EdgeStyle`/`HandleStyle`.

Rendering mode lives on the widget rather than its style, since style types are colors-only — `Background::variant()` selects the pattern. The minimap has no such knob: it always rasterizes at quadrant resolution.

## Serialization

Feature-gated behind `serde`. The serialization boundary follows xyflow's model: **serialize graph data, not presentation.**

`FlowSnapshot` is the serialization unit — `nodes`, `edges`, `viewport`. App-level config (`Flow` settings like `min_zoom`) and theme (`Palette`) are not included; apps set those in code, same as xyflow sets them as React props/CSS.

### What's serialized vs skipped

**Style fields are `#[serde(skip)]`** — `HandleStyle` on `Handle`, `EdgeStyle` on `StepEdge`/`StraightEdge`. These are presentation (chars, colors, markers) owned by the app, not graph data. On deserialization they get `Default`, and the app applies its current styles. This prevents style fossilization: when an app upgrades its visual identity, users' saved flows render with the new styles, not the old ones baked in at save time.

**Everything else on DTOs** is graph data and serializes normally, split into two categories:

**Required fields** — identity and geometry that must always be present. Deserialization fails if missing.

| Type | Required |
|------|----------|
| `Node` | `id`, `position`, `content`, `width`, `height` |
| `Edge` | `id`, `source`, `target`, `content` |
| `Handle` | `position`, `handle_type` |
| `Connection` | `source`, `target` |

**Behavioral fields** — have `#[serde(default)]` so they can be omitted. This serves two purposes: (1) forward compatibility when new fields are added in future versions, and (2) ergonomic hand-crafted JSON where users don't need to specify every flag. Everything else (booleans, options, enums with defaults) falls in this category.

Non-zero default helpers live in `types::serde_defaults`, gated behind `#[cfg(feature = "serde")]`. When adding new fields to DTOs, always annotate behavioral fields with `#[serde(default)]`. Required fields should NOT get defaults — missing data should fail loudly. Style fields should use `#[serde(skip)]`.

### Standalone serializable types

`Palette`, `Theme`, `HandleStyle`, `EdgeStyle`, and other public types derive `Serialize`/`Deserialize` as a convenience — apps can serialize them independently (e.g., saving theme preferences to a config file). This does not mean they belong in the snapshot.

---

See [INTERNALS.md](./INTERNALS.md) for implementation details.
