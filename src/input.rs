//! Backend-agnostic input types.
//!
//! This module provides abstract input types ([`KeyEvent`], [`MouseEvent`]) that work
//! across terminal backends. Enable a backend feature (`crossterm`, `termion`, `termwiz`,
//! or `ratzilla`) to get automatic `From` conversions from backend-specific types.
//!
//! # Usage
//!
//! ```no_run
//! # #![allow(unused)]
//! use rataflow::{Flow, KeyEvent, MouseEvent};
//!
//! # let mut flow: Flow = Flow::new();
//! # #[cfg(feature = "crossterm")]
//! # {
//! # use ratatui::crossterm::event::{KeyCode as CtKeyCode, KeyEvent as CtKeyEvent};
//! # let crossterm_key = CtKeyEvent::from(CtKeyCode::Char('a'));
//! // With crossterm (default)
//! let key: KeyEvent = crossterm_key.into();
//! flow.handle_key_event(key);
//!
//! // Or pass the backend event straight in — the conversion is implicit
//! flow.handle_key_event(crossterm_key);
//! # }
//! ```
//!
//! # Backend Support
//!
//! - `crossterm` feature: Enables `From<crossterm::event::KeyEvent>` and `From<crossterm::event::MouseEvent>`
//! - `termion` feature: Enables `From<termion::event::Key>` and `From<termion::event::MouseEvent>`
//! - `termwiz` feature: Enables `From<termwiz::input::KeyEvent>` and `From<termwiz::input::MouseEvent>`

/// Key code abstraction covering common keys used in flow graph interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// Up arrow key
    Up,
    /// Down arrow key
    Down,
    /// Left arrow key
    Left,
    /// Right arrow key
    Right,
    /// A character key
    Char(char),
    /// Delete key
    Delete,
    /// Backspace key
    Backspace,
    /// Escape key
    Esc,
    /// Enter/Return key
    Enter,
    /// Tab key
    Tab,
    /// Home key
    Home,
    /// End key
    End,
    /// Page Up key
    PageUp,
    /// Page Down key
    PageDown,
    /// Function key (F1-F12)
    F(u8),
    /// Any other key not explicitly handled
    Other,
}

/// Keyboard modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    /// Shift key is held
    pub shift: bool,
    /// Control key is held
    pub ctrl: bool,
    /// Alt key is held
    pub alt: bool,
}

impl Modifiers {
    /// No modifiers pressed.
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
    };

    /// Shift modifier only.
    pub const SHIFT: Self = Self {
        shift: true,
        ctrl: false,
        alt: false,
    };

    /// Control modifier only.
    pub const CTRL: Self = Self {
        shift: false,
        ctrl: true,
        alt: false,
    };

    /// Alt modifier only.
    pub const ALT: Self = Self {
        shift: false,
        ctrl: false,
        alt: true,
    };

    /// Returns true if any modifier is held.
    pub fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt
    }
}

/// Abstract keyboard event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    /// The key that was pressed.
    pub code: KeyCode,
    /// Modifier keys held during the key press.
    pub modifiers: Modifiers,
}

impl KeyEvent {
    /// Creates a new key event with the given code and no modifiers.
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: Modifiers::NONE,
        }
    }

    /// Creates a new key event with the given code and modifiers.
    pub fn with_modifiers(code: KeyCode, modifiers: Modifiers) -> Self {
        Self { code, modifiers }
    }
}

/// Mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button (scroll wheel click)
    Middle,
    /// Unknown or other button
    Unknown,
}

/// Kind of mouse event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    /// Button pressed down
    Down(MouseButton),
    /// Button released
    Up(MouseButton),
    /// Mouse dragged with button held
    Drag(MouseButton),
    /// Mouse moved (no button held)
    Moved,
    /// Scroll wheel up
    ScrollUp,
    /// Scroll wheel down
    ScrollDown,
    /// Scroll wheel left (horizontal)
    ScrollLeft,
    /// Scroll wheel right (horizontal)
    ScrollRight,
}

/// Abstract mouse event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    /// Kind of mouse event
    pub kind: MouseEventKind,
    /// Column (x) position in terminal coordinates
    pub column: u16,
    /// Row (y) position in terminal coordinates
    pub row: u16,
    /// Modifier keys held during the mouse event
    pub modifiers: Modifiers,
}

