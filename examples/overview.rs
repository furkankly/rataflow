//! Comprehensive example combining custom nodes, edges, hierarchy,
//! companion widgets, and third-party integration (ratatui-image, tachyonfx).
//!
//! Node types: Text (Source, Target), Group with child (Group, Child), Raw Buffer,
//! Input with text fields, ratatui-image, tachyonfx shader effects.
//! Edge types: Step, Straight, Animated.

use std::{
    cell::{Cell, RefCell},
    io::stdout,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode},
    execute,
};
use rataflow::{
    Background, BackgroundStyle, BackgroundVariant, Controls, Edge, EdgeContent, EdgeMarker,
    EdgePathContext, EdgeRenderContext, EdgeStyle, EventResponse, Flow, FlowEvent, Handle,
    HandlePosition, HandleStyle, MiniMap, Node, NodeContent, NodeExtent, NodeRenderContext, Path,
    TextContent, compute_step_path, compute_straight_path,
};
use rataflow_examples::autopilot::{Autopilot, Key, Step, typed};
use rataflow_examples::{ExampleMeta, render_shell};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, BorderType, Paragraph, Sparkline, StatefulWidget, Widget},
};

fn meta() -> ExampleMeta<'static> {
    // Title, description and keys come from the shared registry so the
    // native example and the wasm demo cannot describe themselves
    // differently. `q` is added here because only this build can quit.
    rataflow_examples::meta::overview().with_quit()
}
use ratatui_image::{
    StatefulImage,
    picker::{Picker, ProtocolType},
    protocol::StatefulProtocol,
};
use tachyonfx::{Effect, EffectRenderer, Interpolation, Motion, fx};

/// All node types in this example.
enum OverviewNode {
    Text(TextContent),
    Sparkline {
        label: String,
        data: Vec<u64>,
    },
    RawBuffer {
        label: String,
    },
    Input {
        label: String,
        width_value: String,
        height_value: String,
        /// 0 = none, 1 = width, 2 = height
        focus: u8,
    },
    Image {
        label: String,
        // Boxed: `StatefulProtocol` is large enough to dominate the enum's size.
        native_protocol: Box<RefCell<StatefulProtocol>>,
        halfblocks_protocol: Box<RefCell<StatefulProtocol>>,
        /// What `Picker::from_query_stdio` detected — shown in the title so it's
        /// visible which protocol the terminal actually negotiated.
        native_protocol_type: ProtocolType,
        use_halfblocks: bool,
    },
    Fx {
        label: String,
        effect: RefCell<Effect>,
        last_tick: Cell<Instant>,
    },
}

impl std::fmt::Debug for OverviewNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OverviewNode::Text(t) => f.debug_tuple("Text").field(t).finish(),
            OverviewNode::Sparkline { label, data } => f
                .debug_struct("Sparkline")
                .field("label", label)
                .field("data", data)
                .finish(),
            OverviewNode::RawBuffer { label } => {
                f.debug_struct("RawBuffer").field("label", label).finish()
            }
            OverviewNode::Input {
                label,
                width_value,
                height_value,
                focus,
            } => f
                .debug_struct("Input")
                .field("label", label)
                .field("width_value", width_value)
                .field("height_value", height_value)
                .field("focus", focus)
                .finish(),
            OverviewNode::Image {
                label,
                native_protocol_type,
                use_halfblocks,
                ..
            } => f
                .debug_struct("Image")
                .field("label", label)
                .field("native_protocol_type", native_protocol_type)
                .field("use_halfblocks", use_halfblocks)
                .finish_non_exhaustive(),
            OverviewNode::Fx { label, .. } => f
                .debug_struct("Fx")
                .field("label", label)
                .finish_non_exhaustive(),
        }
    }
}

impl Default for OverviewNode {
    fn default() -> Self {
        OverviewNode::Text(TextContent::default())
    }
}

impl NodeContent for OverviewNode {
    fn render(&self, ctx: &NodeRenderContext, buf: &mut Buffer) {
        match self {
            OverviewNode::Text(text_content) => {
                render_text_node(ctx, text_content, buf);
            }
            OverviewNode::Sparkline { label, data } => {
                render_sparkline_node(ctx, label, data, buf);
            }
            OverviewNode::RawBuffer { label } => {
                render_raw_buffer_node(ctx, label, buf);
            }
            OverviewNode::Input {
                label,
                width_value,
                height_value,
                focus,
            } => {
                render_input_node(ctx, label, width_value, height_value, *focus, buf);
            }
            OverviewNode::Image {
                label,
                native_protocol,
                halfblocks_protocol,
                native_protocol_type,
                use_halfblocks,
            } => {
                let (protocol, protocol_type) = if *use_halfblocks {
                    (halfblocks_protocol, ProtocolType::Halfblocks)
                } else {
                    (native_protocol, *native_protocol_type)
                };
                render_image_node(ctx, label, protocol_type, protocol, buf);
            }
            OverviewNode::Fx {
                label,
                effect,
                last_tick,
            } => {
                render_fx_node(ctx, label, effect, last_tick, buf);
            }
        }
    }
}

