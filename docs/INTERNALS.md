# rataflow Internals

Implementation details and design decisions for contributors.

## Edge Routing

### Stem Length

Edges have a minimum distance they travel from handles before routing begins (`stem_length`). This prevents edges from turning immediately after leaving a handle, creating cleaner layouts when nodes are close together.

```
Without stem_length:        With stem_length:
[source]→[corner]→[target]  [source]→[stem]→[corner]→[stem]→[target]
```

`StepEdge` defaults to `stem_length=1.0`. Use `.with_stem_length(n)` to change.

### Adaptive Stem Length

Fixed stem lengths cause visual artifacts when nodes are close. Stems are automatically clamped based on handle orientation:

- **Opposite parallel** (e.g., Right→Left): stems face each other and share the available space — `effective_stem = min(stem_length, distance / 2)`
- **Same-direction parallel** (e.g., Right→Right): stems extend the same way, no crossing possible — full `stem_length` used
- **Mixed handles** (e.g., Right→Top): each stem clamped independently via dot-product projection along the handle's outward direction, preventing overshoot when the target is in the favorable direction

This is O(1) with no allocations.

### Edge Attachment

Where an edge attaches is a property of the **edge type**, not a mode on the graph. `EdgeContent::compute_path` receives an `EdgePathContext`: the endpoints the flow resolved from the edge's handles, plus the two node rectangles those handles sit on. Most edges use the endpoints. `FloatingEdge` ignores them and derives its own from the rectangles, so it re-attaches as the nodes move.

The side is chosen by `HandlePosition::facing`, which weighs the centre offset against each half-extent so the answer respects the node's shape rather than just `|dx|` vs `|dy|`. A wide, short node facing a target up and to the right exits through its *top*, because that edge lies nearer. This is the model xyflow uses for floating edges.

*Where on that side* is a second setting, `FloatingAttachment`, and the grid is what makes it a separate one. `Midpoint` gives four attachment points per node, so the endpoint holds still and then jumps when the facing side flips. `Perimeter` solves for where the line between the two centres crosses the outline — the intersection-point arithmetic xyflow also does — so the endpoint slides instead. A stepped route draws in box-drawing characters, which move a whole cell at a time and want their stem leaving a side's centre; a straight route draws in braille, whose 2x4 sub-cell dots can show a continuous slide. So each route defaults to the attachment that suits it and `with_attachment` overrides either way. Unset, the field is `None` rather than a concrete default, which is the same shape `style` on that type already uses for "whatever suits the route".

The rectangles are owned copies. An edge is being asked where to draw, not handed the graph — the same trade `Flow::node_bounds` makes.

This replaced an `EdgeAttachment::{Fixed, Floating}` enum on `Edge`, plus a `Node::with_side_handles` builder that existed only to give floating somewhere to land. Both were library-side policy on the wrong type: they made the *graph* carry a setting that belongs to one edge. The context is a capability instead, and `FloatingEdge` is one thing built on it. An edge wanting an attachment neither built-in offers — nearest point rather than centre-to-centre crossing, say, or a fixed offset along a side — writes it against the same two rectangles, with no variant added anywhere. xyflow reaches the same place from the other direction: its floating edges are userland, built on `useInternalNode`.

Two consequences worth naming. `Path` carries the sides it actually uses, so endpoint markers are oriented from the path rather than from the resolved handle — an edge that leaves from a side its handle does not sit on still gets its arrow on the right side. And because `compute_path` feeds both rendering and hit testing, whatever an edge derives is used by both; the two cannot disagree, however unusual the derivation.

`Flow::resolve_edge_handles` remains the single place handles are looked up, so `render_edges` and `edge_hits_at` still cannot drift apart on that.

## Edge Preview

The edge preview is a colored line from a source handle to a target position, shown during connection creation. The rendering reads from `EdgePreview` on `Flow` — a single source of truth written by both mouse drag handlers and the public keyboard API.

- Path: `E::default().compute_path()` — uses the default edge variant's path shape
- Stroke: `EdgePreviewStyle::style` — defaults to `EdgeStyle::without_markers()`: no arrow, just the colored path
- Color: `EdgePreviewStyle` on `Flow` — validation feedback (default: muted green=valid, muted red=invalid, light gray=no target)
- Target direction: uses the real target handle's position when resolved, falls back to `opposite()` for free-space dragging
- Render offsets: taken from the path's own sides, so a preview edge that picks its own endpoints is still oriented correctly; free-space dragging uses `(0, 0)` because the endpoint is the cursor, not a node border

The edge type supplies the preview's *shape*; `EdgePreviewStyle` owns how that shape is drawn. It holds an `EdgeStroke`, not an `EdgeStyle`, so it carries rasterization without carrying color — the validation state is the only thing that can set the color, and no precedence rule is needed because the two cannot both specify one.

Color was always resolved this way; only the structural half used to be hardcoded, which is why a braille or dotted edge previewed as box-drawing. Set via `Flow::with_preview_style()` / `set_preview_style()`.

`EdgePreview.to_world` is `Option<Position>` — `None` means connection mode is active but nothing is rendered yet (e.g., after `start_edge_preview()` before a target is set). `Some(pos)` triggers rendering.

This is why `EdgeContent` requires `Default`.

### Mouse vs Keyboard

Mouse drag code sets `edge_preview` as a side effect of entering `DragState::CreatingConnection` / `ReconnectingEdge`, updates it during drag via `update_edge_preview_to_position()`, and clears it on mouse-up or cancel. Keyboard-driven code calls `start_edge_preview()` → `preview_to_handle()` / `preview_to_node()` / `cycle_to_handle()` → `complete_edge_preview()` or `clear_edge_preview()` — same field, same validation, same rendering.