impl MouseEvent {
    /// Creates a new mouse event.
    pub fn new(kind: MouseEventKind, column: u16, row: u16) -> Self {
        Self {
            kind,
            column,
            row,
            modifiers: Modifiers::NONE,
        }
    }

    /// Creates a new mouse event with modifiers.
    pub fn with_modifiers(
        kind: MouseEventKind,
        column: u16,
        row: u16,
        modifiers: Modifiers,
    ) -> Self {
        Self {
            kind,
            column,
            row,
            modifiers,
        }
    }
}

// =============================================================================
// Crossterm backend conversions
// =============================================================================

#[cfg(feature = "crossterm")]
mod crossterm_impl {
    use super::*;
    use ratatui::crossterm::event as ct;

    impl From<ct::KeyCode> for KeyCode {
        fn from(code: ct::KeyCode) -> Self {
            match code {
                ct::KeyCode::Up => KeyCode::Up,
                ct::KeyCode::Down => KeyCode::Down,
                ct::KeyCode::Left => KeyCode::Left,
                ct::KeyCode::Right => KeyCode::Right,
                ct::KeyCode::Char(c) => KeyCode::Char(c),
                ct::KeyCode::Delete => KeyCode::Delete,
                ct::KeyCode::Backspace => KeyCode::Backspace,
                ct::KeyCode::Esc => KeyCode::Esc,
                ct::KeyCode::Enter => KeyCode::Enter,
                ct::KeyCode::Tab => KeyCode::Tab,
                ct::KeyCode::Home => KeyCode::Home,
                ct::KeyCode::End => KeyCode::End,
                ct::KeyCode::PageUp => KeyCode::PageUp,
                ct::KeyCode::PageDown => KeyCode::PageDown,
                ct::KeyCode::F(n) => KeyCode::F(n),
                _ => KeyCode::Other,
            }
        }
    }

    impl From<ct::KeyModifiers> for Modifiers {
        fn from(mods: ct::KeyModifiers) -> Self {
            Self {
                shift: mods.contains(ct::KeyModifiers::SHIFT),
                ctrl: mods.contains(ct::KeyModifiers::CONTROL),
                alt: mods.contains(ct::KeyModifiers::ALT),
            }
        }
    }

    impl From<ct::KeyEvent> for KeyEvent {
        fn from(event: ct::KeyEvent) -> Self {
            Self {
                code: event.code.into(),
                modifiers: event.modifiers.into(),
            }
        }
    }

    impl From<ct::MouseButton> for MouseButton {
        fn from(button: ct::MouseButton) -> Self {
            match button {
                ct::MouseButton::Left => MouseButton::Left,
                ct::MouseButton::Right => MouseButton::Right,
                ct::MouseButton::Middle => MouseButton::Middle,
            }
        }
    }

    impl From<ct::MouseEventKind> for MouseEventKind {
        fn from(kind: ct::MouseEventKind) -> Self {
            match kind {
                ct::MouseEventKind::Down(btn) => MouseEventKind::Down(btn.into()),
                ct::MouseEventKind::Up(btn) => MouseEventKind::Up(btn.into()),
                ct::MouseEventKind::Drag(btn) => MouseEventKind::Drag(btn.into()),
                ct::MouseEventKind::Moved => MouseEventKind::Moved,
                ct::MouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
                ct::MouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
                ct::MouseEventKind::ScrollLeft => MouseEventKind::ScrollLeft,
                ct::MouseEventKind::ScrollRight => MouseEventKind::ScrollRight,
            }
        }
    }

    impl From<ct::MouseEvent> for MouseEvent {
        fn from(event: ct::MouseEvent) -> Self {
            Self {
                kind: event.kind.into(),
                column: event.column,
                row: event.row,
                modifiers: event.modifiers.into(),
            }
        }
    }
}

// =============================================================================
// Termion backend conversions
// =============================================================================

#[cfg(feature = "termion")]
mod termion_impl {
    use super::*;
    use ratatui::termion::event as ti;