fn render_sparkline_node(ctx: &NodeRenderContext, label: &str, data: &[u64], buf: &mut Buffer) {
    let border_style = if ctx.selected {
        Style::default()
            .fg(Color::Indexed(80))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(71))
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(label)
        .style(Style::default().bg(Color::Indexed(234)));
    let inner = block.inner(ctx.area);
    block.render(ctx.area, buf);

    Sparkline::default()
        .data(data)
        .style(
            Style::default()
                .fg(Color::Indexed(179))
                .bg(Color::Indexed(234)),
        )
        .render(inner, buf);
}

fn render_text_node(ctx: &NodeRenderContext, content: &TextContent, buf: &mut Buffer) {
    let border_style = if ctx.selected {
        Style::default()
            .fg(Color::Indexed(80))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(75))
    };

    let block = Block::bordered().border_style(border_style);

    let paragraph = Paragraph::new(content.text.clone()).block(block);
    paragraph.render(ctx.area, buf);
}

/// Renders a node using raw buffer manipulation (no Block).
fn render_raw_buffer_node(ctx: &NodeRenderContext, label: &str, buf: &mut Buffer) {
    let area = ctx.area;
    if area.width < 3 || area.height < 3 {
        return;
    }

    let border_color = if ctx.selected {
        Color::Indexed(80)
    } else {
        Color::Indexed(75)
    };
    let border_style = Style::default().fg(border_color);
    let fill_style = Style::default().bg(Color::Indexed(234));

    // Terminal cells are ~2:1, so this is an oval approximation.
    let cx = area.x + area.width / 2;
    let cy = area.y + area.height / 2;
    let rx = (area.width / 2).saturating_sub(1) as f64;
    let ry = (area.height / 2).saturating_sub(1) as f64;

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let dx = (x as f64 - cx as f64) / rx.max(1.0);
            let dy = (y as f64 - cy as f64) / ry.max(1.0);
            let dist = dx * dx + dy * dy;

            if dist <= 1.0 {
                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_style(fill_style);
            }
        }
    }

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let dx = (x as f64 - cx as f64) / rx.max(1.0);
            let dy = (y as f64 - cy as f64) / ry.max(1.0);
            let dist = dx * dx + dy * dy;

            if dist > 0.6 && dist <= 1.1 {
                let cell = buf.cell_mut((x, y)).unwrap();
                let ch = if dist > 0.95 {
                    if dy.abs() > dx.abs() * 0.5 {
                        if dy < 0.0 { '~' } else { '_' }
                    } else if dx < 0.0 {
                        '('
                    } else {
                        ')'
                    }
                } else {
                    ' '
                };
                cell.set_char(ch);
                cell.set_style(border_style);
            }
        }
    }

    if !label.is_empty() && area.height >= 1 && area.width > label.len() as u16 {
        let label_x = cx.saturating_sub(label.len() as u16 / 2);
        let label_style = Style::default()
            .fg(Color::Indexed(231))
            .add_modifier(Modifier::BOLD);
        buf.set_string(label_x, cy, label, label_style);
    }
}

fn render_input_node(
    ctx: &NodeRenderContext,
    label: &str,
    width_value: &str,
    height_value: &str,
    focus: u8,
    buf: &mut Buffer,
) {
    let area = ctx.area;
    let border_style = if ctx.selected {
        Style::default()
            .fg(Color::Indexed(80))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(71))
    };

    let block = Block::bordered()
        .border_type(BorderType::Double)
        .border_style(border_style)
        .title(label)
        .style(Style::default().bg(Color::Indexed(234)));
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.height < 4 || inner.width < 10 {
        return;
    }

    let field_width = inner.width.saturating_sub(8);
    let width_label_area = Rect::new(inner.x, inner.y, 7, 1);
    let width_input_area = Rect::new(inner.x + 7, inner.y, field_width.min(6), 1);

    buf.set_string(
        width_label_area.x,
        width_label_area.y,
        "Width:",
        Style::default().fg(Color::Indexed(248)),
    );

    let width_style = if focus == 1 {
        Style::default()
            .bg(Color::Indexed(242))
            .fg(Color::Indexed(231))
    } else {
        Style::default().fg(Color::Indexed(231))
    };
    buf.set_string(
        width_input_area.x,
        width_input_area.y,
        width_value,
        width_style,
    );
    if focus == 1 {
        for x in width_input_area.x + width_value.len() as u16
            ..width_input_area.x + width_input_area.width
        {
            if let Some(cell) = buf.cell_mut((x, width_input_area.y)) {
                cell.set_char(' ');
                cell.set_style(width_style);
            }
        }
    }

    if inner.height >= 2 {
        let height_label_area = Rect::new(inner.x, inner.y + 1, 8, 1);
        let height_input_area = Rect::new(inner.x + 8, inner.y + 1, field_width.min(6), 1);

        buf.set_string(
            height_label_area.x,
            height_label_area.y,
            "Height:",
            Style::default().fg(Color::Indexed(248)),
        );

        let height_style = if focus == 2 {
            Style::default()
                .bg(Color::Indexed(242))
                .fg(Color::Indexed(231))
        } else {
            Style::default().fg(Color::Indexed(231))
        };
        buf.set_string(
            height_input_area.x,
            height_input_area.y,
            height_value,
            height_style,
        );
        if focus == 2 {
            for x in height_input_area.x + height_value.len() as u16
                ..height_input_area.x + height_input_area.width
            {
                if let Some(cell) = buf.cell_mut((x, height_input_area.y)) {
                    cell.set_char(' ');
                    cell.set_style(height_style);
                }
            }
        }
    }

    if inner.height >= 4 {
        buf.set_string(
            inner.x,
            inner.y + 3,
            "Tab: switch, Enter: apply",
            Style::default().fg(Color::Indexed(242)),
        );
    }
}