`preview_to_handle()` is the core primitive that validates and sets the to-handle. `preview_to_node()` finds the closest compatible handle, `cycle_to_handle()` steps through to-handles — both call `preview_to_handle()`. `cycle_from_handle()` changes the from-handle without touching the to-side, only re-validating `is_valid` — preserves to-handle selection when cycling the from-side.

### Shared Handle Filtering

`compatible_targets()` is a free function in `edge_preview.rs` that filters `HandleBounds` by connection mode and connectability flags (`connectable` and `connectable_end`). Both paths use it — the keyboard path via `validated_preview_target()` + `compatible_targets()`, the mouse path via `find_connectable_handle_by_position()` which calls `compatible_targets()` per node while scanning all nodes by proximity.

## Connection Validation

### 3-Layer Model

1. **Connectability flags** — `Node.connectable` (master switch), `Handle.connectable`, `Handle.connectable_start`, `Handle.connectable_end`
2. **Connection mode** — `Strict` (source→target only) vs `Loose` (any handle combination)
3. **Validator callback** — `set_connection_validator(|conn| bool)` for custom logic

### Connectability vs Edge Direction

Handle flags control **drag interaction**, not edge topology:
- `connectable_start` — can drags begin here?
- `connectable_end` — can drags end here?

After normalization, an edge may end at a handle with `connectable_end=false` if the drag started there. The flags gate the drag operation, not the final edge direction.

### Connection Normalization

Connections are normalized based on which handle **type** you started from:

```
Started from source handle: source = drag_start, target = drop_target
Started from target handle: source = drop_target, target = drag_start (swapped)
```

This determines edge direction and marker placement. In Loose mode with source→source or target→target connections, the marker direction depends on which handle initiated the drag.

### Duplicate Detection

