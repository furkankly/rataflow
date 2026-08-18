//! A scripted mouse pointer, for recordings only.
//!
//! VHS (which records the demo GIFs) is keyboard-only — its entire command set
//! is `Type`/arrows/`Ctrl`/`Sleep`, with no way to move or click a mouse. And
//! even if it could, it films a headless terminal where no OS pointer exists to
//! film. So a GIF of a mouse-driven feature is impossible to capture directly.
//!
//! This closes that gap from the other side: rataflow owns every cell of the
//! frame, so the *application* can move a pointer and draw it. An autopilot runs
//! a script of waypoints, emits real [`MouseEvent`]s into the flow, and paints a
//! cursor glyph where that pointer is.
//!
//! The events are the point. An earlier version of the overview demo animated
//! `set_node_position` directly, which looked wrong because nothing was actually
//! being dragged — no hit test, no drag threshold, no edge re-routing through
//! the real path. Synthesizing `Down`/`Drag`/`Up` runs the genuine state
//! machine, so the recording shows the library working rather than a cartoon of
//! it.
//!
//! Everything here is demo scaffolding, gated by the caller behind
//! `RATAFLOW_DEMO=1`. Nothing reaches a real user.

use std::time::Duration;

use ratatui::buffer::Buffer;

use rataflow::{MouseButton, MouseEvent, MouseEventKind};

/// U+1F400. Double-width in every terminal that renders it.
const RAT: &str = "\u{1F400}";

/// A key the script can press.
///
/// Deliberately tiny. The obvious move is to carry a `crossterm::KeyCode`, but
/// this crate does not depend on crossterm — the examples do — and rataflow's
/// own `KeyCode` converts *from* crossterm rather than to it, so either choice
/// would have meant a translation table somewhere. Three variants cover what a
/// demo script needs to type, and the example maps them to whatever its own
/// handler takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Tab,
    Enter,
    Char(char),
}

/// One instruction in a pointer script.
#[derive(Clone, Debug)]
pub enum Step {
    /// Glide to a terminal cell over `secs`.
    MoveTo { col: u16, row: u16, secs: f64 },
    /// Press the left button where the pointer currently is.
    Press,
    /// Release the left button.
    Release,
    /// Hold still. People stop before they click and after they drop, so a few
    /// short pauses are what separate a pointer from an interpolation. Keep
    /// them short: long ones are what made the first cut feel like a
    /// screensaver.
    Dwell(f64),
    /// Wheel clicks at the current position. The library zooms around the
    /// cursor, so this reads as "zoom into where I am pointing".
    Scroll { up: bool, clicks: u8 },
    /// Sweep once around an ellipse, starting from wherever the pointer is.
    ///
    /// The centre is derived so the current position IS the start of the arc,
    /// which is what keeps a held node from teleporting when the orbit begins.
    /// Dragging a node around a loop keeps its edges re-routing for the whole
    /// gesture — a straight drag shows the same fact once and then stops.
    ///
    /// `rx`/`ry` are in cells, and cells are about twice as tall as wide, so a
    /// visually round circle wants `rx` roughly double `ry`.
    Orbit { rx: f64, ry: f64, secs: f64 },
    /// Refit the viewport around the whole graph, so the recording ends on the
    /// picture it started with.
    ///
    /// This used to be a keypress in the tape ('f', after the script finished),
    /// which quietly made the closing frame depend on something outside the
    /// script. Two costs. The pointer had to be walked somewhere harmless first,
    /// and the only "empty" cell the search reliably found was on top of the
    /// Controls widget — so the cursor came to rest on [f] exactly as the view
    /// recentred, reading as a click on a keyboard-only control. And a capture
    /// setup that cannot type — recording Ghostty with `screencapture`, where
    /// synthesising a keypress means asking for Accessibility permission on top
    /// of Screen Recording — could not produce the ending at all.
    ///
    /// As a step it is neither: nothing moves, the fit just happens. Follow it
    /// with a [`Step::Dwell`] long enough for the animation to settle, since the
    /// last frame is the one that gets used as a still.
    Fit,
    /// Press a key, which the caller feeds to its own key handler.
    ///
    /// Not every demo beat is a gesture. A widget whose whole interface is
    /// "Tab: switch, Enter: apply" is driven from the keyboard, and filming it
    /// any other way would be inventing an interaction the app does not have.
    /// The pointer simply stays where it was.
    ///
    /// These do NOT go to `Flow::handle_key_event`. Some of them are the
    /// example's own logic — an input field's digits are not a library concern
    /// — so they have to travel the same path a real keypress does, or the
    /// recording stops being a recording of the app.
    Key(Key),
}