/// Short protocol names, kept within the node's 22-cell title width.
fn protocol_name(protocol_type: ProtocolType) -> &'static str {
    match protocol_type {
        ProtocolType::Halfblocks => "blocks",
        ProtocolType::Sixel => "sixel",
        ProtocolType::Kitty => "kitty",
        ProtocolType::Iterm2 => "iTerm2",
    }
}

fn render_image_node(
    ctx: &NodeRenderContext,
    label: &str,
    protocol_type: ProtocolType,
    protocol: &RefCell<StatefulProtocol>,
    buf: &mut Buffer,
) {
    let area = ctx.area;

    let border_style = if ctx.selected {
        Style::default()
            .fg(Color::Indexed(80))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(75))
    };

    let block = Block::bordered()
        .border_type(BorderType::Thick)
        .border_style(border_style)
        .title(format!("{label} · {}", protocol_name(protocol_type)))
        .style(Style::default().bg(Color::Indexed(234)));
    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width > 0 && inner.height > 0 {
        let image_widget = StatefulImage::default();
        image_widget.render(inner, buf, &mut *protocol.borrow_mut());
    }
}

/// Renders a node with live tachyonfx shader effects applied post-render.
fn render_fx_node(
    ctx: &NodeRenderContext,
    label: &str,
    effect: &RefCell<Effect>,
    last_tick: &Cell<Instant>,
    buf: &mut Buffer,
) {
    let area = ctx.area;
    let border_style = if ctx.selected {
        Style::default()
            .fg(Color::Indexed(80))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(133))
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(label)
        .style(Style::default().bg(Color::Indexed(234)));
    let inner = block.inner(area);
    block.render(area, buf);

    let [centered] = ratatui::layout::Layout::vertical([ratatui::layout::Constraint::Length(2)])
        .flex(ratatui::layout::Flex::Center)
        .areas(inner);

    Paragraph::new("NodeContent trait\ntachyonfx")
        .style(Style::default().fg(Color::Indexed(80)))
        .alignment(ratatui::layout::Alignment::Center)
        .render(centered, buf);

    // Apply tachyonfx effect over the already-rendered content.
    let now = Instant::now();
    let elapsed = now.duration_since(last_tick.get());
    last_tick.set(now);

    let mut fx = effect.borrow_mut();
    if fx.running() {
        buf.render_effect(&mut *fx, inner, elapsed);
    }
}

/// Edge type — variants differ by path algorithm.
#[derive(Clone, Debug)]
enum MixedEdge {
    Step {
        style: EdgeStyle,
        selected_style: EdgeStyle,
    },
    Straight {
        style: EdgeStyle,
        selected_style: EdgeStyle,
    },
}

impl Default for MixedEdge {
    fn default() -> Self {
        MixedEdge::Step {
            style: EdgeStyle::default(),
            selected_style: EdgeStyle::default(),
        }
    }
}

impl EdgeContent for MixedEdge {
    fn compute_path(&self, ctx: &EdgePathContext) -> Path {
        match self {
            MixedEdge::Step { .. } => compute_step_path(
                ctx.from,
                ctx.to,
                ctx.source_position,
                ctx.target_position,
                3.0,
            ),
            MixedEdge::Straight { .. } => {
                compute_straight_path(ctx.from, ctx.to, ctx.source_position, ctx.target_position)
            }
        }
    }

    fn render(&self, ctx: &EdgeRenderContext, buf: &mut Buffer) {
        let style = match self {
            MixedEdge::Step {
                style,
                selected_style,
            }
            | MixedEdge::Straight {
                style,
                selected_style,
            } => {
                if ctx.selected {
                    selected_style
                } else {
                    style
                }
            }
        };

        let label = ctx.label.map(Text::raw);
        ctx.render_path(style, label.as_ref(), buf);
    }
}

/// Waypoints the script aims at, read from the live layout by [`demo_pilot`].
///
/// A struct rather than seven positional `u16`s: they are all the same type,
/// so a transposed pair would compile and simply aim the demo somewhere else.
#[derive(Clone, Copy, Default)]
struct Waypoints {
    /// Centre of the nested child D.
    dx: u16,
    dy: u16,
    /// Centre of the Sparkline B, which the orbit drags.
    gx: u16,
    gy: u16,
    /// Just inside parent C's right edge — far enough to pin D against it.
    clamp_x: u16,
    /// Empty canvas, found by search, for the pan beat.
    pane_x: u16,
    pane_y: u16,
}