`connection_exists()` checks for exact match (source, target, handles). No reverse check — handle types prevent true duplicates (a source handle can't become a target). In Loose mode, A→B and B→A are distinct edges with different semantic direction.

Handle comparison uses xyflow semantics: `None == None` is a match. Handle IDs are optional — `None` means "the only handle of this type on the node".

### Mode-Aware Handle Lookup

`HandleBounds::get(id, handle_type, connection_mode)`:
- **Strict mode**: Only searches handles of the specified type
- **Loose mode**: Searches all handles (source and target)

Edge rendering and hit testing use this for correct handle resolution after normalization.

## Design Decisions

### Why f64 Geometry (not ratatui's u16)

1. **Signed coordinates** — viewport math produces negatives (nodes outside visible area)
2. **Fractional zoom** — `canvas = world * zoom + pan` produces f64
3. **Layout compatibility** — Sugiyama outputs f64
4. **Smooth dragging** — at zoom > 1, moving 1 cell = fractional world units

Conversion to ratatui's u16 types happens at the render boundary in `RenderContext`. The geometry module stays pure — no ratatui types, no rounding decisions. All terminal-space outputs are i32: `world_to_terminal` returns `(i32, i32)` for points, `world_to_terminal_rect` returns `(i32, i32, i32, i32)` for rect edges (left, top, right, bottom). Dimensions are derived from edge differences (`right - left`), so downstream `as u16` casts are pure type narrowing.

#### Coordinate Pipeline

```
f64 (world) → i32 (logical terminal) → u16 (buffer)
```

**Rendering** follows this pipeline for all canvas elements:

| Element | f64 → i32 | i32 stage | i32 → u16 |
|---------|-----------|-----------|-----------|
| **Nodes** | `world_to_terminal_rect` → `(left, top, right, bottom)` | Clip with `max`/`min` against canvas bounds | Visible rect + scratch buffer offset |
| **Edges** | `world_to_terminal` per point → `Vec<(i32, i32)>` | Cohen-Sutherland clipping | Buffer writes at clipped positions |
| **Braille edges** | `world_to_terminal_f64` per point → `Vec<(f64, f64)>` | Per-dot bounds check (cell from `floor`, dot from the fraction) | Buffer writes at in-canvas cells |
| **Handles** | `world_to_terminal` → `(i32, i32)` | Bounds check in `render_handle` | Single cell write |
| **Labels** | `world_to_terminal` → `(i32, i32)` | Arithmetic clipping | `set_string` |

The i32 stage is where all clipping/rejection decisions happen. The `as u16` cast only occurs after confirming the coordinate is within canvas bounds. i32 exists because off-screen elements have negative terminal positions that u16 cannot represent.

**Hit testing** stays entirely in f64 world space — it never enters the i32 stage. Mouse input is converted once via `terminal_to_world` (with +0.5 cell-center compensation), then all comparisons (node bounds, handle distance, edge path proximity) happen in world coordinates. This keeps hit testing zoom-independent.

**Minimap** has its own coordinate transform (world → minimap-local scale/offset) that bypasses `RenderContext` entirely.

### Why floor() for Positions

Positions use `floor()` — picks which cell to start in. Correct for odd-dimension centering:

| Height | Center | round() | floor() |
|--------|--------|---------|---------|
| 3      | y+1.5  | y+2 ✗   | y+1 ✓   |
| 5      | y+2.5  | y+3 ✗   | y+2 ✓   |

Dimensions are derived from floored edge positions (`right - left`), so they're always exact integers — no independent rounding decision needed.

**Edge-derived dimension tradeoff:** at any fractional zoom (high or low), same-width nodes can differ by 1 terminal cell. The difference occurs whenever `w * zoom` is non-integer and the two nodes' `x * zoom + pan` fractional parts straddle a floor boundary differently — position-dependent. For example, with `w=5` and `zoom=1.5`: a node at `x=0` gets `floor(7.5) - floor(0.0) = 7` cells, while a node at `x=1` gets `floor(9.0) - floor(1.5) = 8` cells. At zoom 1.0 with integer dimensions (the common case) there's no difference at all.

A 1-cell difference on typical node widths (10-30+ cells) is usually imperceptible, though it becomes more noticeable at low zoom or with small world dimensions where terminal widths drop below ~5 cells. Either way, the alternative is worse — independent rounding would break handle alignment. Deriving dimensions from floored edges guarantees that handles (rendered via `world_to_terminal` on their world position) always land within their node's terminal rect. With independent rounding, a handle at a node's right edge could end up 1 cell outside the node body.

**Minimap exception:** the minimap scales dimensions independently (not edge-derived), so it uses `round()` to ensure equal-width nodes always get equal minimap width regardless of position. It can afford this because it has no handles or edges to align with, and at minimap scale (2-3 cell node widths) a 1-cell difference would be 33-50% — far too visible.

Known edge case: at high zoom with fractional pan, handles can be 1 cell off. Acceptable because it's rare and handles/edges stay aligned.

### Cell-Center Compensation in terminal_to_canvas

`canvas_to_terminal` uses `floor()` to snap canvas positions to integer terminal cells. The inverse — `terminal_to_canvas` — must add 0.5 to map to the **center** of the cell, not its top-left corner. Without this, hit testing drifts downward/rightward as zoom decreases.

**Why:** `floor()` maps a canvas position to the cell whose top-left is at or below that position. The cell that displays a canvas point `c` spans `[floor(c), floor(c)+1)`. Its center is at `floor(c) + 0.5`. When the inverse path treats the cell as its top-left (`floor(c) + 0.0`), the recovered canvas position is up to 1.0 lower than the original — and in world space that error is `1.0 / zoom`, which grows as you zoom out.

**Impact by zoom level:** each terminal cell covers `1/zoom` world units. At zoom 1.0 the max error is <1 world unit (sub-cell, invisible). At zoom 0.5 it's up to 2 world units. At zoom 0.25 it's up to 4 world units. The direction is always down-right because `floor()` rounds toward negative infinity.

```
Rendering (world → terminal):     floor(world * zoom + pan) + area_offset
Hit testing (terminal → world):   ((terminal - area_offset + 0.5) - pan) / zoom
                                                          ^^^^^ cell-center compensation
```

**What stays pure:** world coordinates are unaffected. The +0.5 lives entirely at the terminal↔canvas boundary — it's a display discretization concern. Node positions, dimensions, bounds, and all world-space geometry remain exact f64 values.

**Effect on other paths:**
- **Panning:** both anchor and current mouse position get +0.5, so the delta cancels out. No change.
- **Zoom-around:** the anchor point shifts to cell center (0.5 canvas units). Semantically more correct; visually imperceptible.

### NodeExtent::Parent (Position-Only Constraint)

Matching xyflow behavior, `NodeExtent::Parent` constrains **position only**, not dimensions. Users handle dimension clamping at app level:

```rust
// App-level dimension clamping (xyflow pattern)
let max_w = (parent.width - child.position.x).max(min_w);
let max_h = (parent.height - child.position.y).max(min_h);
flow.set_node_dimensions("child", new_w.min(max_w), new_h.min(max_h));
```

Why not clamp dimensions in the library:
- More control for users (they choose clamping strategy)
- Library doesn't mutate user-provided dimensions
- Matches xyflow's API expectations

See `examples/overview.rs` for the full pattern with input synchronization.

### Error Handling

Two boundaries with parallel conventions:

- **Developer boundaries** (validated, returns `Result`): `Flow::with_graph(nodes, edges)?`, `add_node()`
- **User operations** (infallible or no-op): `flow.remove_selected_nodes()`, `flow.select_next_node()`

**Graph validation** at developer boundaries checks:
- Duplicate node/edge IDs
- Invalid references (edge endpoints, parent IDs)
- Self-referential edges
- Handle ID constraints (xyflow semantics):
  - Multiple handles of same type require explicit IDs (`AmbiguousHandles`)
  - Handle IDs must be unique within their type (`DuplicateHandleId`)

Orphan edges during render are skipped silently (defensive programming).

### Interaction Lock

The interaction lock lives on `Flow` — `apply()` and `handle_mouse_event()` gate mutations internally, making it self-enforcing.

Following xyflow's model, the lock disables **mutations** (select, drag, connect, delete) but allows **viewport operations** (pan, zoom, fit view, center). When locked:

- `apply()` returns `NotHandled` for mutation actions, passes viewport actions through
- `handle_mouse_event()` skips hit testing on left-click and goes straight to panning (without clearing selection); scroll zoom still works

The Controls widget reads the lock state directly from `&Flow` at render time.

### Directional Navigation

`select_node_in_direction(Direction)` uses weighted nearest-neighbor with directional bias. The algorithm runs in f64 world space using `position_absolute` (node center = top-left + half dimensions).

**Scoring:** `score = distance × (1 + DIRECTION_BIAS × angular_penalty)` where `DIRECTION_BIAS = 2.0` and `angular_penalty = sin(angle)` = `|cross_product| / distance`. Candidates with `dot_product ≤ 0` (behind the current node in the given direction) are excluded.

**Properties:**
- No unreachable nodes — four 180° hemispheres cover all 360°; each direction makes monotonic progress along its primary axis
- No cycles — monotonicity prevents A→B→A
- No configuration — `DIRECTION_BIAS` is a hardcoded perceptual constant, like VS Code's spatial navigation and game UI gamepad navigation
- Hidden nodes are skipped; no-selection fallback selects first node

**Location:** `state/selection.rs` — `Direction` enum, `DIRECTION_BIAS` constant, `select_node_in_direction()`, `node_center()` helper.

### Event Handling

Extends ratatui ecosystem patterns with rich event returns for user extension:

```rust
// Granular operations
flow.select_next_node();
flow.select_node_in_direction(Direction::Right);
flow.pan(dx, dy);

// Event handlers return EventResponse — iterate with into_events()
for event in flow.handle_mouse_event(mouse.into()).into_events() {
    match event {
        FlowEvent::NodeClicked { node_id } => show_details(&node_id),
        FlowEvent::ConnectionCompleted(conn) => {
            flow.add_edge_from_connection(conn, StepEdge::default());
        }
        _ => {}
    }
}

// Keyboard with custom bindings
if let Some(action) = my_bindings(&key.into()) {
    flow.apply(action); // returns EventResponse
}

// Handler chaining via NotHandled fallthrough
let response = flow.handle_controls_key_event(key.into());
if matches!(response, EventResponse::NotHandled) {
    flow.handle_key_event(key.into());
}
```

Unlike tui-textarea (returns `bool`), we return `EventResponse` with three variants (`NotHandled`, `Handled`, `Event(FlowEvent)`) to enable handler chaining and user code to react to specific interactions without manual state tracking. This mirrors xyflow's callback model (onNodeClick, onConnect) adapted for immediate-mode TUI.

## Implementation Notes

### Deferred Fit-View

`request_fit_view()` defers the viewport fit to the next render of `Flow`, after `set_canvas_area()` but before drawing. This means the fit always uses the current frame's canvas size — no need to render first. If the canvas size changes between frames (e.g., terminal resize on startup), the fit is re-applied automatically until the size stabilizes (typically 2 frames).

```rust
// Works — fit deferred to render time
flow.request_fit_view();
loop {
    terminal.draw(|f| { ... })?;
    // ...
}
```

`center_on_selected()` still needs canvas size from a prior render.

### Configuration Defaults

```rust
flow.min_zoom            // 0.5
flow.max_zoom            // 2.0
flow.handle_hit_radius   // 1.5 world units
flow.edge_hit_threshold  // 1.5 world units
flow.connection_radius   // 2.0 world units
```

### RenderContext

Handles canvas offset automatically:

```rust
render_ctx.world_to_terminal(&viewport, world_pos) -> (i32, i32)
render_ctx.world_to_terminal_f64(&viewport, world_pos) -> (f64, f64)
render_ctx.terminal_to_world(&viewport, col, row) -> Position
```

Canvas coordinates only needed for panning anchor and zoom-around-point.

`world_to_terminal_f64` is the same mapping minus the final `floor()`; the two agree by construction, since the canvas offset is an integer. It exists for sub-cell rendering, which needs the fraction `floor()` discards. It stays `pub(crate)` — whole-cell coordinates are the public contract, and everything drawing below that granularity lives in the library.

The world→terminal direction is exposed publicly in two places, both delegating here so rounding stays in one spot: `EdgeRenderContext::world_to_terminal`/`is_in_bounds` (custom edge content, which renders without access to the `Flow`) and `Flow::world_to_terminal`/`node_terminal_rect`/`is_in_bounds` (app-drawn overlays, typically right after rendering the flow in the same draw pass). The two behave identically — the context holds borrows of the same `Flow` fields the `Flow` methods read. `node_terminal_rect` returns the same unclipped `(left, top, right, bottom)` i32 edges the node renderer computes from `InternalNode::bounds()`.

### Braille Strokes

`EdgeStyle::braille()` switches `render_path` from one character per cell to a 2x4 dot grid per cell. Box-drawing can only approximate a slope — a diagonal becomes a staircase of `│`, one per row — so free-form (straight) edges gain the most. Stepped edges are already axis-aligned and gain nothing.

Characters live on [`EdgeStroke::Chars`] rather than on `EdgeStyle`, so a braille style has none to read — the mode that cannot use them cannot carry them. `render_braille_path` consults only `stroke_style`; markers and labels still apply, via the shared `render_markers_and_label`. The char builders (`with_line_chars` and friends) promote a braille stroke back to `Chars`, since setting a character is a request for character rendering.

Three things differ from the character renderer:

**Dots accumulate before any write.** The whole polyline collects into a `HashMap<cell, u8>` mask, then each cell is written once. On write the mask is OR-ed into whatever braille the cell already holds, which is what merges two braille edges where they cross; the same OR would absorb an edge's own strokes revisiting a cell, so the accumulator is a cost choice rather than a correctness one. Oversampling puts up to eight dots in a single cell, and batching pays the symbol parse and `set_char` once instead of once per dot.

**No corner pass.** Corners exist because the character renderer draws segments independently and needs a glyph at the joint. A braille stroke through a turn is continuous already.

**Clipping is per dot, not per segment.** The Cohen-Sutherland polyline clipper is bypassed; `plot_braille_dot` bounds-checks each dot against the canvas. Cheaper than clipping a polyline that is about to be walked point by point anyway.

Braille does not merge with box-drawing — ratatui's `MergeStrategy` tables have no braille composites, so a braille edge crossing a stepped one replaces that cell rather than combining. Markers and labels are shared with the character renderer via `render_markers_and_label`, so an endpoint arrow overwrites the braille cell under it.

Animation counts distance in whole cells, not sub-cells, so braille marching ants keep the same 2-on-1-off rhythm as characters.

### MiniMap Quadrant Rasterization

The minimap rasterizes nodes on a 2x2 sub-cell grid and emits one quadrant glyph per cell, giving four times the area resolution of whole `█` blocks. All sixteen 2x2 masks exist in Unicode (U+2596-259F plus the half blocks, full block and space), so there is no fallback case — the reason quadrants and not sextants (2x3, Unicode 13, patchy font coverage). There is no whole-cell mode to fall back to either: quadrant glyphs have wider font coverage than the braille the edge renderer already relies on, so the option would guard a case the library does not guard anywhere else.

Position/dimension rounding is unchanged from whole-cell mode — floor the position, round the dimension, `max(1)` — just applied in sub-cell units. Snapping to a grid twice as fine halves the position and size error, and the `max(1)` floor becomes one quadrant instead of one full cell, which is what stops sub-cell nodes from inflating into blocks that merge with their neighbours.

Sub-cells accumulate into a `HashMap<cell, (mask, any_selected)>` rather than overwriting, so a cell shared by two nodes shows both parts. A cell carries one foreground color, so selection wins on collision. Whole-cell mode has the same collision but hides it — one solid block simply covers the other.

Note this does not change the aspect ratio of the marks. Splitting a cell 2x2 leaves each sub-cell with the cell's own ~1:2 width-to-height ratio; only a 2x4 (braille) or 2x3 (sextant) split moves sub-cells toward square. Quadrants buy resolution, not proportion.

The viewport indicator is a background fill drawn before nodes, and quadrant glyphs set only the foreground, so it now shows through the transparent parts of a node mark instead of being covered by a solid block.

### Coding Idioms

**Move over clone.** `Widget::render(self, ...)` consumes `self` — move owned fields (e.g., `self.block`) instead of cloning. Only clone when `self` is needed afterward for method calls (Controls, MiniMap).

**`mem::take` over clone for state transitions.** When processing `drag_state` while calling `&mut self` methods, `mem::take` moves the state out (replacing with `None`) and frees `self` for borrowing — no clone needed.

**`is_some_and` to release borrows before mutation.** When a read borrow (e.g., `internal_node()`) gates a subsequent mutation (e.g., setting `drag_state`), `is_some_and` evaluates and drops the borrow in one expression.

**O(1) lookup over linear scan.** Use `internal_node(id)` (HashMap) instead of `.iter().find(|n| n.id() == id)`.

**Reuse existing methods.** Prefer `HandlePosition::opposite()` over duplicating direction-inversion matches. Prefer `NodeOrigin::offset()` over reimplementing the same arithmetic. When parallel structure exists (start/end, source/target), extract a helper parameterized by the varying part.

**Owned keys in lookup maps.** When iterating and mutating (e.g., `resolve_hierarchy`), use `HashMap<String, ...>` not `HashMap<&str, ...>` to avoid borrow conflicts.

### expand_parent: 3-Phase Hierarchy Resolution

When a child has `expand_parent = true`, the parent auto-grows to contain it. This is processed in `resolve_hierarchy()` across three phases:

1. **Top-down BFS** — computes absolute positions and applies extent constraints (existing logic). Also tracks whether any node uses `expand_parent` and collects BFS levels as `Vec<Vec<usize>>` for the bottom-up pass. `NodeExtent::Parent` clamping is skipped for `expand_parent` children (they push the parent instead of being clamped).

2. **Bottom-up expansion** — iterates BFS levels from deepest to shallowest. For each parent, takes the union of its rect with all `expand_parent` children's rects. If the union is larger, the parent's dimensions grow and its position shifts (origin-aware). All children (not just `expand_parent` ones) get counter-adjusted to preserve their absolute positions. Only runs if phase 1 detected any `expand_parent` nodes.

3. **Re-resolve positions** — re-runs phase 1 to propagate updated positions and recompute handle bounds after expansion. Only runs if phase 2 actually changed something.

Cascading (grandchild→parent→grandparent) settles in one call because phase 2 processes bottom-up — inner expansions happen before outer ones.

### Re-Parenting

`Node::position` is stored relative to the parent, so moving a node between parents is two changes, not one: the `parent_id`, and a rebase of the coordinates. `set_node_parent` does both, and holds the node still on screen while doing it.

The rebase falls out of `absolute = parent_absolute + position + origin_offset`. The origin offset does not change, so shifting `position` by the difference between the old and new parent absolutes leaves the absolute untouched — no call site ever converts between the two frames.

That last point is the reason the method exists rather than leaving callers to edit `parent_id` through `set_nodes`. An app doing it by hand has to reach for absolute positions, unwind them, and re-derive the relative ones; the pieces it needs (`InternalNode`, `position_absolute`, `NodeOrigin::offset`) are all `pub(crate)`, so what it actually writes is an approximation that drifts on origins, extents and `expand_parent` growth.

Cycles are rejected with `Error::CyclicParent` rather than tolerated. `resolve_hierarchy` walks down from roots, so a cycle is not a loop that spins — it is a set of nodes that no longer appear anywhere, silently keeping their last positions. Detection walks up from the proposed parent looking for the node itself.

### Auto-Pan

Implements xyflow-compatible auto-panning during drag operations. When the cursor enters the 5-cell edge zone (`EDGE_DISTANCE = 5.0`) during a node drag or connection drag, the viewport pans continuously in that direction.

**Velocity model:** `auto_pan_velocity()` mirrors xyflow's `calcAutoPanVelocity` — linear ramp from ~0 at the threshold to ±1.0 at the canvas edge, per axis. Total pan per tick: `velocity × speed × dt`, where `speed` defaults to 110 cells/s (≈xyflow's 15px/frame at 60fps = 900px/s ≈ 112 cells/s).