/// How long a script takes, in seconds.
///
/// The single source of truth for that number. It used to be written by hand in
/// three places — here in the steps, in the recorder's sanity check, and in the
/// tape's `Sleep` — and adding a beat meant remembering all three. The tape is
/// the one that punishes a miss: it cuts the recording mid-gesture.
///
/// Only the timed steps count. `Press`, `Release`, `Fit` and `Key` are
/// instantaneous; the pauses around them are `Dwell`s.
pub fn duration(steps: &[Step]) -> f64 {
    steps
        .iter()
        .map(|s| match s {
            Step::MoveTo { secs, .. } | Step::Orbit { secs, .. } => *secs,
            Step::Dwell(secs) => *secs,
            Step::Press | Step::Release | Step::Scroll { .. } | Step::Fit | Step::Key(_) => 0.0,
        })
        .sum()
}

/// Expands a string into one [`Step::Key`] per character, with a pause between.
///
/// Typing arrives one key at a time; emitting a whole field at once looks like
/// a paste, which is not what the widget is demonstrating.
pub fn typed(text: &str) -> Vec<Step> {
    text.chars()
        .flat_map(|c| [Step::Key(Key::Char(c)), Step::Dwell(0.18)])
        .collect()
}

/// Runs a [`Step`] script, emitting mouse events and tracking a cursor.
pub struct Autopilot {
    steps: Vec<Step>,
    index: usize,
    /// Seconds elapsed inside the current step.
    t: f64,
    from: (f64, f64),
    pos: (f64, f64),
    pressed: bool,
    /// Last cell an event was emitted for, so a glide emits one event per cell
    /// crossed rather than one per frame.
    last_cell: (u16, u16),
    finished: bool,
    /// Set by [`Step::Fit`], cleared by [`Autopilot::take_fit`]. A flag rather
    /// than an event because `tick` returns mouse events and a fit is not one —
    /// it is a viewport command, and forging a keystroke to carry it would put
    /// the script back through the key handler it just escaped.
    fit_pending: bool,
    /// Keys that came due this tick, drained by [`Autopilot::take_keys`]. A
    /// queue rather than a single slot because several `Dwell`-free steps can
    /// fall inside one frame's budget.
    keys: Vec<Key>,
}