/// The demo's choreography. THE source for both what it does and how long
/// it takes — [`demo_script_secs`] sums this same list, so the recorder and
/// the tape can no longer disagree with it.
fn demo_steps(w: Waypoints) -> Vec<Step> {
    let Waypoints {
        dx,
        dy,
        gx,
        gy,
        clamp_x,
        pane_x,
        pane_y,
    } = w;
    let mut steps = vec![
        // 1. DRAG A NESTED CHILD until it pins against
        //    its parent's inner edge. Ends pinned — an
        //    earlier cut dragged it back again, which
        //    added motion without adding a fact.
        Step::MoveTo {
            col: dx,
            row: dy,
            secs: 0.45,
        },
        Step::Dwell(0.15),
        Step::Press,
        Step::Dwell(0.1),
        Step::MoveTo {
            col: clamp_x,
            row: dy,
            secs: 0.55,
        },
        Step::Dwell(0.3),
        Step::Release,
        Step::Dwell(0.3),
    ];

    // 1b. DRIVE THE INPUT WIDGET FROM THE KEYBOARD, which is what it is
    //     for: its two fields are D's width and height, and D is the node
    //     the previous beat just dragged. Tab to it, type, apply.
    //
    //     The pointer does not move for any of this and does not need to.
    //     An earlier version of this beat resized D by dragging its
    //     bottom-right grip instead, on the theory that a keyboard change
    //     with the cursor elsewhere would read as the app acting on its
    //     own. It does not — a terminal UI that says "Tab: switch, Enter:
    //     apply" on its face is legible without a cursor to point at it,
    //     and mouse resize is another example's subject anyway.
    //
    //     The Tab count is exact rather than hopeful. `SelectNext` walks
    //     INSERTION order, the nodes go A B C D E F G H, and the press in
    //     step 1 selected D — so F is two Tabs away. Anywhere else in the
    //     script the selection would be whatever the last press left
    //     behind, and the pan beat presses empty canvas, which clears it.
    steps.extend([
        Step::Dwell(0.35),
        Step::Key(Key::Tab),
        Step::Dwell(0.35),
        Step::Key(Key::Tab),
        // Enter on a selected Input node clears both fields and focuses
        // width, so the digits below start from empty rather than
        // appending to "15".
        Step::Dwell(0.45),
        Step::Key(Key::Enter),
        Step::Dwell(0.4),
    ]);
    // Narrower and taller than the 15x6 it starts at, so both numbers
    // visibly do something. Growing the WIDTH would have been the obvious
    // choice and is the wrong one: step 1 pinned D against its parent's
    // right edge, and the apply clamps width to what is left inside the
    // parent — so a bigger number would have landed on screen as no change
    // at all.
    steps.extend(typed("24"));
    steps.extend([Step::Dwell(0.3), Step::Key(Key::Tab), Step::Dwell(0.3)]);
    steps.extend(typed("12"));
    steps.extend([Step::Dwell(0.4), Step::Key(Key::Enter), Step::Dwell(0.7)]);

    steps.extend([
        // 2. DRAG A NODE IN A CIRCLE. One loop keeps
        //    its edges re-routing for the whole
        //    gesture; the straight drag this replaced
        //    made the same point once and stopped.
        //
        //    The Sparkline, not the image node: the
        //    image sits near the bottom-left corner, and
        //    orbiting it pushed the pointer into the
        //    auto-pan edge zone, which scrolled the
        //    whole canvas away mid-gesture.
        Step::MoveTo {
            col: gx,
            row: gy,
            secs: 0.45,
        },
        Step::Press,
        Step::Dwell(0.1),
        Step::Orbit {
            rx: 13.0,
            ry: 6.0,
            secs: 1.7,
        },
        Step::Dwell(0.15),
        Step::Release,
        Step::Dwell(0.3),
        // 3. PAN THE CANVAS by dragging empty space.
        //    A long throw, so it reads as panning rather
        //    than as another small nudge.
        Step::MoveTo {
            col: pane_x,
            row: pane_y,
            secs: 0.4,
        },
        Step::Press,
        Step::Dwell(0.1),
        Step::MoveTo {
            col: pane_x + 24,
            row: pane_y.saturating_sub(7),
            secs: 0.65,
        },
        Step::Dwell(0.15),
        Step::Release,
        Step::Dwell(0.3),
        // 4. ZOOM with the wheel, which magnifies around
        //    the pointer rather than the viewport centre.
        Step::MoveTo {
            col: dx,
            row: dy,
            secs: 0.4,
        },
        Step::Scroll {
            up: true,
            clicks: 2,
        },
        Step::Dwell(0.6),
        Step::Scroll {
            up: false,
            clicks: 2,
        },
        Step::Dwell(0.3),
        // 5. FIT, so the last frame is the whole graph
        //    — the frame that gets pulled as the still
        //    for the social card.
        //
        //    The pointer does not travel for this. It
        //    used to: the tape pressed 'f' and the
        //    script first walked the cursor to an
        //    "empty" cell, but the empty-cell search
        //    scans from the bottom-left, which is
        //    exactly where the Controls widget sits, so
        //    it came to rest on [f] just as the view
        //    recentred — reading as a click on a
        //    keyboard-only control. Nothing needs to
        //    move for a viewport command, so nothing
        //    does. The pointer stays on the node it
        //    just zoomed.
        //
        //    Dwell covers the fit animation. Cutting on
        //    a half-finished ease is the one thing that
        //    looks broken in a still.
        Step::Fit,
        Step::Dwell(1.2),
    ]);

    steps
}