**Frame-rate independence:** xyflow is frame-rate dependent (`panBy(velocity * speed)` per `requestAnimationFrame`). We multiply by elapsed seconds — intentional divergence for consistent behavior across terminal refresh rates.

**Node drag compensation:** when auto-pan shifts the viewport by `(dx, dy)` in canvas space, the world position under the cursor shifts by `(-dx/zoom, -dy/zoom)`. To keep the node under the cursor, `compensate_node_drag` subtracts the world delta from the node's position. The drag offset is NOT adjusted — it's the initial grab delta (`node_pos - mouse_world_pos`) which stays correct because auto-pan shifts both node and cursor's world position equally. The next `on_mouse_drag` recomputes position from `mouse_world_pos + offset` and arrives at the right place. This matches xyflow's `XYDrag.ts` which adjusts `lastPos` (virtual mouse) by `-movement/zoom`, never touches `distance`.

**Connection drag: no compensation.** xyflow's `XYHandle.ts` does `panBy()` with no preview adjustment — the preview naturally updates on the next `pointermove`. We match this: `tick_auto_pan` only compensates node drags, not connection/reconnection drags.

**`terminal_to_canvas` i32 fix:** this method originally used `column.saturating_sub(canvas_area.x)` in u16 — clamping to 0 when the cursor was left of/above the canvas origin. This caused asymmetric auto-pan velocity (left/top edges slower than right/bottom) because `auto_pan_velocity` saw the cursor stuck at ~0.5 instead of going negative. Fixed to `(column as i32 - canvas_area.x as i32) as f64 + 0.5`. This is strictly on the input path (terminal → canvas → world for hit testing/mouse handling) — the rendering path (`canvas_to_terminal`, which already returns i32) is unaffected. The corresponding inverse method `canvas_to_terminal` already returned i32, so the two methods are now symmetric.