    impl From<ti::Key> for KeyEvent {
        fn from(key: ti::Key) -> Self {
            let (code, modifiers) = match key {
                ti::Key::Up => (KeyCode::Up, Modifiers::NONE),
                ti::Key::Down => (KeyCode::Down, Modifiers::NONE),
                ti::Key::Left => (KeyCode::Left, Modifiers::NONE),
                ti::Key::Right => (KeyCode::Right, Modifiers::NONE),
                ti::Key::Char(c) => (KeyCode::Char(c), Modifiers::NONE),
                ti::Key::Ctrl(c) => (KeyCode::Char(c), Modifiers::CTRL),
                ti::Key::Alt(c) => (KeyCode::Char(c), Modifiers::ALT),
                ti::Key::Delete => (KeyCode::Delete, Modifiers::NONE),
                ti::Key::Backspace => (KeyCode::Backspace, Modifiers::NONE),
                ti::Key::Esc => (KeyCode::Esc, Modifiers::NONE),
                ti::Key::Home => (KeyCode::Home, Modifiers::NONE),
                ti::Key::End => (KeyCode::End, Modifiers::NONE),
                ti::Key::PageUp => (KeyCode::PageUp, Modifiers::NONE),
                ti::Key::PageDown => (KeyCode::PageDown, Modifiers::NONE),
                ti::Key::BackTab => (KeyCode::Tab, Modifiers::SHIFT),
                ti::Key::F(n) => (KeyCode::F(n), Modifiers::NONE),
                _ => (KeyCode::Other, Modifiers::NONE),
            };
            Self { code, modifiers }
        }
    }

    impl From<ti::MouseButton> for MouseButton {
        fn from(button: ti::MouseButton) -> Self {
            match button {
                ti::MouseButton::Left => MouseButton::Left,
                ti::MouseButton::Right => MouseButton::Right,
                ti::MouseButton::Middle => MouseButton::Middle,
                ti::MouseButton::WheelUp
                | ti::MouseButton::WheelDown
                | ti::MouseButton::WheelLeft
                | ti::MouseButton::WheelRight => MouseButton::Unknown,
            }
        }
    }

    impl From<ti::MouseEvent> for MouseEvent {
        fn from(event: ti::MouseEvent) -> Self {
            match event {
                ti::MouseEvent::Press(btn, x, y) => {
                    // Termion uses 1-based coordinates
                    let column = x.saturating_sub(1);
                    let row = y.saturating_sub(1);

                    match btn {
                        ti::MouseButton::WheelUp => {
                            Self::new(MouseEventKind::ScrollUp, column, row)
                        }
                        ti::MouseButton::WheelDown => {
                            Self::new(MouseEventKind::ScrollDown, column, row)
                        }
                        _ => Self::new(MouseEventKind::Down(btn.into()), column, row),
                    }
                }
                ti::MouseEvent::Release(x, y) => {
                    let column = x.saturating_sub(1);
                    let row = y.saturating_sub(1);
                    // Termion doesn't tell us which button was released
                    Self::new(MouseEventKind::Up(MouseButton::Left), column, row)
                }
                ti::MouseEvent::Hold(x, y) => {
                    let column = x.saturating_sub(1);
                    let row = y.saturating_sub(1);
                    // Termion doesn't tell us which button is held
                    Self::new(MouseEventKind::Drag(MouseButton::Left), column, row)
                }
            }
        }
    }
}

// =============================================================================
// Termwiz backend conversions
// =============================================================================

#[cfg(feature = "termwiz")]
mod termwiz_impl {
    use super::*;
    use ratatui::termwiz::input as tw;

    impl From<tw::KeyCode> for KeyCode {
        fn from(code: tw::KeyCode) -> Self {
            match code {
                tw::KeyCode::UpArrow => KeyCode::Up,
                tw::KeyCode::DownArrow => KeyCode::Down,
                tw::KeyCode::LeftArrow => KeyCode::Left,
                tw::KeyCode::RightArrow => KeyCode::Right,
                tw::KeyCode::Char(c) => KeyCode::Char(c),
                tw::KeyCode::Delete => KeyCode::Delete,
                tw::KeyCode::Backspace => KeyCode::Backspace,
                tw::KeyCode::Escape => KeyCode::Esc,
                tw::KeyCode::Enter => KeyCode::Enter,
                tw::KeyCode::Tab => KeyCode::Tab,
                tw::KeyCode::Home => KeyCode::Home,
                tw::KeyCode::End => KeyCode::End,
                tw::KeyCode::PageUp => KeyCode::PageUp,
                tw::KeyCode::PageDown => KeyCode::PageDown,
                tw::KeyCode::Function(n) => KeyCode::F(n),
                _ => KeyCode::Other,
            }
        }
    }

