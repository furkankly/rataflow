//! Auto-pan during drag operations.
//!
//! When dragging a node or connection toward the canvas edge, the viewport
//! automatically pans in that direction — linear velocity ramp within a fixed
//! edge zone, continuous panning even when the mouse is stationary.
//!
//! The app must call [`Flow::tick_auto_pan`] periodically (typically every
//! frame) to drive the panning — the library doesn't own the event loop.

use std::time::Duration;

use super::Flow;
use super::mouse::DragState;
use crate::actions::{EventResponse, FlowEvent};
use crate::content::{EdgeContent, NodeContent};

/// Distance from canvas edge (in cells) where auto-pan activates.
const EDGE_DISTANCE: f64 = 5.0;

/// Default auto-pan speed in canvas cells per second.
pub(crate) const DEFAULT_AUTO_PAN_SPEED: f64 = 110.0;

/// Computes a normalized velocity (-1.0 to 1.0) for one axis.
///
/// Returns 0 when the cursor is inside the safe zone (more than `distance`
/// from either edge). Ramps linearly from ~0 to ±1 as the cursor moves from
/// the threshold toward the canvas edge:
/// - Positive: cursor near the min edge (left/top), pan to reveal content in that direction
/// - Negative: cursor near the max edge (right/bottom)
fn auto_pan_velocity(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        (value - min).abs().clamp(1.0, min) / min
    } else if value > max {
        -((value - max).abs().clamp(1.0, min) / min)
    } else {
        0.0
    }
}

impl<N: NodeContent, E: EdgeContent> Flow<N, E> {
    /// Advances auto-pan state by the given elapsed time.
    ///
    /// When a drag operation (node move or connection creation) is active and
    /// the cursor is within 5 cells of the canvas edge, the
    /// viewport pans in that direction. The dragged node or connection preview
    /// is adjusted to stay under the cursor.
    ///
    /// Call this in your event loop alongside [`tick_animation`](Self::tick_animation).
    /// Returns events when panning occurs (`ViewportChanged`, `NodeDragged`).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::time::Instant;
    /// # use rataflow::Flow;
    /// # let mut flow: Flow = Flow::new();
    /// let mut last_tick = Instant::now();
    ///
    /// loop {
    ///     let now = Instant::now();
    ///     let elapsed = now - last_tick;
    ///     last_tick = now;
    ///
    ///     flow.tick_auto_pan(elapsed);
    ///     flow.tick_animation(elapsed);
    ///
    ///     // terminal.draw(|f| { /* ... */ })?;
    ///     // event handling ...
    /// #   break;
    /// }
    /// ```
    pub fn tick_auto_pan(&mut self, elapsed: Duration) -> EventResponse {
        // Resizing and box-selecting are dragged toward the canvas edge just like
        // moving a node, so they pan under the same setting.
        let (is_node_drag, is_connection_drag) = match &self.drag_state {
            DragState::MovingNode { drag_started, .. } if *drag_started => (true, false),
            DragState::ResizingNode { .. } | DragState::SelectingBox { .. } => (true, false),
            DragState::CreatingConnection | DragState::ReconnectingEdge { .. } => (false, true),
            _ => return EventResponse::NotHandled,
        };

        if (is_node_drag && !self.auto_pan_on_node_drag)
            || (is_connection_drag && !self.auto_pan_on_connect)
        {
            return EventResponse::NotHandled;
        }

        let canvas_pos = match self.last_mouse_canvas_pos {
            Some(pos) => pos,
            None => return EventResponse::NotHandled,
        };

        let canvas_size = self.render_context.canvas_size();
        if canvas_size.width <= EDGE_DISTANCE * 2.0 || canvas_size.height <= EDGE_DISTANCE * 2.0 {
            return EventResponse::Handled;
        }

        let vx = auto_pan_velocity(
            canvas_pos.x,
            EDGE_DISTANCE,
            canvas_size.width - EDGE_DISTANCE,
        );
        let vy = auto_pan_velocity(
            canvas_pos.y,
            EDGE_DISTANCE,
            canvas_size.height - EDGE_DISTANCE,
        );

        if vx == 0.0 && vy == 0.0 {
            return EventResponse::Handled;
        }

        let dt = elapsed.as_secs_f64();
        let dx = vx * self.auto_pan_speed * dt;
        let dy = vy * self.auto_pan_speed * dt;

        self.viewport.x += dx;
        self.viewport.y += dy;

        // The viewport shifted in canvas space; compensate the dragged element
        // by the equivalent world-space delta so it stays under the cursor.
        let world_dx = dx / self.viewport.zoom;
        let world_dy = dy / self.viewport.zoom;

        let mut events = vec![FlowEvent::ViewportChanged {
            x: self.viewport.x,
            y: self.viewport.y,
            zoom: self.viewport.zoom,
        }];

        if is_node_drag {
            self.compensate_node_drag(world_dx, world_dy, &mut events);
        }

        EventResponse::Event(events)
    }