Key file: `state/auto_pan.rs`.

### Drag Hierarchy Coalescing

During node dragging, `drag_hierarchy_pending` flag defers `resolve_hierarchy()` until render time or mouse-up. This avoids redundant hierarchy traversals when N drag events arrive between frames — only the last position matters.

Safe because `on_mouse_drag` computes position from `mouse_world_pos + offset` (captured at drag start) and does not read `position_absolute` or handle bounds. Each event independently overwrites `node.position`.

After resolving, `resolve_drag_hierarchy_if_pending` refreshes the cached `parent_absolute` in the drag state. Without this, `expand_parent` leftward/upward shifts would desync the drag offset — the parent's coordinate frame moves but the cached reference stays stale, causing amplified movement.

Key files: `state/mod.rs` (flag + resolve), `state/mouse.rs` (sets flag), `ui/canvas.rs` (resolves before drawing).

### DragState Machine

Mouse interactions are driven by a state machine (`DragState`) that tracks the current drag operation. All transitions happen in three handlers: `on_mouse_down`, `on_mouse_drag`, `on_mouse_up`, plus `FlowAction::CancelConnection` from keyboard.

`CreatingConnection` is a unit variant — all preview data lives in `EdgePreview`. `ReconnectingEdge` only carries `edge_id` (to identify the edge being reconnected). Handle resolution, validation, and rendering state all live in `EdgePreview`.