impl Autopilot {
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            index: 0,
            t: 0.0,
            from: (0.0, 0.0),
            pos: (0.0, 0.0),
            pressed: false,
            last_cell: (u16::MAX, u16::MAX),
            finished: false,
            fit_pending: false,
            keys: Vec::new(),
        }
    }

    /// Starts the pointer somewhere other than the top-left.
    pub fn starting_at(mut self, col: u16, row: u16) -> Self {
        self.pos = (col as f64, row as f64);
        self.from = self.pos;
        self
    }

    pub fn finished(&self) -> bool {
        self.finished
    }

    pub fn cursor(&self) -> (u16, u16) {
        (self.pos.0.round() as u16, self.pos.1.round() as u16)
    }

    pub fn pressed(&self) -> bool {
        self.pressed
    }

    /// Whether a [`Step::Fit`] came due since the last call, clearing it.
    ///
    /// Callers driving an [`Autopilot`] directly must poll this after `tick`;
    /// [`DemoPilot::tick_into`] does it for you.
    pub fn take_fit(&mut self) -> bool {
        std::mem::take(&mut self.fit_pending)
    }

    /// Keys pressed since the last call, clearing them.
    ///
    /// Poll after `tick`, alongside [`take_fit`](Self::take_fit).
    pub fn take_keys(&mut self) -> Vec<Key> {
        std::mem::take(&mut self.keys)
    }

    /// Advances the script and returns the events to feed to the flow.
    ///
    /// Returns a Vec because a single frame can complete a step and begin the
    /// next — releasing and starting to move away in the same tick.
    pub fn tick(&mut self, elapsed: Duration) -> Vec<MouseEvent> {
        let mut events = Vec::new();
        if self.finished {
            return events;
        }
        let mut budget = elapsed.as_secs_f64();

        // A step can consume only part of the frame's time, so loop until the
        // budget is spent or the script ends. Without this, several short
        // Dwells in a row would each cost a whole frame.
        while budget > 0.0 && !self.finished {
            let Some(step) = self.steps.get(self.index).cloned() else {
                self.finished = true;
                break;
            };
            match step {
                Step::MoveTo { col, row, secs } => {
                    let target = (col as f64, row as f64);
                    if self.t == 0.0 {
                        self.from = self.pos;
                    }
                    self.t += budget;
                    let done = self.t >= secs;
                    let k = if secs <= 0.0 {
                        1.0
                    } else {
                        (self.t / secs).clamp(0.0, 1.0)
                    };
                    let e = ease_in_out(k);
                    self.pos = (
                        self.from.0 + (target.0 - self.from.0) * e,
                        self.from.1 + (target.1 - self.from.1) * e,
                    );
                    if let Some(ev) = self.motion_event() {
                        events.push(ev);
                    }
                    if done {
                        budget = self.t - secs;
                        self.t = 0.0;
                        self.index += 1;
                    } else {
                        budget = 0.0;
                    }
                }
                Step::Dwell(secs) => {
                    self.t += budget;
                    if self.t >= secs {
                        budget = self.t - secs;
                        self.t = 0.0;
                        self.index += 1;
                    } else {
                        budget = 0.0;
                    }
                }
                Step::Press => {
                    self.pressed = true;
                    let (c, r) = self.cursor();
                    events.push(MouseEvent::new(
                        MouseEventKind::Down(MouseButton::Left),
                        c,
                        r,
                    ));
                    self.index += 1;
                }
                Step::Release => {
                    self.pressed = false;
                    let (c, r) = self.cursor();
                    events.push(MouseEvent::new(MouseEventKind::Up(MouseButton::Left), c, r));
                    self.index += 1;
                }
                Step::Orbit { rx, ry, secs } => {
                    // Anchor the ellipse so angle 0 is the pointer's position at
                    // the moment the orbit starts.
                    if self.t == 0.0 {
                        self.from = (self.pos.0 - rx, self.pos.1);
                    }
                    self.t += budget;
                    let done = self.t >= secs;
                    let k = if secs <= 0.0 {
                        1.0
                    } else {
                        (self.t / secs).clamp(0.0, 1.0)
                    };
                    // Smoothstep, not the cubic ease used for straight moves.
                    // Cubic is so front/back-loaded that a quarter of the way
                    // through the loop the node has travelled six degrees — it
                    // reads as a stall, not a swing.
                    let angle = smoothstep(k) * std::f64::consts::TAU;
                    self.pos = (
                        self.from.0 + rx * angle.cos(),
                        self.from.1 + ry * angle.sin(),
                    );
                    if let Some(ev) = self.motion_event() {
                        events.push(ev);
                    }
                    if done {
                        budget = self.t - secs;
                        self.t = 0.0;
                        self.index += 1;
                    } else {
                        budget = 0.0;
                    }
                }
                Step::Scroll { up, clicks } => {
                    let (c, r) = self.cursor();
                    let kind = if up {
                        MouseEventKind::ScrollUp
                    } else {
                        MouseEventKind::ScrollDown
                    };
                    for _ in 0..clicks {
                        events.push(MouseEvent::new(kind, c, r));
                    }
                    self.index += 1;
                }
                Step::Fit => {
                    self.fit_pending = true;
                    self.index += 1;
                }
                Step::Key(k) => {
                    self.keys.push(k);
                    self.index += 1;
                }
            }
        }
        events
    }

    /// One event per cell entered. Emitting per frame instead would flood the
    /// drag handler with duplicates at the same coordinate.
    fn motion_event(&mut self) -> Option<MouseEvent> {
        let cell = self.cursor();
        if cell == self.last_cell {
            return None;
        }
        self.last_cell = cell;
        let kind = if self.pressed {
            MouseEventKind::Drag(MouseButton::Left)
        } else {
            MouseEventKind::Moved
        };
        Some(MouseEvent::new(kind, cell.0, cell.1))
    }

    /// Paints the pointer. Call after rendering the flow, so it sits on top.
    ///
    /// A terminal app does not normally draw its own pointer — the emulator
    /// does — so this exists purely so a recording has something to follow.
    pub fn draw(&self, buf: &mut Buffer) {
        let (col, row) = self.cursor();
        if col >= buf.area.width || row >= buf.area.height {
            return;
        }
        // A rat for a mouse pointer: rataflow is built on ratatui, and the pun
        // is free. Verified to render under VHS — plenty of glyphs do not.
        //
        // No press state. An earlier version lit the cell cyan while the button
        // was down, which just read as an unexplained blue box — a real cursor
        // does not change colour when you click. What the button is doing is
        // already legible from the thing being dragged.
        buf[(col, row)].set_symbol(RAT);
        // The rat is double-width. Without blanking the cell it spills into,
        // whatever was underneath shows through its right half as a sliver.
        if col + 1 < buf.area.width {
            buf[(col + 1, row)].set_symbol("");
        }
    }
}

