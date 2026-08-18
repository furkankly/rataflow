# rataflow

Library for building node-based UIs in the terminal — from a static diagram to a fully interactive editor. Built on [ratatui](https://github.com/ratatui/ratatui), inspired by [xyflow](https://github.com/xyflow/xyflow) (React Flow).

See `docs/ARCHITECTURE.md` and `docs/INTERNALS.md` for design rationale and implementation details. See `README.md` for performance characteristics and feature flags.

## Documentation Principles

**Parallel abstractions get parallel wording.** When two types share the same design philosophy (e.g., `EdgeStyle`/`HandleStyle`, `StepEdge`/`StraightEdge`, `FlowAction`/`ControlsAction`), their doc comments must use the same sentence structure, same level of specificity, and same categories of description.

Examples of parallel wording:
- `EdgeStyle`: "Visual configuration for edge rendering." / `EdgePreviewStyle`: "Visual configuration for edge preview rendering." / `HandleStyle`: "Visual configuration for handle rendering."
- `StepEdge`: "Built-in edge type that routes with..." / `StraightEdge`: "Built-in edge type that draws..."
- `FlowAction`: "Semantic actions for flow graph interaction." / `ControlsAction`: "Semantic actions for Controls widget interaction."
- Companion widgets: all open with "Companion widget for [purpose]." then describe what they render.

When adding new abstractions, find the existing parallel and match its structure.

**This governs naming as well as wording — but only where the things named actually correspond.** `EdgeStyle::stroke_style` and `HandleStyle::char_style` look like they should match and deliberately don't, because a stroke is not the parallel of a char: an edge has rasterization modes and `EdgeStroke::Braille` has no character to name, while a handle is a single character. Matching the names would assert a parallel that isn't there. See `docs/INTERNALS.md` § Braille Strokes.

## Rendering Model

Three distinct rendering approaches — this asymmetry is intentional, do not "fix" it:

| Element | Custom rendering? | Library primitive |
|---------|------------------|------------------|
| **Nodes** | Yes (`NodeContent`) | None — use ratatui directly |
| **Edges** | Yes (`EdgeContent`) | `EdgeStyle` + `EdgeRenderContext::render_path()` |
| **Handles** | No — library-rendered | `HandleStyle` on `Handle` instances |

All style structs (`EdgeStyle`, `HandleStyle`, `EdgePreviewStyle`, `ControlsStyle`, `MiniMapStyle`, `BackgroundStyle`) share the same design: private fields, `Default` + builders, `Option<Style>`/`Option<Color>` for theme-derived values. `None` means "use theme" — resolved at render time via `resolved_style(palette)`. Structural fields (characters, markers, booleans) are concrete with sensible defaults since they don't vary by theme.

### Canvas Rendering Pipeline

1. Render edges + edge preview to separate buffer (symbol merging at intersections)
2. Composite edge buffer onto main buffer (non-space cells only)
3. Render nodes + handles in z-order (body then handles per node, into per-node scratch buffers)

**Node scratch buffers:** Each visible node is rendered into a per-node buffer at local `(0, 0)` coordinates with full dimensions, then only the visible portion is composited onto the main buffer. This ensures `NodeContent::render()` always receives the complete area (correct partial rendering) and sidesteps ratatui's u16 coordinate space (nodes off the left/top edge have negative terminal positions). The overhead is negligible even at scale: most nodes are culled by the visibility check before any buffer is allocated, each buffer is node-sized (typically ~10x3 cells), and frame time is dominated by the canvas-sized edge buffer and edge path computation — not node rendering. Benchmarked at 28k+ nodes (WASM) and 37k+ nodes (native) with no measurable impact on frame time or memory.

**Node opacity:** Nodes are opaque by default (`node.opaque = true`) — the entire node area blocks content behind it, even cells not written by `NodeContent::render()`. Set `opaque: false` on parent nodes in hierarchical graphs so edges and children remain visible inside the parent.

## Module Structure

```
src/
├── types/      # Primitives: Position, Dimensions, Node, Edge, Handle, Viewport
├── state/      # Flow, split semantically: graph ops, selection, viewport, mouse, edge preview, auto-pan, event handling, FlowOps trait
├── ui/         # Widgets and rendering, split semantically: edges, handles, controls, minimap, background
├── actions.rs  # Semantic actions (FlowAction) and events (FlowEvent)
├── content.rs  # Extension point: NodeContent, EdgeContent traits + render contexts
├── input.rs    # Backend-agnostic input types (KeyEvent, MouseEvent)
├── error.rs    # Structured error types with thiserror
└── layout.rs   # Layout algorithms (feature-gated: `sugiyama`)
```

`types`, `state`, `ui` are the three layers. The standalone modules (`actions`, `content`, `input`, `error`, `layout`) are library-wide concerns — their names speak for themselves.

## Conventions

1. **Use `content` not `data`** — our generic combines type + data (unlike React Flow's `type` + `data`).
2. **Use Flow methods** for mutations (`set_node_position`, `set_node_dimensions`, `select_node`, etc.) — they trigger necessary recomputations. Don't mutate fields directly.
3. **Selection is per-entity** — `node.selected` / `edge.selected` are the sole source of truth, not stored centrally. Mouse selection timing for draggable nodes depends on `select_nodes_on_drag` and `node_drag_threshold`: when both are `true`/`0.0` (defaults), selection is immediate on mouse-down; when threshold > 0, selection is deferred to when the threshold is exceeded (drag) or mouse-up (click); when `select_nodes_on_drag` is false, selection only happens on click (mouse-up), never on drag. `DragState::MovingNode::selected` tracks whether selection already occurred — checked on mouse-up to avoid double-selection. Non-selectable but draggable nodes can be dragged without being selected (hit test uses `selectable || draggable`). `deselect_on_drag` (default true) controls whether dragging an unselected node clears existing selection — set to false for apps with detail panels that should persist across drag interactions.
4. **Dimensions are explicit** — no DOM measurement. Use `Node::from_text("id", pos, "text")` for auto-sizing.
5. **`NodeExtent::Parent` constrains position only** — dimension clamping is app-level.
6. **Events are meaningful interactions requiring app-level response** — `FlowEvent` variants describe interactions an app would commonly run code in response to. Gesture events use the interaction name (`NodeClicked`, `NodeDragStarted`). State-change events use the outcome name (`ViewportChanged`, `SelectionChanged`, `Deleted`). No implicit graph mutations; `ConnectionCompleted` is informational, the user calls `add_edge_from_connection`. Selection is the one implicit side effect. If an app would only "render differently" in response, read state during render — no event needed. See `docs/ARCHITECTURE.md` § Actions and Events.
7. **Two navigation modes** — directional (spatial: arrow keys → `SelectUp`/`Down`/`Left`/`Right`) and sequential (insertion order: Tab/Shift+Tab → `SelectNext`/`SelectPrev`). Directional uses weighted nearest-neighbor with a fixed directional bias constant (`DIRECTION_BIAS = 2.0`) — not configurable by users.

## API Surface

- **`pub`** — stable user-facing API (types, Flow methods, widget structs).
- **`pub(crate)`** — shared across modules but hidden from users (e.g., Flow internal fields, InternalNode, RenderContext). Prefer this over `pub` for anything not part of the public contract.
- **Private** — everything else. Default to private, widen only when needed.

### API Design Principle

Two rules govern field visibility:

1. **`pub` when users read, private + builders when users only configure.** Types whose fields users query (`node.selected`, `state.min_zoom`) use pub fields — making them private would just mean a getter for every field. Types users only write during construction and never inspect (style structs, companion widgets) use private fields + builders, which buys non-exhaustive safety for free.
2. **Side effects require operations.** Flow graph state (`pub(crate)`) has side effects on write (hierarchy resolution, index rebuilding), so mutations go through setters and operations. Flow config (`pub`) has no side effects, so users assign directly.

| Category | Fields | Builders | Why |
|----------|--------|----------|-----|
| **DTOs** (Node, Edge, Handle, etc.) | `pub` | construction sugar | Users read fields back; Flow exposes `&T` only, setters for runtime mutation |
| **Flow config** (min_zoom, locked, etc.) | `pub` | `with_*` (construction sugar) | Users read and write; no side effects |
| **Flow graph** (nodes, edges, lookups, edge preview) | `pub(crate)` | `with_*` triggers computation | Side effects on write |
| **Content widgets** (TextContent, StepEdge) | `pub` | construction sugar | Users read and write via `content_mut()` |
| **Style structs** (EdgeStyle, HandleStyle, etc.) | private | required | Users only configure, never read back |
| **Companion widgets** (Controls, MiniMap, etc.) | private | ratatui convention | Users only configure; take `&Flow` reference |

## Error Handling

- **Developer boundaries** return `Result<_, FlowError>` — `Flow::with_graph()`, `add_node()`, `add_edge()` validate graph integrity (duplicate IDs, invalid refs, self-loops).
- **User operations** are infallible or no-op — `select_next_node()`, `pan()`, `remove_selected_nodes()` silently do nothing if there's nothing to act on.
- **Render-time** is defensive — orphan edges (referencing removed nodes) are skipped silently, never panic.

## Serialization

Feature-gated behind `serde`. The boundary is graph data vs presentation — following xyflow's model. `FlowSnapshot` serializes nodes, edges, viewport. Style fields (`HandleStyle`, `EdgeStyle`) are `#[serde(skip)]` — presentation is app-owned, not graph data. New DTO fields: `#[serde(default)]` for behavioral, unannotated for required identity/geometry, `#[serde(skip)]` for style/presentation. See `docs/ARCHITECTURE.md` § Serialization for the full policy.

## Testing

Test critical logic, not language features. Good test targets: coordinate transforms, hit detection, path computation, graph validation, selection state machines. Bad test targets: getter/setter round-trips, struct construction, trivial delegation.

All tests are inline `#[cfg(test)]` modules — no separate `tests/` directory.

## Dependencies

Minimal. Three core deps (`ratatui`, `thiserror`, and optionally `rust-sugiyama`). Do not add new dependencies without strong justification — especially for something achievable with std.

## Running

```bash
cargo fmt                       # Format code
cargo clippy                    # Lint — must pass with no warnings
cargo test                      # Run tests
```

## Commits

[Conventional Commits](https://www.conventionalcommits.org/), lowercase imperative subject. Scope is a module, not a file: `types`, `state`, `ui`, `layout`, `content`, `actions`, `input`, `error`, `examples`, `web`, `docs`.

```
feat(state): add box selection on right-drag
fix(ui): skip orphan edges referencing removed nodes
```

`cliff.toml` sets `filter_unconventional = true` — a non-conforming commit is dropped from the changelog. Lint with `committed HEAD~1..HEAD`, generate with `git cliff -o CHANGELOG.md`.