```
                          ┌─────────────────────────────────────────────────────────────────┐
                          │                            None                                 │
                          └──┬──────────┬───────────┬───────────┬──────────┬────────────────┘
                             │          │           │           │          │
                    hit:     │          │           │           │          │  hit:
                 source/     │   hit:   │  hit:     │  hit:     │          │  nothing
                target       │  source/ │  node     │  node     │  hit:    │
                handle       │  target  │  body     │  body     │  edge    │
              (connectable)  │  handle  │ (drag ok, │ (no drag  │          │
                             │  (not    │ draggable)│  or not   │          │
                             │  conn.)  │           │ draggable)│          │
                             ▼          ▼           ▼           ▼          │
            ┌──────────────────┐  ┌───────────────────┐  ┌───────────────────┐
            │ Creating         │  │ MovingNode         │  │ AwaitingNodeClick │
            │ Connection       │  │ {drag_started:     │  │ { node_id }       │
            │ (unit variant)   │  │  false}            │  │                   │
            │ + EdgePreview    │  │                    │  │                   │
            └──┬────────┬──────┘  └──┬──────────┬──────┘  └──────────┬────────┘
               │        │           │          │                    │
            drag:    mouse-up:   drag:      mouse-up:            mouse-up:
            update   valid       distance   ┌──────────┐         → None
            preview  target?     > thresh?  │          │         ⇒ NodeClicked
               │     ┌──┴──┐     ┌──┴──┐    │          │
               │    yes    no   yes    no   │          │          ┌──────────┐
               │     │      │    │      │   │          │          │          │
               │     │      │    ▼      │   │          │          │          ▼
               │     │      │  set      │   │  drag_   │          │   ┌─────────────┐
               │     │      │  drag_    │   │  started  │          │   │  Panning     │
               │     │      │  started  │   │  =false?  │          │   │  { anchor,   │
               ▼     │      │  =true    │   │  ┌─┴──┐  │          │   │  initial_vp }│
            (self)   │      │  ⇒ Node   │   │ yes  no  │          │   └──┬──────┬────┘
               │     │      │  Drag     │   │  │    │   │          │      │      │
               │     │      │  Started  │   │  │    ▼   │          │   drag:  mouse-up:
               │     │      │    │      │   │  │ ⇒ Node │          │  update   → None
               │     │      │    ▼      │   │  │  Drag  │          │  viewport
               │     │      │  (Moving  │   │  │  Ended │          │     │
               │     │      │   Node,   │   │  ▼       │          │     ▼
               │     │      │  ongoing  │   │⇒ Node    │          │  (self)
               │     │      │  drag)    │   │ Clicked  │          │
               │     │      │  ⇒ Node   │   │          │          │
               │     │      │  Dragged  │   └──────────┘          │
               │     ▼      ▼    │      │                         │
               │   → None  → None│      │                   hit:  │
               │  ⇒ Conn. ⇒ Conn.│      │                nothing  │
               │  Compl.  Cancel. │      │                   │     │
               │  clear   clear   │      │                   ▼     │
               │  preview preview │      │            ┌─────────────┐
               │                  ▼      ▼            │  Panning     │──── drag: update viewport
               │               → None  → None         │              │     (self-loop)
               └── keyboard: CancelConnection         └──────────────┘
                   → None, clear preview
                   ⇒ ConnectionCancelled

ReconnectingEdge (branches from handle hits when a selected reconnectable edge exists):

    hit: source/target handle
    with exactly 1 selected
    reconnectable edge at endpoint
              │
              ▼
    ┌───────────────────┐
    │ ReconnectingEdge   │
    │ { edge_id }        │
    │ + EdgePreview      │   ⇐ from = fixed end (opposite of dragged)
    └──┬────────┬────────┘
       │        │
    drag:    mouse-up:
    update   valid
    preview  target?
       │     ┌──┴──┐
       │    yes    no
       │     │      │
       ▼     ▼      ▼
    (self) → None  → None
       │  ⇒ Recon. ⇒ Recon.
       │  Compl.   Cancel.
       │  clear    clear
       │  preview  preview
       │
       └── keyboard: CancelConnection
           → None, clear preview
           ⇒ ReconnectionCancelled
```