    impl From<tw::Modifiers> for Modifiers {
        fn from(mods: tw::Modifiers) -> Self {
            Self {
                shift: mods.contains(tw::Modifiers::SHIFT),
                ctrl: mods.contains(tw::Modifiers::CTRL),
                alt: mods.contains(tw::Modifiers::ALT),
            }
        }
    }

    impl From<tw::KeyEvent> for KeyEvent {
        fn from(event: tw::KeyEvent) -> Self {
            Self {
                code: event.key.into(),
                modifiers: event.modifiers.into(),
            }
        }
    }

    impl From<tw::MouseButtons> for MouseButton {
        fn from(buttons: tw::MouseButtons) -> Self {
            if buttons.contains(tw::MouseButtons::LEFT) {
                MouseButton::Left
            } else if buttons.contains(tw::MouseButtons::RIGHT) {
                MouseButton::Right
            } else if buttons.contains(tw::MouseButtons::MIDDLE) {
                MouseButton::Middle
            } else {
                MouseButton::Unknown
            }
        }
    }

    /// Convert termwiz MouseEvent to our MouseEvent.
    ///
    /// **Note:** Termwiz's `MouseEvent` is a state snapshot (position + current buttons),
    /// not a discrete event. This conversion infers the event kind:
    /// - If buttons are held: `Down` (on first conversion) or `Drag` (if tracking state)
    /// - If no buttons: `Moved`
    ///
    /// For proper press/release detection, track button state changes in your app.
    impl From<tw::MouseEvent> for MouseEvent {
        fn from(event: tw::MouseEvent) -> Self {
            let column = event.x;
            let row = event.y;
            let modifiers = event.modifiers.into();

            // Termwiz doesn't distinguish press/release/drag in MouseEvent.
            // We infer based on button state.
            let kind = if event.mouse_buttons.is_empty() {
                MouseEventKind::Moved
            } else {
                // Buttons are held - could be Down or Drag depending on prior state
                // Default to Down; user can track state for proper Drag detection
                MouseEventKind::Down(event.mouse_buttons.into())
            };

            Self {
                kind,
                column,
                row,
                modifiers,
            }
        }
    }
}

// Helper for termwiz users to track mouse state and generate proper events
#[cfg(feature = "termwiz")]
pub mod termwiz_helpers {
    use super::*;
    use ratatui::termwiz::input as tw;

    /// Tracks termwiz mouse state to generate proper press/release/drag events.
    ///
    /// Termwiz's `MouseEvent` only provides state snapshots (position + current buttons).
    /// This helper compares consecutive events to detect state changes.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // `termwiz` is not a direct dependency, so this snippet is not compiled here.
    /// let mut tracker = MouseStateTracker::new();
    ///
    /// // In your event loop:
    /// if let InputEvent::Mouse(tw_mouse) = event {
    ///     if let Some(flow_event) = tracker.process(tw_mouse) {
    ///         flow.handle_mouse_event(flow_event);
    ///     }
    /// }
    /// ```
    #[derive(Debug, Default)]
    pub struct MouseStateTracker {
        prev_buttons: tw::MouseButtons,
        prev_x: u16,
        prev_y: u16,
    }

    impl MouseStateTracker {
        /// Creates a new mouse state tracker.
        pub fn new() -> Self {
            Self::default()
        }

        /// Processes a termwiz MouseEvent and returns a flow MouseEvent if meaningful.
        ///
        /// Returns `Some(MouseEvent)` for press, release, drag, or any movement.
        /// Returns `None` if no change occurred.
        pub fn process(&mut self, event: tw::MouseEvent) -> Option<MouseEvent> {
            let column = event.x;
            let row = event.y;
            let modifiers = event.modifiers.into();
            let buttons = event.mouse_buttons.clone();

            // Detect button state changes
            let was_pressed = !self.prev_buttons.is_empty();
            let is_pressed = !buttons.is_empty();
            let moved = column != self.prev_x || row != self.prev_y;

            let kind = if !was_pressed && is_pressed {
                // Button(s) just pressed
                Some(MouseEventKind::Down(buttons.clone().into()))
            } else if was_pressed && !is_pressed {
                // Button(s) just released
                Some(MouseEventKind::Up(self.prev_buttons.clone().into()))
            } else if is_pressed && moved {
                // Dragging with button held
                Some(MouseEventKind::Drag(buttons.clone().into()))
            } else if !is_pressed && moved {
                // Mouse moved without buttons
                Some(MouseEventKind::Moved)
            } else {
                // No meaningful change
                None
            };

            // Update state
            self.prev_buttons = buttons;
            self.prev_x = column;
            self.prev_y = row;

            kind.map(|k| MouseEvent {
                kind: k,
                column,
                row,
                modifiers,
            })
        }
    }
}