    /// Adjusts the dragged node's position to compensate for auto-pan movement,
    /// keeping the node under the cursor.
    ///
    /// Only the node position is adjusted — not the drag offset. The offset is
    /// the initial grab delta (`node_pos - mouse_world_pos`), which stays correct
    /// because auto-pan shifts both the node and the cursor's world position by
    /// the same amount. The next `on_mouse_drag` will recompute position from
    /// `mouse_world_pos + offset` and arrive at the right place.
    fn compensate_node_drag(&mut self, world_dx: f64, world_dy: f64, events: &mut Vec<FlowEvent>) {
        if let DragState::MovingNode { ref node_id, .. } = self.drag_state {
            let node_id = node_id.clone();
            if let Some(node) = self.internal_node_mut(&node_id) {
                node.node.position.x -= world_dx;
                node.node.position.y -= world_dy;
            }
            self.drag_hierarchy_pending = true;
            events.push(FlowEvent::NodeDragged { node_id });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::mouse::DragState;
    use crate::types::{Node, Position};
    use crate::ui::TextContent;
    use ratatui::layout::Rect;

    fn make_flow() -> Flow {
        let nodes = vec![
            Node::new(
                "a",
                Position::new(0.0, 0.0),
                (10.0, 5.0),
                TextContent::from("A"),
            ),
            Node::new(
                "b",
                Position::new(30.0, 0.0),
                (10.0, 5.0),
                TextContent::from("B"),
            ),
        ];
        let mut flow = Flow::with_graph(nodes, vec![]).unwrap();
        flow.render_context.canvas_area = Rect::new(0, 0, 80, 24);
        flow
    }

    fn start_node_drag(flow: &mut Flow) {
        flow.drag_state = DragState::MovingNode {
            node_id: "a".into(),
            offset: Position::new(0.0, 0.0),
            parent_absolute: None,
            start_pos: Position::new(0.0, 0.0),
            drag_started: true,
            selected: true,
        };
    }

    #[test]
    fn velocity_linear_ramp() {
        // Inside safe zone: no velocity
        assert_eq!(auto_pan_velocity(EDGE_DISTANCE, EDGE_DISTANCE, 75.0), 0.0);
        assert_eq!(auto_pan_velocity(40.0, EDGE_DISTANCE, 75.0), 0.0);

        // At the canvas edge: max velocity
        assert!((auto_pan_velocity(0.0, EDGE_DISTANCE, 75.0) - 1.0).abs() < f64::EPSILON);
        assert!((auto_pan_velocity(80.0, EDGE_DISTANCE, 75.0) + 1.0).abs() < f64::EPSILON);

        // Closer to edge = faster (monotonic)
        let v_near = auto_pan_velocity(1.0, EDGE_DISTANCE, 75.0);
        let v_mid = auto_pan_velocity(3.0, EDGE_DISTANCE, 75.0);
        assert!(v_near > v_mid);

        // Left/top edge → positive, right/bottom edge → negative
        assert!(auto_pan_velocity(2.0, EDGE_DISTANCE, 75.0) > 0.0);
        assert!(auto_pan_velocity(77.0, EDGE_DISTANCE, 75.0) < 0.0);
    }

    #[test]
    fn pans_toward_nearest_edge() {
        let mut flow = make_flow();
        start_node_drag(&mut flow);

        // Cursor near left edge → viewport pans right (positive x)
        flow.last_mouse_canvas_pos = Some(Position::new(2.0, 12.0));
        let vp_before = flow.viewport.x;
        flow.tick_auto_pan(Duration::from_millis(16));
        assert!(flow.viewport.x > vp_before);

        // Cursor near right edge → viewport pans left (negative x)
        flow.last_mouse_canvas_pos = Some(Position::new(78.0, 12.0));
        let vp_before = flow.viewport.x;
        flow.tick_auto_pan(Duration::from_millis(16));
        assert!(flow.viewport.x < vp_before);
    }

    #[test]
    fn diagonal_panning_in_corner() {
        let mut flow = make_flow();
        start_node_drag(&mut flow);
        let initial = (flow.viewport.x, flow.viewport.y);
        flow.last_mouse_canvas_pos = Some(Position::new(1.0, 1.0));

        flow.tick_auto_pan(Duration::from_millis(16));

        assert!(flow.viewport.x > initial.0);
        assert!(flow.viewport.y > initial.1);
    }

    #[test]
    fn node_position_compensated() {
        let mut flow = make_flow();
        start_node_drag(&mut flow);
        let initial_x = flow.node("a").unwrap().position.x;
        // Cursor near left edge → viewport pans left → world under cursor moves left
        flow.last_mouse_canvas_pos = Some(Position::new(2.0, 12.0));

        flow.tick_auto_pan(Duration::from_millis(16));

        let new_x = flow.node("a").unwrap().position.x;
        assert!(
            new_x < initial_x,
            "node must follow cursor leftward: {new_x} < {initial_x}"
        );
    }

    #[test]
    fn speed_scales_pan_amount() {
        let mut flow = make_flow();
        start_node_drag(&mut flow);
        flow.last_mouse_canvas_pos = Some(Position::new(0.0, 12.0));

        let mut flow_fast = flow.clone();
        flow_fast.auto_pan_speed = DEFAULT_AUTO_PAN_SPEED * 2.0;

        flow.tick_auto_pan(Duration::from_millis(100));
        flow_fast.tick_auto_pan(Duration::from_millis(100));

        let ratio = flow_fast.viewport.x / flow.viewport.x;
        assert!(
            (ratio - 2.0).abs() < f64::EPSILON,
            "2x speed should produce 2x pan: ratio={ratio}"
        );
    }
}