/// How long [`demo_steps`] runs, in seconds.
///
/// Waypoints do not affect timing — only the `secs` and `Dwell` values do —
/// so this can answer without a terminal, a layout, or a running app. That
/// is what lets `RATAFLOW_DEMO=duration` print it before ratatui starts.
fn demo_script_secs() -> f64 {
    rataflow_examples::autopilot::duration(&demo_steps(Waypoints::default()))
}
/// The recording script, built from the CURRENT layout.
///
/// Lifted out of the key handler so it can be fired two ways: by 'g' when a
/// human is driving, and automatically at startup under RATAFLOW_DEMO=auto.
/// The auto path exists for capture setups that cannot send keystrokes —
/// recording Ghostty with `screencapture`, say, where synthesising a keypress
/// would mean asking for Accessibility permission on top of Screen Recording.
fn demo_pilot(flow: &Flow<OverviewNode, MixedEdge>) -> Option<Autopilot> {
    // Waypoints are read from the CURRENT layout rather than
    // hardcoded, so the script survives the graph moving.
    let mid = |r: (i32, i32, i32, i32)| {
        (
            ((r.0 + r.2) / 2).max(0) as u16,
            ((r.1 + r.3) / 2).max(0) as u16,
        )
    };
    if let (Some(d), Some(c), Some(g)) = (
        flow.node_terminal_rect("D"),
        flow.node_terminal_rect("C"),
        flow.node_terminal_rect("B"),
    ) {
        let (dx, dy) = mid(d);
        let (gx, gy) = mid(g);
        // Just inside the parent's right edge: far enough to
        // pin the child against it, which is the thing worth
        // seeing (the old sweep stopped well short and never
        // touched the boundary at all).
        let clamp_x = (c.2 - 4).max(0) as u16;

        // Empty canvas for the pan beat, FOUND rather than
        // guessed. Offsetting from the parent's corner put the
        // press inside the Sparkline, so the pan beat grabbed
        // that node and dragged it instead — a press only pans
        // when it hits MouseHit::Nothing.
        //
        // Every node's rect gets a margin, because a press one
        // cell off a border still lands on the node. The 6-cell
        // inset keeps the whole throw out of the auto-pan edge
        // zone, which otherwise scrolls the canvas on its own
        // and makes the drag look like it did something else.
        let occupied: Vec<(i32, i32, i32, i32)> = flow
            .nodes()
            .filter_map(|n| flow.node_terminal_rect(&n.id))
            .collect();
        let size = flow.canvas_size();
        let (cw, ch) = (size.width as i32, size.height as i32);
        const INSET: i32 = 6;
        const THROW_X: i32 = 24;
        const THROW_Y: i32 = 7;
        let free = |x: i32, y: i32| {
            !occupied
                .iter()
                .any(|(l, t, r, b)| x >= l - 2 && x <= r + 2 && y >= t - 1 && y <= b + 1)
        };
        let mut spot = None;
        'search: for y in (INSET..ch - INSET).rev() {
            for x in INSET..cw - INSET - THROW_X {
                // The release point has to be clear too, or the
                // pan ends on a node and the next press misfires.
                if free(x, y) && free(x + THROW_X, y - THROW_Y) {
                    spot = Some((x as u16, y as u16));
                    break 'search;
                }
            }
        }
        let (pane_x, pane_y) = spot.unwrap_or((INSET as u16, (ch - INSET) as u16));

        return Some(
            Autopilot::new(demo_steps(Waypoints {
                dx,
                dy,
                gx,
                gy,
                clamp_x,
                pane_x,
                pane_y,
            }))
            .starting_at(dx.saturating_sub(20), dy + 8),
        );
    }
    None
}