// =============================================================================
// Ratzilla backend conversions (WebAssembly)
// =============================================================================

#[cfg(all(feature = "ratzilla", target_arch = "wasm32"))]
mod ratzilla_impl {
    use super::*;

    impl From<ratzilla::event::KeyCode> for KeyCode {
        fn from(code: ratzilla::event::KeyCode) -> Self {
            match code {
                ratzilla::event::KeyCode::Up => KeyCode::Up,
                ratzilla::event::KeyCode::Down => KeyCode::Down,
                ratzilla::event::KeyCode::Left => KeyCode::Left,
                ratzilla::event::KeyCode::Right => KeyCode::Right,
                ratzilla::event::KeyCode::Char(c) => KeyCode::Char(c),
                ratzilla::event::KeyCode::Delete => KeyCode::Delete,
                ratzilla::event::KeyCode::Backspace => KeyCode::Backspace,
                ratzilla::event::KeyCode::Esc => KeyCode::Esc,
                ratzilla::event::KeyCode::Enter => KeyCode::Enter,
                ratzilla::event::KeyCode::Tab => KeyCode::Tab,
                ratzilla::event::KeyCode::Home => KeyCode::Home,
                ratzilla::event::KeyCode::End => KeyCode::End,
                ratzilla::event::KeyCode::PageUp => KeyCode::PageUp,
                ratzilla::event::KeyCode::PageDown => KeyCode::PageDown,
                ratzilla::event::KeyCode::F(n) => KeyCode::F(n),
                ratzilla::event::KeyCode::Unidentified => KeyCode::Other,
            }
        }
    }

    impl From<ratzilla::event::KeyEvent> for KeyEvent {
        fn from(event: ratzilla::event::KeyEvent) -> Self {
            Self {
                code: event.code.into(),
                modifiers: Modifiers {
                    shift: event.shift,
                    ctrl: event.ctrl,
                    alt: event.alt,
                },
            }
        }
    }

    impl From<ratzilla::event::MouseButton> for MouseButton {
        fn from(button: ratzilla::event::MouseButton) -> Self {
            match button {
                ratzilla::event::MouseButton::Left => MouseButton::Left,
                ratzilla::event::MouseButton::Right => MouseButton::Right,
                ratzilla::event::MouseButton::Middle => MouseButton::Middle,
                ratzilla::event::MouseButton::Back
                | ratzilla::event::MouseButton::Forward
                | ratzilla::event::MouseButton::Unidentified => MouseButton::Unknown,
            }
        }
    }

    impl From<ratzilla::event::MouseEvent> for MouseEvent {
        fn from(event: ratzilla::event::MouseEvent) -> Self {
            use ratzilla::event::MouseEventKind as Rz;
            // ratzilla reports button-less moves and carries the button inside the
            // press/release variants; it has no drag or wheel. Drag is synthesized
            // by the app from button state, and wheel arrives via
            // [`Flow::handle_wheel`]. Click/enter/exit have no flow equivalent, so
            // they read as plain moves.
            let kind = match event.kind {
                Rz::ButtonDown(b) => MouseEventKind::Down(b.into()),
                Rz::ButtonUp(b) => MouseEventKind::Up(b.into()),
                Rz::Moved
                | Rz::SingleClick(_)
                | Rz::DoubleClick(_)
                | Rz::Entered
                | Rz::Exited
                | Rz::Unidentified => MouseEventKind::Moved,
            };

            Self {
                kind,
                column: event.col,
                row: event.row,
                modifiers: Modifiers {
                    shift: event.shift,
                    ctrl: event.ctrl,
                    alt: event.alt,
                },
            }
        }
    }
}