**Legend:** `→` = state transition, `⇒` = emitted FlowEvent, `(self)` = stays in same state, `+ EdgePreview` = preview state set alongside DragState.

**Key behaviors:**

| Behavior | Mechanism |
|----------|-----------|
| Click vs drag distinction | `MovingNode.drag_started` + distance threshold (default 2.0) |
| Consistent NodeClicked timing | Always fires on mouse-up, whether via `MovingNode` (threshold not exceeded) or `AwaitingNodeClick` |
| Connection validation feedback | `EdgePreview.is_valid` updated each drag event, drives preview color |
| Connection normalization | Starting from target handle swaps source/target on completion |
| Hierarchy coalescing | `MovingNode` drag sets `drag_hierarchy_pending`, resolved at render or mouse-up |
| Locked mode bypass | Left-click goes directly to `Panning` (skips hit testing), handled in `handle_mouse_event` |

Key files: `state/mouse.rs` (DragState, on_mouse_down/drag/up, hit_test), `state/edge_preview.rs` (EdgePreview), `state/event_handlers.rs` (handle_mouse_event, CancelConnection).

### Reconnection

Edge reconnection lets users drag an existing edge's source or target endpoint to a different handle. In xyflow, reconnection is triggered by hovering an edge to reveal invisible SVG anchor circles at its endpoints, then dragging them. In terminal there's no hover state, so we use **selected edge as the mode switch**: select an edge, then drag from one of its endpoint handles to reconnect. No edge selected → normal connection creation.

**Trigger mechanism:** when a handle is clicked, `on_mouse_down` calls `find_reconnectable_edge_at()` before the normal connectability check. If exactly one selected edge connects at that endpoint and the edge's `reconnectable` setting allows it (resolved via `Reconnectable::allows()` against the global `edges_reconnectable` default), the drag enters `ReconnectingEdge` instead of `CreatingConnection`. The `EdgePreview` is set with the **fixed end** (the opposite of what's being dragged) as the source — this is the anchor point for the preview line.

**Ambiguity rule:** when multiple selected edges share the same endpoint handle, reconnection is skipped (ambiguous which edge to reconnect) and falls through to normal connection creation.

**Shared drag logic:** `ReconnectingEdge` and `CreatingConnection` use the same `update_edge_preview_to_position()` method during `on_mouse_drag` — handle lookup, duplicate detection, and validator callback are identical for both paths.

**Completion:** on mouse-up with a valid target, `ReconnectionCompleted` is emitted with both old and new connections. Like `ConnectionCompleted`, the event is informational — the app calls `reconnect_edge()` to apply the change. On mouse-up without a valid target, `ReconnectionCancelled` is emitted and the original edge is unchanged.

**Canvas rendering:** during reconnection, the original edge is hidden (skipped in the edge render loop) and replaced by the edge preview from the fixed end to the target position, using the same `render_drag_edge_preview` as new connections.

### SelectionChanged Diff Check

`SelectionChanged` is diff-checked — only emitted when selection actually changed during a handler call. The mechanism:

1. **Snapshot at handler entry** — `apply()` and `handle_mouse_event()` call `snapshot_selection()`, which scans nodes/edges and copies selected IDs into `prev_selection_node_ids` / `prev_selection_edge_ids` on `Flow`. This captures the current reality, including any programmatic mutations the user made between handler calls.

2. **Zero-alloc comparison** — `selection_matches_snapshot()` walks nodes/edges comparing selected IDs against the snapshot inline, without allocating. Returns early on first mismatch.