/// Cubic ease-in-out: slow at both ends, quick through the middle.
///
/// This is the whole difference between "a human moved that" and "a computer
/// interpolated that". Linear motion, and the sine sweep the old overview demo
/// used, both read as machinery because they never rest at the endpoints.
/// Gentler than [`ease_in_out`]: eases at both ends but keeps moving between
/// them. Right for sustained motion like an orbit.
fn smoothstep(k: f64) -> f64 {
    k * k * (3.0 - 2.0 * k)
}

fn ease_in_out(k: f64) -> f64 {
    if k < 0.5 {
        4.0 * k * k * k
    } else {
        let f = -2.0 * k + 2.0;
        1.0 - f * f * f / 2.0
    }
}

/// Per-example wiring for the scripted pointer.
///
/// Every example that wants a recording needs the same four things: read the
/// env var, start a script on a key, pump events into the flow each frame, and
/// draw the cursor last. Without this they each grew their own copy, and the
/// copies drifted — the first one animated node positions directly instead of
/// emitting events, and nothing flagged it.
///
/// Usage, in an example's main loop:
///
/// ```ignore
/// let mut demo = DemoPilot::from_env();          // reads RATAFLOW_DEMO
/// // ... on the trigger key:
/// demo.start(vec![Step::MoveTo { .. }, Step::Press, ..]);
/// // ... each frame, before drawing:
/// demo.tick_into(&mut flow, elapsed);
/// // ... inside the draw closure, after rendering the flow:
/// demo.draw(frame.buffer_mut());
/// ```
pub struct DemoPilot {
    enabled: bool,
    pilot: Option<Autopilot>,
}

impl DemoPilot {
    /// Enabled only when `RATAFLOW_DEMO` is set, so a real user of the example
    /// can press the trigger key and nothing happens.
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("RATAFLOW_DEMO").is_ok(),
            pilot: None,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Begins a script. Ignored unless recording, and ignored if one is already
    /// running — a second trigger mid-run would teleport the pointer.
    pub fn start(&mut self, steps: Vec<Step>, from: (u16, u16)) {
        if !self.enabled || self.pilot.is_some() {
            return;
        }
        self.pilot = Some(Autopilot::new(steps).starting_at(from.0, from.1));
    }

    /// Advances the script, feeds its events through the flow's real mouse path
    /// — the same entry point a terminal's mouse reports would take — and
    /// returns whatever the flow emitted.
    ///
    /// Returning the events is not optional. An example that builds edges does
    /// it in response to `ConnectionCompleted`, so a version of this that
    /// swallowed the response could only ever make a connection *preview*: the
    /// drag looked right, the release did nothing, and the graph came out
    /// unchanged. Callers that create nothing can ignore the Vec.
    pub fn tick_into<N: rataflow::NodeContent, E: rataflow::EdgeContent>(
        &mut self,
        flow: &mut rataflow::Flow<N, E>,
        elapsed: Duration,
    ) -> Vec<rataflow::FlowEvent> {
        let mut emitted = Vec::new();
        if let Some(p) = self.pilot.as_mut() {
            for ev in p.tick(elapsed) {
                emitted.extend(flow.handle_mouse_event(ev).into_events());
            }
            // After the events, not before: fitting first would recompute the
            // viewport under a drag that is still in flight.
            if p.take_fit() {
                flow.request_fit_view();
            }
        }
        emitted
    }

    /// Keys that came due this tick, for the caller to feed to its OWN handler.
    ///
    /// Deliberately not folded into [`tick_into`](Self::tick_into) the way mouse
    /// events are: a scripted key is often the example's own logic rather than
    /// the library's (see [`Step::Key`]), and this type cannot tell which. Handing
    /// them back keeps them on the path a real keypress takes.
    pub fn take_keys(&mut self) -> Vec<Key> {
        self.pilot
            .as_mut()
            .map(Autopilot::take_keys)
            .unwrap_or_default()
    }

    /// Paints the cursor. Call last, so it sits above what it points at.
    pub fn draw(&self, buf: &mut Buffer) {
        if let Some(p) = self.pilot.as_ref() {
            p.draw(buf);
        }
    }
}
