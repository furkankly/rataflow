//! Animation operations for Flow.

use std::time::Duration;

use super::Flow;
use crate::content::{EdgeContent, NodeContent};
use crate::ui::ANIMATION_PATTERN_LENGTH;

/// Default animation speed in milliseconds per phase step.
pub(crate) const DEFAULT_ANIMATION_SPEED_MS: u64 = 120;

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    /// Advances the animation clock by the given elapsed time.
    ///
    /// Call this in your event loop with the elapsed time since the last tick.
    /// Edges with [`animated: true`](crate::Edge::animated) will use this clock
    /// to produce a marching ants effect. The pattern shifts by one cell every
    /// [`animation_speed_ms`](Self::animation_speed_ms) milliseconds (default: 120).
    ///
    /// The library takes a [`Duration`] rather than reading the clock internally
    /// so it stays portable (`Instant` is unavailable on `wasm32`).
    ///
    /// # Typical usage
    ///
    /// ```no_run
    /// # use std::time::Instant;
    /// # use rataflow::Flow;
    /// # let mut flow: Flow = Flow::new();
    /// let mut last_tick = Instant::now();
    ///
    /// loop {
    ///     let now = Instant::now();
    ///     flow.tick_animation(now - last_tick);
    ///     last_tick = now;
    ///
    ///     // terminal.draw(|f| { /* ... */ })?;
    ///     // event handling ...
    /// #   break;
    /// }
    /// ```
    pub fn tick_animation(&mut self, elapsed: Duration) {
        self.animation_elapsed_ms = self
            .animation_elapsed_ms
            .wrapping_add(elapsed.as_millis() as u64);

        // Reset periodically to avoid unbounded growth.
        // The cycle length is pattern_length * speed_ms; we keep one full cycle.
        let cycle_ms = ANIMATION_PATTERN_LENGTH as u64 * self.animation_speed_ms.max(1);
        if self.animation_elapsed_ms >= cycle_ms {
            self.animation_elapsed_ms %= cycle_ms;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Node, Position};
    use crate::ui::TextContent;

    fn make_test_state() -> Flow {
        let nodes = vec![Node::new(
            "a",
            Position::new(0.0, 0.0),
            (10.0, 5.0),
            TextContent::from("A"),
        )];
        Flow::with_graph(nodes, vec![]).unwrap()
    }

    #[test]
    fn test_tick_advances_time() {
        let mut state = make_test_state();
        assert_eq!(state.animation_elapsed_ms, 0);

        state.tick_animation(Duration::from_millis(50));
        assert_eq!(state.animation_elapsed_ms, 50);

        state.tick_animation(Duration::from_millis(30));
        assert_eq!(state.animation_elapsed_ms, 80);
    }

    #[test]
    fn test_tick_resets_periodically() {
        let mut state = make_test_state();
        let cycle_ms = ANIMATION_PATTERN_LENGTH as u64 * state.animation_speed_ms;

        // Tick past one full cycle
        state.tick_animation(Duration::from_millis(cycle_ms + 50));
        assert_eq!(state.animation_elapsed_ms, 50);
    }
}