/// Translates a scripted [`Key`] into the crossterm event the handler expects.
fn demo_key_event(key: Key) -> crossterm::event::KeyEvent {
    use crossterm::event::{KeyEvent, KeyModifiers};
    let code = match key {
        Key::Tab => KeyCode::Tab,
        Key::Enter => KeyCode::Enter,
        Key::Char(c) => KeyCode::Char(c),
    };
    // NONE matters: rataflow reads Tab as SelectNext only with no modifiers,
    // and Shift+Tab as SelectPrev.
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// The example's own key handling, lifted out of the event loop.
///
/// It exists as a function so a scripted key — see [`Step::Key`] — can travel
/// exactly the path a real keypress takes. Handling demo keys separately was
/// the alternative and it is the same trap the autopilot was built to avoid:
/// a second copy of the logic that drifts from the one users hit.
///
/// Returns `true` when the app should quit.
fn handle_key(key: crossterm::event::KeyEvent, flow: &mut Flow<OverviewNode, MixedEdge>) -> bool {
    let f_focus = flow.node("F").and_then(|n| {
        if let OverviewNode::Input { focus, .. } = &n.content {
            if *focus > 0 && n.selected {
                Some(*focus)
            } else {
                None
            }
        } else {
            None
        }
    });

    if f_focus.is_some() {
        match key.code {
            KeyCode::Esc => {
                if let Some(OverviewNode::Input { focus, .. }) = flow.node_content_mut("F") {
                    *focus = 0;
                }
            }
            KeyCode::Tab => {
                if let Some(OverviewNode::Input { focus, .. }) = flow.node_content_mut("F") {
                    *focus = if *focus == 1 { 2 } else { 1 };
                }
            }
            KeyCode::Enter => {
                // Apply values to D, clamped to fit within parent C.
                if let Some(f_node) = flow.node("F") {
                    if let OverviewNode::Input {
                        width_value,
                        height_value,
                        ..
                    } = &f_node.content
                    {
                        let new_width: f64 = width_value.parse().unwrap_or(15.0);
                        let new_height: f64 = height_value.parse().unwrap_or(6.0);

                        let (clamped_w, clamped_h) =
                            if let (Some(parent), Some(child)) = (flow.node("C"), flow.node("D")) {
                                let max_w = (parent.width - child.position.x).max(5.0);
                                let max_h = (parent.height - child.position.y).max(3.0);
                                (
                                    new_width.max(5.0).min(max_w),
                                    new_height.max(3.0).min(max_h),
                                )
                            } else {
                                (new_width.max(5.0), new_height.max(3.0))
                            };

                        flow.set_node_dimensions("D", clamped_w, clamped_h);

                        if let Some(OverviewNode::Input {
                            width_value,
                            height_value,
                            focus,
                            ..
                        }) = flow.node_content_mut("F")
                        {
                            *width_value = (clamped_w as u32).to_string();
                            *height_value = (clamped_h as u32).to_string();
                            *focus = 0;
                        }
                    }
                } else if let Some(OverviewNode::Input { focus, .. }) = flow.node_content_mut("F") {
                    *focus = 0;
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if let Some(OverviewNode::Input {
                    width_value,
                    height_value,
                    focus,
                    ..
                }) = flow.node_content_mut("F")
                {
                    let field = if *focus == 1 {
                        width_value
                    } else {
                        height_value
                    };
                    if field.len() < 3 {
                        field.push(c);
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(OverviewNode::Input {
                    width_value,
                    height_value,
                    focus,
                    ..
                }) = flow.node_content_mut("F")
                {
                    let field = if *focus == 1 {
                        width_value
                    } else {
                        height_value
                    };
                    field.pop();
                }
            }
            _ => {}
        }
    } else {
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('p') => {
                if let Some(OverviewNode::Image { use_halfblocks, .. }) = flow.node_content_mut("G")
                {
                    *use_halfblocks = !*use_halfblocks;
                }
            }
            KeyCode::Enter => {
                if flow.node("F").is_some_and(|n| n.selected)
                    && let Some(OverviewNode::Input {
                        focus,
                        width_value,
                        height_value,
                        ..
                    }) = flow.node_content_mut("F")
                {
                    width_value.clear();
                    height_value.clear();
                    *focus = 1;
                }
            }
            _ => {
                let response = flow.handle_controls_key_event(key);
                if matches!(response, EventResponse::NotHandled) {
                    flow.handle_key_event(key);
                }
            }
        }
    }
    false
}

fn main() -> rataflow_examples::Result<()> {
    let mut nodes = vec![
        Node::new(
            "A",
            (-30.0, 10.0),
            (18.0, 5.0),
            OverviewNode::Text(TextContent::from("ratatui Paragraph\nbuiltin")),
        )
        .with_handles(vec![Handle::source(HandlePosition::Right).with_style(
            HandleStyle::new('◉', Style::default().fg(Color::Indexed(71))),
        )])
        .with_selected(true),
        Node::new(
            "B",
            (0.0, 22.0),
            (26.0, 6.0),
            OverviewNode::Sparkline {
                label: "ratatui Sparkline".to_string(),
                data: vec![
                    3, 7, 2, 9, 4, 8, 1, 6, 5, 8, 3, 7, 2, 5, 9, 4, 6, 3, 8, 5, 4, 7, 2, 8, 5, 9,
                    3, 6, 4, 7, 2, 8, 5, 9, 3, 6, 4, 7, 2, 8,
                ],
            },
        )
        .with_handles(vec![
            Handle::target(HandlePosition::Left).with_style(HandleStyle::new(
                '◆',
                Style::default().fg(Color::Indexed(80)),
            )),
            Handle::source(HandlePosition::Right)
                .with_id("right")
                .with_style(HandleStyle::new(
                    '◇',
                    Style::default().fg(Color::Indexed(179)),
                )),
            Handle::source(HandlePosition::Bottom)
                .with_id("bottom")
                .with_style(HandleStyle::new(
                    '●',
                    Style::default().fg(Color::Indexed(133)),
                )),
        ]),
        Node::new(
            "C",
            (30.0, 0.0),
            (35.0, 18.0),
            OverviewNode::Text(TextContent::from("Parent node")),
        )
        .with_opaque(false)
        .with_handles(vec![Handle::target(HandlePosition::Left).with_offset(0.9)]),
        // D is a child of C — position is relative to C.
        Node::new(
            "D",
            (5.0, 5.0),
            (15.0, 6.0),
            OverviewNode::Text(TextContent::from("Nested child\n(drag me)")),
        )
        .with_parent("C")
        .with_extent(NodeExtent::Parent)
        .with_handles(vec![
            Handle::target(HandlePosition::Left),
            Handle::source(HandlePosition::Right)
                .with_id("out1")
                .with_offset(0.3),
            Handle::source(HandlePosition::Right)
                .with_id("out2")
                .with_offset(0.7),
        ]),
        Node::new(
            "E",
            (85.0, 0.0),
            (16.0, 8.0),
            OverviewNode::RawBuffer {
                label: "buf[(x, y)]".to_string(),
            },
        )
        .with_handles(vec![Handle::target(HandlePosition::Left)]),
        Node::new(
            "F",
            (85.0, 16.0),
            (28.0, 7.0),
            OverviewNode::Input {
                label: "Input".to_string(),
                width_value: "15".to_string(),
                height_value: "6".to_string(),
                focus: 0,
            },
        )
        .with_handles(vec![Handle::target(HandlePosition::Left)]),
    ];

    let dyn_img = image::open("assets/ferris.png").expect("Failed to load ferris.png");
    // Protocol detection is IO-based (ratatui-image >= 10.0.6): the capability query wins over
    // env hints, so no KITTY_WINDOW_ID override is needed — that env var is stale under tmux and
    // inherited by terminals launched from kitty.
    //
    // Warp is the exception: it answers the kitty capability query but never implemented the
    // unicode-placeholder half of the protocol (warpdotdev/Warp#6210), so placeholders render as
    // tofu, and its iTerm2 path doesn't clear either — no protocol works there. Fall back to
    // halfblocks, the same workaround ratatui-image applies to WezTerm and Konsole. A terminal
    // launched from Warp that doesn't set its own TERM_PROGRAM is a false positive, but that
    // degrades to blocks rather than breaking.
    let is_warp = std::env::var("TERM_PROGRAM").is_ok_and(|term| term == "WarpTerminal");
    let native_picker = if is_warp {
        Picker::halfblocks()
    } else {
        Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks())
    };
    let halfblocks_picker = Picker::halfblocks();

    nodes.push(
        Node::new(
            "G",
            (5.0, 38.0),
            (24.0, 9.0),
            OverviewNode::Image {
                label: "ratatui-image".to_string(),
                native_protocol: Box::new(RefCell::new(
                    native_picker.new_resize_protocol(dyn_img.clone()),
                )),
                halfblocks_protocol: Box::new(RefCell::new(
                    halfblocks_picker.new_resize_protocol(dyn_img),
                )),
                native_protocol_type: native_picker.protocol_type(),
                use_halfblocks: false,
            },
        )
        .with_handles(vec![
            Handle::target(HandlePosition::Top),
            Handle::source(HandlePosition::Right),
        ]),
    );

    let fx_effect = fx::parallel(&[
        fx::sweep_in(
            Motion::LeftToRight,
            10,
            0,
            Color::Indexed(234),
            (1200, Interpolation::CubicOut),
        ),
        fx::repeating(fx::hsl_shift_fg(
            [360.0, 0.0, 0.0],
            (3000, Interpolation::Linear),
        )),
    ]);

    nodes.push(
        Node::new(
            "H",
            (45.0, 35.0),
            (20.0, 6.0),
            OverviewNode::Fx {
                label: "tachyonfx".to_string(),
                effect: RefCell::new(fx_effect),
                last_tick: Cell::new(Instant::now()),
            },
        )
        .with_handles(vec![Handle::target(HandlePosition::Left)]),
    );

    let edges: Vec<Edge<MixedEdge>> = vec![
        Edge::new("e_ab", "A", "B")
            .with_content(MixedEdge::Straight {
                // Braille, matching `StraightEdge`'s own default — a straight edge
                // runs at whatever angle the nodes leave it at, and one character
                // per cell can only staircase that.
                style: EdgeStyle::braille()
                    .with_stroke_style(Style::default().fg(Color::Indexed(208)))
                    .with_marker_start(EdgeMarker::Custom('■'))
                    .with_marker_end(EdgeMarker::ArrowClosed),
                selected_style: EdgeStyle::braille(),
            })
            .with_animated(true)
            .with_label("EdgeContent trait\nstraight path"),
        Edge::new("e_ad", "A", "D"),
        Edge::new("e_de", "D", "E")
            .with_source_handle(Some("out1".to_string()))
            .with_animated(true),
        Edge::new("e_df", "D", "F")
            .with_source_handle(Some("out2".to_string()))
            .with_content(MixedEdge::Step {
                // Dotted, to show the stroke is a choice per edge rather than a
                // property of the routing.
                style: EdgeStyle::dotted()
                    .with_stroke_style(Style::default().fg(Color::Indexed(114)))
                    .with_marker_end(EdgeMarker::Arrow),
                selected_style: EdgeStyle::dotted().with_marker_end(EdgeMarker::Arrow),
            }),
        Edge::new("e_bc", "B", "C").with_source_handle(Some("right".to_string())),
        Edge::new("e_bg", "B", "G").with_source_handle(Some("bottom".to_string())),
        Edge::new("e_gh", "G", "H"),
    ];

    // Answer and exit before ratatui touches the terminal, so this works over a
    // pipe — which is the point: the recorder and the asset build both ask for
    // it, and neither has a tty to spare. See demo_script_secs.
    if std::env::var("RATAFLOW_DEMO").as_deref() == Ok("duration") {
        println!("{:.2}", demo_script_secs());
        return Ok(());
    }

    let mut flow: Flow<OverviewNode, MixedEdge> = Flow::with_graph(nodes, edges)?;
    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;

    let mut last_tick = Instant::now();
    flow.request_fit_view();

    // Recording-only autopilot (set RATAFLOW_DEMO=1), triggered by the
    // otherwise-unused 'g'. VHS is keyboard-only and films a headless terminal
    // with no OS pointer, so a mouse demo has to be drawn and driven by the app
    // itself. See rataflow_examples::autopilot.
    let demo_var = std::env::var("RATAFLOW_DEMO").unwrap_or_default();
    let demo = !demo_var.is_empty();
    // "auto" fires the script itself, one frame in. Waiting for a frame matters:
    // the waypoints come from node_terminal_rect, which is empty until the flow
    // has been laid out once.
    let auto = demo_var == "auto";
    let mut auto_armed = auto;
    let mut pilot: Option<Autopilot> = None;
    // Seconds of settled graph before the pointer moves, so an external
    // recorder has time to start and the clip opens on the layout rather than
    // halfway through the first glide. The tape got this free by launching the
    // binary under `Hide` and pressing 'g' whenever it liked; a screen recorder
    // has no such handle, so the wait lives here instead.
    let lead = std::env::var("RATAFLOW_DEMO_LEAD")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.2);
    let mut laid_out: Option<Instant> = None;

    'app: loop {
        let now = Instant::now();
        let elapsed = now - last_tick;
        last_tick = now;
        flow.tick_animation(elapsed);
        flow.tick_auto_pan(elapsed);

        if auto_armed && flow.node_terminal_rect("D").is_some() {
            // Two gates, and they are different things. The layout has to exist
            // before the waypoints can be read from it; the lead is then timed
            // from that moment, not from process start, so it is not spent
            // waiting on the fit animation.
            let since = *laid_out.get_or_insert(now);
            if (now - since).as_secs_f64() >= lead {
                pilot = demo_pilot(&flow);
                auto_armed = false;
            }
        }

        // Feed the scripted pointer's events through the REAL mouse path, so the
        // recording shows hit testing, the click-vs-drag threshold and live edge
        // re-routing rather than an animation of a node's position.
        if let Some(p) = pilot.as_mut() {
            for ev in p.tick(elapsed) {
                flow.handle_mouse_event(ev);
            }
            // The closing fit is a Step, not the 'f' key: see Step::Fit. Polled
            // after the events so a fit never lands mid-drag.
            if p.take_fit() {
                flow.request_fit_view();
            }
            // Scripted keys go through handle_key, not a shortcut into the
            // flow: some of them are this example's own input-widget logic.
            // The quit result is ignored because no script types 'q'.
            for k in p.take_keys() {
                handle_key(demo_key_event(k), &mut flow);
            }
        }

        // Under `auto` the run IS the recording, so it ends when the script
        // does. A capture that has to be stopped from outside would either
        // trail dead frames or get killed mid-run, and a kill skips the
        // terminal restore — leaving the alt screen up and the tty in raw mode.
        if auto && pilot.as_ref().is_some_and(|p| p.finished()) {
            break 'app;
        }

        terminal.draw(|frame| {
            let area = render_shell(frame, frame.area(), &meta());

            // Flat dark background (was vertical gradient with all components < 30)
            let color = Color::Indexed(234);
            let buf = frame.buffer_mut();
            for row in 0..area.height {
                for x in area.x..area.x + area.width {
                    buf[(x, area.y + row)].set_bg(color);
                }
            }

            frame.render_widget(
                Background::new(&flow)
                    .variant(BackgroundVariant::Dots)
                    .gap(10, 5)
                    .style(BackgroundStyle::default().with_pattern_color(Color::Indexed(24))),
                area,
            );
            frame.render_widget(&mut flow, area);
            frame.render_widget(Controls::new(&flow), area);
            frame.render_widget(MiniMap::new(&flow), area);
            // Last, so the pointer sits on top of everything it is pointing at.
            if let Some(p) = pilot.as_ref() {
                p.draw(frame.buffer_mut());
            }
        })?;

        // Poll with 16ms timeout (~60fps frame budget). If any events arrived,
        // drain all pending events before next render. Without draining, mouse
        // events (125-1000Hz) accumulate faster than render rate, causing input lag.
        if event::poll(Duration::from_millis(16))? {
            loop {
                match event::read()? {
                    // Recording-only: trigger the one-shot horizontal child drag.
                    CrosstermEvent::Key(key) if demo && key.code == KeyCode::Char('g') => {
                        pilot = demo_pilot(&flow);
                    }
                    CrosstermEvent::Key(key) => {
                        if handle_key(key, &mut flow) {
                            break 'app;
                        }
                    }
                    CrosstermEvent::Mouse(mouse) => {
                        for event in flow.handle_mouse_event(mouse).into_events() {
                            match event {
                                FlowEvent::ConnectionCompleted(conn) => {
                                    flow.add_edge_from_connection(conn, MixedEdge::default());
                                }
                                FlowEvent::ReconnectionCompleted {
                                    edge_id,
                                    new_connection,
                                    ..
                                } => {
                                    flow.reconnect_edge(&edge_id, new_connection);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
                // poll(ZERO) checks for another event without blocking.
                // If none are buffered, break out and render.
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
    }

    execute!(stdout(), DisableMouseCapture)?;
    ratatui::restore();
    Ok(())
}