3. **Conditional emission** — `maybe_selection_changed_event()` uses the comparison to return `None` (suppressed) or `Some(SelectionChanged { ... })`.

Suppressed scenarios: clicking an already-selected node, clearing an empty selection, `SelectNext` wrapping to the same node in a single-node graph. The gesture event (e.g., `NodeClicked`) still fires — only the redundant `SelectionChanged` is suppressed.

The snapshot cost is O(n) scan + O(k) string clones per handler entry (k = selected count, typically 0-5). The savings come from eliminating the event allocation and all downstream app-level work when selection didn't change.

Key files: `state/event_handlers.rs` (`snapshot_selection`, `maybe_selection_changed_event`, `selection_matches_snapshot`), `state/mod.rs` (`prev_selection_*` fields).

### Selection Box and Resize

The right button carries both: a click opens a context menu, a drag draws a selection box. `on_right_down` computes the menu event but holds it in `DragState::AwaitingContextMenu` rather than emitting, because the same press may still become a drag; `on_right_up` emits it only if `node_drag_threshold` was never exceeded. This also puts right-click on the same footing as left, where `NodeClicked` has always fired on release.

The right button rather than a modifier: xterm and its derivatives conventionally use Shift to *bypass* mouse reporting so users can select terminal text, so a shift-drag often never reaches the application at all. The right button has no such convention attached.

`selection_on_drag` (default off) redirects the *left* pane gesture to the box instead, for terminals that never deliver the right button — Warp reserves right-click for its own menu, the same way xterm-family terminals reserve Shift. Pressing a node still drags it; only the pane gesture changes. Because the flag is ordinary config an app can flip it per-frame, it doubles as the binding mechanism: hold a modifier, enter a mode, and the trigger becomes whatever the app decides.

Selection is by intersection via `nodes_in`, and replaces the previous selection rather than adding to it. The marquee is drawn in the canvas pass from the theme accent with no style struct — transient feedback, not part of the graph's appearance — using dashed box-drawing rather than the middle dot, which `BackgroundVariant::Dots` already occupies.

Resizing grabs within `resize_handle_radius` of the **bottom-right** corner, on any node that is `resizable` and visible. One grip rather than four, and no selection prerequisite: the grip is drawn (`◢`), so the affordance sits where the gesture is and a press either lands on it or does not. Four invisible corners would make every press near a node's edge guess between move and resize.

Anchoring at the bottom-right is also what keeps the drag stateless. The top-left never moves, so `apply_resize` derives width and height afresh from the initial bounds on every event and nothing accumulates. A grip that moved the position would have to fold each delta into a running total, and repeated events at one cursor position would compound. Pulling past the anchor clamps to `min_node_size` rather than inverting the node.

### Node Drag Threshold

Like xyflow's `nodeDragThreshold`, we distinguish clicks from drags using a distance threshold (default 2.0 world units). On mouse down:

1. Enter `DragState::MovingNode` with `drag_started = false` and `start_pos`
2. On drag, check if `distance(current_pos, start_pos) > threshold`
3. If threshold not exceeded: don't move node, return `Handled`
4. If threshold exceeded: set `drag_started = true`, emit `NodeDragStarted`
5. On mouse up: emit `NodeClicked` if `!drag_started`, else `NodeDragEnded`

This prevents accidental drags from triggering position changes and makes click detection reliable. For non-draggable nodes, we use `DragState::AwaitingNodeClick` to defer `NodeClicked` to mouse up for consistency.

## Clipping

- **Early exit**: skip edges whose bounding box doesn't intersect canvas
- **Segment clipping**: `render_path()` clips each segment using Cohen-Sutherland
- **Braille strokes**: clipped per dot instead, bypassing the polyline clipper

## Terminal vs React Flow

| React Flow | rataflow |
|-------------------|------------------------|
| DOM measures dimensions | Dimensions declared upfront |
| Handles discovered via DOM | Handles declared on Node |
| Content determines size | Size explicit before render |

Terminal has no layout engine — everything must be explicit.

### Per-Element Interaction Flags

| Flag | xyflow Node | xyflow Edge | rataflow Node | rataflow Edge |
|------|:-----------:|:-----------:|:-----------------:|:-----------------:|
| `hidden` | ✓ | ✓ | ✓ | ✓ |
| `selected` | ✓ | ✓ | ✓ | ✓ |
| `selectable` | ✓ | ✓ | ✓ | ✓ |
| `deletable` | ✓ | ✓ | ✓ | ✓ |
| `expand_parent` | ✓ | — | ✓ ¹ | — |
| `draggable` | ✓ | — | ✓ | — |
| `dragging` | ✓ | — | — ³ | — |
| `connectable` | ✓ | — | ✓ ⁴ | — |
| `focusable` | ✓ | ✓ | — ² | — ² |
| `animated` | — | ✓ | — | ✓ |
| `reconnectable` | — | ✓ | — | ✓ |
| `resizing` | ✓ | — | — ³ | — |

¹ When `expand_parent = true`, the parent automatically grows to contain the child. Processed bottom-up in `resolve_hierarchy()` so cascading (child→parent→grandparent) settles in one call.
² DOM/accessibility concept — not applicable to terminal.
³ `dragging` and `resizing` are not flags on `Node` — xyflow needs them there to trigger React re-renders. rataflow computes these from state at render time (e.g., `NodeRenderContext.dragging` from `DragState`).
⁴ `Node.connectable` is a master switch for all handles. Handle-level flags (`connectable`, `connectable_start`, `connectable_end`) provide fine-grained control.
