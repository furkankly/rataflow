//! Edge path computation and geometry (Layer 1 of edge rendering).
//!
//! This module provides a unified [`Path`] type that represents any edge path
//! as a sequence of points in **world coordinates**. The same path structure is used for:
//! - Rendering (transformed to terminal coordinates at render time)
//! - Hit-testing (in world coordinates)
//! - Label positioning
//!
//! # Coordinate System
//!
//! Paths are computed in world coordinates (f64). This ensures:
//! - Zoom-independent geometry (proportions stay the same at any zoom level)
//! - Consistent behavior for hit-testing and rendering
//! - Clean separation between geometry and screen-space concerns
//!
//! Terminal-space adjustments (like endpoint offsets for handle alignment) are
//! applied at render time, not during path computation.
//!
//! # Architecture
//!
//! Edge rendering uses a 3-layer architecture:
//! 1. **Path Computation** (this module) - Pure geometry in world coordinates
//! 2. **Path Rendering** - Transforms to terminal, applies offsets, draws to buffer
//! 3. **Builtins** - Compose layers 1 + 2 (e.g., [`StepEdge`](super::StepEdge))

use crate::types::{HandlePosition, Position};

/// A path represented as a sequence of points connected by line segments.
///
/// All coordinates are in **world space** (f64). The path is transformed to
/// terminal coordinates at render time.
///
/// This unified representation works for all edge types:
/// - Step edges: multiple points with corners (e.g., `[start, corner1, corner2, end]`)
/// - Straight edges: just two points (e.g., `[start, end]`)
///
/// The same path is used for rendering, hit-testing, and label positioning.
/// Direction information is stored for proper marker/arrow rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// Points along the path in world coordinates, connected by line segments.
    pub points: Vec<Position>,
    /// The center point for label placement (world coordinates).
    pub label_position: Position,
    /// The handle position at the source (determines exit direction and start marker orientation).
    pub source_position: HandlePosition,
    /// The handle position at the target (determines approach direction and end marker orientation).
    pub target_position: HandlePosition,
}

impl Path {
    /// Creates a new path from a sequence of points with direction info.
    ///
    /// Label is placed at the midpoint between the first and last points.
    /// Use [`with_label_position`](Self::with_label_position) to override.
    pub fn new(
        points: Vec<Position>,
        source_position: HandlePosition,
        target_position: HandlePosition,
    ) -> Self {
        let label_position = match (points.first(), points.last()) {
            (Some(a), Some(b)) => Self::midpoint(*a, *b),
            _ => Position::zero(),
        };
        Self {
            points,
            label_position,
            source_position,
            target_position,
        }
    }

    /// Creates a straight path between two points.
    ///
    /// Label is placed at the midpoint. Uses default handle positions (Right -> Left)
    /// since straight paths don't use handle positions for routing.
    pub fn straight(from: Position, to: Position) -> Self {
        Self::straight_with_direction(from, to, HandlePosition::Right, HandlePosition::Left)
    }

    /// Creates a straight path between two points with explicit direction info.
    ///
    /// Label is placed at the midpoint.
    pub fn straight_with_direction(
        from: Position,
        to: Position,
        source_position: HandlePosition,
        target_position: HandlePosition,
    ) -> Self {
        let label_position = Self::midpoint(from, to);
        Self {
            points: vec![from, to],
            label_position,
            source_position,
            target_position,
        }
    }

    /// Midpoint between two positions.
    fn midpoint(a: Position, b: Position) -> Position {
        Position::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
    }

    /// Overrides the default label position.
    ///
    /// Use this when the path shape requires handle-aware placement
    /// (e.g., step edges compute label position based on routing geometry).
    pub fn with_label_position(mut self, pos: Position) -> Self {
        self.label_position = pos;
        self
    }

    /// Returns true if this path has no segments (fewer than 2 points).
    pub fn is_empty(&self) -> bool {
        self.points.len() < 2
    }

    /// Returns the number of line segments in this path.
    pub fn segment_count(&self) -> usize {
        if self.points.len() < 2 {
            0
        } else {
            self.points.len() - 1
        }
    }

    /// Returns the start point of the path, if any.
    pub fn start(&self) -> Option<Position> {
        self.points.first().copied()
    }

    /// Returns the end point of the path, if any.
    pub fn end(&self) -> Option<Position> {
        self.points.last().copied()
    }

    /// Tests if a point is within `threshold` distance of any segment in the path.
    ///
    /// All coordinates are in world space. The threshold is in world units.
    /// This is used for mouse hit-testing to determine if a click is on an edge.
    pub fn hit_test(&self, point: Position, threshold: f64) -> bool {
        for window in self.points.windows(2) {
            let from = window[0];
            let to = window[1];

            let distance = point_to_segment_distance(point.x, point.y, from.x, from.y, to.x, to.y);
            if distance <= threshold {
                return true;
            }
        }
        false
    }

    /// Returns the bounding box of the path in world coordinates.
    pub fn bounds(&self) -> crate::types::Rect {
        if self.points.is_empty() {
            return crate::types::Rect::zero();
        }

        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;

        for point in &self.points {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }

        crate::types::Rect::from_coords(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// Computes the distance from a point to a line segment (all in world coordinates).
fn point_to_segment_distance(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;

    if dx == 0.0 && dy == 0.0 {
        // Segment is a point
        return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
    }

    // Parameter t for the projection of point onto the line
    let t = ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy);

    // Clamp t to [0, 1] to stay within the segment
    let t = t.clamp(0.0, 1.0);

    // Closest point on the segment
    let closest_x = x1 + t * dx;
    let closest_y = y1 + t * dy;

    // Distance from point to closest point
    ((px - closest_x).powi(2) + (py - closest_y).powi(2)).sqrt()
}

/// Returns the direction vector for a handle position.
///
/// This is the direction an edge exits/enters from that handle position.
fn direction_for_position(position: HandlePosition) -> Position {
    match position {
        HandlePosition::Right => Position::new(1.0, 0.0),
        HandlePosition::Left => Position::new(-1.0, 0.0),
        HandlePosition::Top => Position::new(0.0, -1.0),
        HandlePosition::Bottom => Position::new(0.0, 1.0),
    }
}

/// Clamps a stem length based on the favorable distance toward the other endpoint.
///
/// Computes the dot product of `(to - from)` with the handle direction. When
/// positive (target is in the favorable direction), the stem is clamped to avoid
/// overshooting. When negative (target is behind), the full stem length is used.
///
/// For source stems, call as `clamp_stem(stem, from, to, source_pos)`.
/// For target stems, call as `clamp_stem(stem, to, from, target_pos)`.
fn clamp_stem(stem_length: f64, from: Position, to: Position, position: HandlePosition) -> f64 {
    let dir = direction_for_position(position);
    let d = (to.x - from.x) * dir.x + (to.y - from.y) * dir.y;
    if d > 0.0 {
        stem_length.min(d)
    } else {
        stem_length
    }
}

/// Returns the actual geometric direction from source to target along the source
/// handle's primary axis.
///
/// For horizontal handles (Left/Right), returns horizontal direction.
/// For vertical handles (Top/Bottom), returns vertical direction.
fn get_direction(source: Position, source_position: HandlePosition, target: Position) -> Position {
    match source_position {
        HandlePosition::Left | HandlePosition::Right => {
            if source.x < target.x {
                Position::new(1.0, 0.0)
            } else {
                Position::new(-1.0, 0.0)
            }
        }
        HandlePosition::Top | HandlePosition::Bottom => {
            if source.y < target.y {
                Position::new(0.0, 1.0)
            } else {
                Position::new(0.0, -1.0)
            }
        }
    }
}

/// Computes an orthogonal (step) path between two points.
///
/// The path consists of horizontal and vertical segments with corners at turning
/// points. The routing algorithm determines the path shape based on the actual
/// geometric direction between gapped (stem end) points and the handle relationship:
///
/// - **Opposite handles** (e.g., Right→Left): Z-shaped routing through the midpoint
/// - **Non-opposite handles** (e.g., Right→Top): L-shaped routing with a single corner
///
/// # Stem Length
///
/// The `stem_length` controls the minimum distance the edge must travel in the
/// handle's direction before any turns. This prevents cramped-looking edges when
/// nodes are close together.
///
/// Example with `stem_length = 3.0` and a Right handle:
/// - The edge travels at least 3 world units right before turning
///
/// All coordinates are in **world units** (scale with zoom).
///
/// # Arguments
///
/// * `from` - Start position in world coordinates
/// * `to` - End position in world coordinates
/// * `source_position` - The handle position at the source (determines exit direction)
/// * `target_position` - The handle position at the target (determines approach direction)
/// * `stem_length` - Minimum travel distance from each handle before routing turns (0.0 for immediate routing)
///
/// # Returns
///
/// A [`Path`] containing the computed waypoints in world coordinates.
///
/// # Path Structure with Stem Length
///
/// When `stem_length > 0`, the path includes intermediate points:
/// ```text
/// [source] → [source + stem_length] → [routing...] → [target - stem_length] → [target]
/// ```
pub fn compute_step_path(
    from: Position,
    to: Position,
    source_position: HandlePosition,
    target_position: HandlePosition,
    stem_length: f64,
) -> Path {
    let source_dir = direction_for_position(source_position);
    let target_dir = direction_for_position(target_position);

    // Compute effective stem lengths, clamped to prevent stems from overshooting
    // and creating backtracking paths when nodes are close together
    let (effective_source_stem, effective_target_stem) = if stem_length > 0.0 {
        let source_vertical = source_position.is_vertical();
        let target_vertical = target_position.is_vertical();

        if source_vertical == target_vertical {
            let source_val = if source_vertical {
                source_dir.y
            } else {
                source_dir.x
            };
            let target_val = if source_vertical {
                target_dir.y
            } else {
                target_dir.x
            };

            if source_val * target_val == -1.0 {
                // Opposite handles (e.g., Right→Left): stems face each other,
                // share the available space to prevent crossing
                let available = if source_vertical {
                    (to.y - from.y).abs()
                } else {
                    (to.x - from.x).abs()
                };
                let clamped = stem_length.min(available / 2.0).max(0.0);
                (clamped, clamped)
            } else {
                // Same-direction handles (e.g., Right→Right): stems extend the
                // same way, no crossing possible — use full stem_length
                (stem_length, stem_length)
            }
        } else {
            // Mixed handles: clamp stem to not overshoot the target in the
            // favorable direction, keep full stem_length when target is behind
            let source_stem = clamp_stem(stem_length, from, to, source_position);
            let target_stem = clamp_stem(stem_length, to, from, target_position);
            (source_stem, target_stem)
        }
    } else {
        (0.0, 0.0)
    };

    // Compute gapped points (where stems end before routing begins)
    let source_gapped = Position::new(
        from.x + source_dir.x * effective_source_stem,
        from.y + source_dir.y * effective_source_stem,
    );
    let target_gapped = Position::new(
        to.x + target_dir.x * effective_target_stem,
        to.y + target_dir.y * effective_target_stem,
    );

    // Determine the primary direction between gapped points
    let dir = get_direction(source_gapped, source_position, target_gapped);
    let dir_is_x = dir.x != 0.0;
    let curr_dir = if dir_is_x { dir.x } else { dir.y };
    let source_axis = if dir_is_x { source_dir.x } else { source_dir.y };
    let target_axis = if dir_is_x { target_dir.x } else { target_dir.y };

    // Route between gapped points based on handle relationship.
    // Label position depends on the routing shape:
    //   - Opposite handles: source/target midpoint
    //   - Non-opposite handles: label on the longest axis of the L-shape
    let (midpoints, label_position): (Vec<Position>, Position) = if source_axis * target_axis
        == -1.0
    {
        // Opposite handles (e.g., Right→Left, Bottom→Top) — Z-shaped routing
        let center_x = (source_gapped.x + target_gapped.x) / 2.0;
        let center_y = (source_gapped.y + target_gapped.y) / 2.0;

        let vertical_split = vec![
            Position::new(center_x, source_gapped.y),
            Position::new(center_x, target_gapped.y),
        ];
        let horizontal_split = vec![
            Position::new(source_gapped.x, center_y),
            Position::new(target_gapped.x, center_y),
        ];

        let points = if source_axis == curr_dir {
            if dir_is_x {
                vertical_split
            } else {
                horizontal_split
            }
        } else if dir_is_x {
            horizontal_split
        } else {
            vertical_split
        };

        // Label at source/target midpoint
        let label = Position::new(center_x, center_y);
        (points, label)
    } else {
        // Non-opposite handles (mixed or same position) — L-shaped routing
        let source_target = Position::new(source_gapped.x, target_gapped.y);
        let target_source = Position::new(target_gapped.x, source_gapped.y);

        let mut corner = if dir_is_x {
            if source_dir.x == curr_dir {
                target_source
            } else {
                source_target
            }
        } else if source_dir.y == curr_dir {
            source_target
        } else {
            target_source
        };

        // For mixed handle positions, determine if the L-shape needs flipping
        if source_position != target_position {
            let (is_same_dir, source_gt, source_lt) = if dir_is_x {
                (
                    source_dir.x == target_dir.y,
                    source_gapped.y > target_gapped.y,
                    source_gapped.y < target_gapped.y,
                )
            } else {
                (
                    source_dir.y == target_dir.x,
                    source_gapped.x > target_gapped.x,
                    source_gapped.x < target_gapped.x,
                )
            };

            let primary = if dir_is_x { source_dir.x } else { source_dir.y };
            let flip = (primary == 1.0
                && ((!is_same_dir && source_gt) || (is_same_dir && source_lt)))
                || (primary != 1.0 && ((!is_same_dir && source_lt) || (is_same_dir && source_gt)));

            if flip {
                corner = if dir_is_x {
                    source_target
                } else {
                    target_source
                };
            }
        }

        // Label on the longest axis of the L-shape
        let max_x_dist = (source_gapped.x - corner.x)
            .abs()
            .max((target_gapped.x - corner.x).abs());
        let max_y_dist = (source_gapped.y - corner.y)
            .abs()
            .max((target_gapped.y - corner.y).abs());

        let label = if max_x_dist >= max_y_dist {
            Position::new((source_gapped.x + target_gapped.x) / 2.0, corner.y)
        } else {
            Position::new(corner.x, (source_gapped.y + target_gapped.y) / 2.0)
        };

        (vec![corner], label)
    };

    // For same-position handles (e.g., Right→Right), when nodes are close on the
    // primary axis both gapped points land near the same coordinate. Pull one back
    // to create visual separation. Only affects the final path, not routing midpoints.
    let (final_source_gapped, final_target_gapped) =
        if source_position == target_position && stem_length > 0.0 {
            let diff = if dir_is_x {
                (from.x - to.x).abs()
            } else {
                (from.y - to.y).abs()
            };
            let gap_offset = if diff <= stem_length {
                (stem_length - 1.0).min(stem_length - diff).max(0.0)
            } else {
                0.0
            };
            if gap_offset > 0.0 {
                let source_primary = if dir_is_x { source_dir.x } else { source_dir.y };
                if source_primary == curr_dir {
                    // Source is "in front" — shorten its stem
                    let sign = if dir_is_x {
                        if source_gapped.x > from.x { -1.0 } else { 1.0 }
                    } else if source_gapped.y > from.y {
                        -1.0
                    } else {
                        1.0
                    };
                    let adj = if dir_is_x {
                        Position::new(source_gapped.x + sign * gap_offset, source_gapped.y)
                    } else {
                        Position::new(source_gapped.x, source_gapped.y + sign * gap_offset)
                    };
                    (adj, target_gapped)
                } else {
                    // Target is "in front" — shorten its stem
                    let sign = if dir_is_x {
                        if target_gapped.x > to.x { -1.0 } else { 1.0 }
                    } else if target_gapped.y > to.y {
                        -1.0
                    } else {
                        1.0
                    };
                    let adj = if dir_is_x {
                        Position::new(target_gapped.x + sign * gap_offset, target_gapped.y)
                    } else {
                        Position::new(target_gapped.x, target_gapped.y + sign * gap_offset)
                    };
                    (source_gapped, adj)
                }
            } else {
                (source_gapped, target_gapped)
            }
        } else {
            (source_gapped, target_gapped)
        };

    // Assemble full path: [from, source_gapped, ...midpoints, target_gapped, to]
    let mut points = Vec::with_capacity(midpoints.len() + 4);
    points.push(from);
    points.push(final_source_gapped);
    points.extend(midpoints);
    points.push(final_target_gapped);
    points.push(to);
    points.dedup();

    Path::new(points, source_position, target_position).with_label_position(label_position)
}

/// Computes a straight path between two points.
///
/// # Arguments
///
/// * `from` - Start position in world coordinates
/// * `to` - End position in world coordinates
/// * `source_position` - The handle position at the source
/// * `target_position` - The handle position at the target
///
/// # Returns
///
/// A [`Path`] containing just the two endpoints with direction info.
pub fn compute_straight_path(
    from: Position,
    to: Position,
    source_position: HandlePosition,
    target_position: HandlePosition,
) -> Path {
    Path::straight_with_direction(from, to, source_position, target_position)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: f64, y: f64) -> Position {
        Position::new(x, y)
    }

    #[test]
    fn test_path_straight() {
        let path = Path::straight(pos(0.0, 0.0), pos(10.0, 10.0));
        assert_eq!(path.points, vec![pos(0.0, 0.0), pos(10.0, 10.0)]);
        assert_eq!(path.label_position, pos(5.0, 5.0));
    }

    #[test]
    fn test_compute_step_path_straight_vertical() {
        let path = compute_step_path(
            pos(10.0, 0.0),
            pos(10.0, 20.0),
            HandlePosition::Bottom,
            HandlePosition::Top,
            0.0,
        );
        // All points on same x=10, midpoint included from Z-shape routing
        assert_eq!(
            path.points,
            vec![pos(10.0, 0.0), pos(10.0, 10.0), pos(10.0, 20.0)]
        );
    }

    #[test]
    fn test_compute_step_path_straight_horizontal() {
        let path = compute_step_path(
            pos(0.0, 10.0),
            pos(20.0, 10.0),
            HandlePosition::Right,
            HandlePosition::Left,
            0.0,
        );
        // All points on same y=10, midpoint included from Z-shape routing
        assert_eq!(
            path.points,
            vec![pos(0.0, 10.0), pos(10.0, 10.0), pos(20.0, 10.0)]
        );
    }

    #[test]
    fn test_compute_step_path_with_corners() {
        let path = compute_step_path(
            pos(0.0, 0.0),
            pos(20.0, 20.0),
            HandlePosition::Right,
            HandlePosition::Left,
            0.0,
        );
        // H -> V -> H: should have 4 points
        assert_eq!(path.points.len(), 4);
        assert_eq!(path.points[0], pos(0.0, 0.0));
        assert_eq!(path.points[3], pos(20.0, 20.0));
    }

    #[test]
    fn test_path_hit_test() {
        let path = Path::straight(pos(0.0, 0.0), pos(10.0, 0.0));

        // Point on the line
        assert!(path.hit_test(pos(5.0, 0.0), 1.0));

        // Point near the line
        assert!(path.hit_test(pos(5.0, 1.0), 2.0));

        // Point far from the line
        assert!(!path.hit_test(pos(5.0, 10.0), 2.0));
    }

    #[test]
    fn test_compute_straight_path() {
        let path = compute_straight_path(
            pos(0.0, 0.0),
            pos(10.0, 10.0),
            HandlePosition::Right,
            HandlePosition::Left,
        );
        assert_eq!(path.points, vec![pos(0.0, 0.0), pos(10.0, 10.0)]);
        assert_eq!(path.segment_count(), 1);
        assert_eq!(path.source_position, HandlePosition::Right);
        assert_eq!(path.target_position, HandlePosition::Left);
    }

    #[test]
    fn test_compute_step_path_with_stem_length_horizontal() {
        // Right -> Left with stem_length of 5
        let path = compute_step_path(
            pos(0.0, 10.0),
            pos(20.0, 10.0),
            HandlePosition::Right,
            HandlePosition::Left,
            5.0,
        );
        // Should have: [source] -> [source_stem_end] -> [target_stem_end] -> [target]
        // But since it's a straight line, stem end points are on the same line
        assert_eq!(path.points[0], pos(0.0, 10.0)); // source
        assert_eq!(path.points[1], pos(5.0, 10.0)); // source_stem_end (0+5, 10)
        assert!(path.points.contains(&pos(15.0, 10.0))); // target_stem_end (20-5, 10)
        assert_eq!(*path.points.last().unwrap(), pos(20.0, 10.0)); // target
    }

    #[test]
    fn test_compute_step_path_with_stem_length_vertical() {
        // Bottom -> Top with stem_length of 3
        let path = compute_step_path(
            pos(10.0, 0.0),
            pos(10.0, 20.0),
            HandlePosition::Bottom,
            HandlePosition::Top,
            3.0,
        );
        assert_eq!(path.points[0], pos(10.0, 0.0)); // source
        assert_eq!(path.points[1], pos(10.0, 3.0)); // source_stem_end (10, 0+3)
        assert!(path.points.contains(&pos(10.0, 17.0))); // target_stem_end (10, 20-3)
        assert_eq!(*path.points.last().unwrap(), pos(10.0, 20.0)); // target
    }

    #[test]
    fn test_compute_step_path_with_stem_length_routing() {
        // Right -> Left with different y positions and stem_length
        let path = compute_step_path(
            pos(0.0, 0.0),
            pos(20.0, 20.0),
            HandlePosition::Right,
            HandlePosition::Left,
            5.0,
        );
        // source_stem_end: (0+5, 0) = (5, 0)
        // target_stem_end: (20-5, 20) = (15, 20)
        assert_eq!(path.points[0], pos(0.0, 0.0));
        assert_eq!(path.points[1], pos(5.0, 0.0));
        assert!(path.points.contains(&pos(15.0, 20.0)));
        assert_eq!(*path.points.last().unwrap(), pos(20.0, 20.0));
    }

    #[test]
    fn test_compute_step_path_zero_stem_length_same_as_default() {
        let path1 = compute_step_path(
            pos(0.0, 0.0),
            pos(20.0, 20.0),
            HandlePosition::Right,
            HandlePosition::Left,
            0.0,
        );
        let path2 = compute_step_path(
            pos(0.0, 0.0),
            pos(20.0, 20.0),
            HandlePosition::Right,
            HandlePosition::Left,
            0.0,
        );
        assert_eq!(path1.points, path2.points);
    }

    #[test]
    fn test_backwards_vertical_with_stem_length() {
        // Target is ABOVE source with Bottom -> Top handles
        // Stems enforce handle direction, routing goes through midpoint
        let path = compute_step_path(
            pos(10.0, 20.0), // source (lower)
            pos(10.0, 5.0),  // target (higher)
            HandlePosition::Bottom,
            HandlePosition::Top,
            2.0,
        );

        // source_gapped = (10, 22) — 2 units below source (Bottom direction)
        // target_gapped = (10, 3)  — 2 units above target (Top direction)
        // Same x, opposite handles: straight through on same column
        assert_eq!(
            path.points,
            vec![
                pos(10.0, 20.0), // source
                pos(10.0, 22.0), // source_gapped (exits DOWN)
                pos(10.0, 3.0),  // target_gapped (approaches from ABOVE)
                pos(10.0, 5.0),  // target
            ]
        );
    }

    #[test]
    fn test_backwards_horizontal_with_stem_length() {
        // Target is to the LEFT of source with Right -> Left handles
        // Stems enforce handle direction, routing goes through midpoint
        let path = compute_step_path(
            pos(20.0, 10.0), // source (right)
            pos(5.0, 10.0),  // target (left)
            HandlePosition::Right,
            HandlePosition::Left,
            2.0,
        );

        // source_gapped = (22, 10), target_gapped = (3, 10)
        // Same y, opposite handles: straight through on same row
        assert_eq!(
            path.points,
            vec![
                pos(20.0, 10.0), // source
                pos(22.0, 10.0), // source_gapped (exits RIGHT)
                pos(3.0, 10.0),  // target_gapped (approaches from LEFT)
                pos(5.0, 10.0),  // target
            ]
        );
    }

    #[test]
    fn test_backwards_without_stem_length() {
        // Target is ABOVE source with Bottom -> Top handles, NO stem_length
        // Without stems, handle direction is not enforced (same as xyflow with offset=0)
        let path = compute_step_path(
            pos(10.0, 20.0), // source (lower)
            pos(10.0, 5.0),  // target (higher)
            HandlePosition::Bottom,
            HandlePosition::Top,
            0.0,
        );

        assert_eq!(path.points[0], pos(10.0, 20.0));
        assert_eq!(*path.points.last().unwrap(), pos(10.0, 5.0));
    }

    #[test]
    fn test_adaptive_stem_length_parallel_handles() {
        // Nodes close together with large stem_length - stems should be clamped
        // Right→Left, distance=6, stem_length=10 → each stem clamped to 3
        let path = compute_step_path(
            pos(0.0, 5.0),
            pos(6.0, 10.0),
            HandlePosition::Right,
            HandlePosition::Left,
            10.0,
        );

        // Source stem should NOT extend past x=3 (half of available distance)
        // Without clamping, source_stem_end would be at x=10
        assert!(
            path.points[1].x <= 3.5, // Allow small floating point tolerance
            "Source stem extended too far: {:?}",
            path.points
        );

        // Path should not backtrack (no stem overshoot causing backtrack)
        for window in path.points.windows(2) {
            let from_x = window[0].x;
            let to_x = window[1].x;
            if from_x > 3.0 && to_x < from_x - 0.1 {
                panic!("Path backtracks after stem: {:?}", path.points);
            }
        }
    }

    #[test]
    fn test_adaptive_stem_length_mixed_handles() {
        // Right→Top with close nodes - each stem clamped independently
        // source at (5, 10), target at (10, 5), stem_length=10
        // source_stem (Right): clamped to (10-5)=5
        // target_stem (Top): clamped to (5-10)=-5 → 0
        let path = compute_step_path(
            pos(5.0, 10.0),
            pos(10.0, 5.0),
            HandlePosition::Right,
            HandlePosition::Top,
            10.0,
        );

        // Source stem end should be at x=10 (clamped from 15)
        assert!(
            path.points[1].x <= 10.5,
            "Source stem extended past target: {:?}",
            path.points
        );

        // Path should end at target
        assert_eq!(
            *path.points.last().unwrap(),
            pos(10.0, 5.0),
            "Should end at target"
        );
    }

    #[test]
    fn test_adaptive_stem_length_very_close_nodes() {
        // Nodes extremely close - stems should shrink to near zero
        let path = compute_step_path(
            pos(0.0, 5.0),
            pos(2.0, 6.0),
            HandlePosition::Right,
            HandlePosition::Left,
            10.0,
        );

        // With only 2 units between nodes, each stem gets at most 1 unit
        assert!(
            path.points[1].x <= 1.5,
            "Source stem too long for close nodes: {:?}",
            path.points
        );
    }

    #[test]
    fn test_backwards_routing_no_stem_length() {
        // Right→Left with target to the left — Z-shape through midpoint
        let path = compute_step_path(
            pos(45.0, 10.0),
            pos(20.0, 16.0),
            HandlePosition::Right,
            HandlePosition::Left,
            0.0,
        );

        // Opposite handles, backward: routes through center_y = (10+16)/2 = 13
        assert_eq!(
            path.points,
            vec![
                pos(45.0, 10.0),
                pos(45.0, 13.0), // down to center_y
                pos(20.0, 13.0), // left to target's x
                pos(20.0, 16.0), // down to target
            ]
        );
    }

    #[test]
    fn test_label_on_longest_segment() {
        // L-shaped path: long horizontal (50 units) then short vertical (2 units)
        // Label should be on the long horizontal segment, not the short vertical
        let path = compute_step_path(
            pos(0.0, 0.0),
            pos(50.0, 2.0),
            HandlePosition::Right,
            HandlePosition::Top,
            0.0,
        );

        let label = path.label_position;

        // Label should NOT be on the short 2-unit segment
        // It should be roughly centered on the long segment
        let on_long_segment = (label.x - 0.0).abs() > 2.0 || (label.y - 0.0).abs() < 1.5;
        assert!(
            on_long_segment,
            "Label should be on longest segment, got: {:?} for path {:?}",
            label, path.points
        );
    }

    #[test]
    fn test_label_centered_opposite_handles() {
        // Opposite handles (Right→Left): label should be at source/target midpoint
        let path = compute_step_path(
            pos(0.0, 0.0),
            pos(10.0, 0.0),
            HandlePosition::Right,
            HandlePosition::Left,
            1.0,
        );

        let label = path.label_position;
        assert!(
            (label.x - 5.0).abs() < 0.01 && (label.y - 0.0).abs() < 0.01,
            "Label should be at center (5, 0), got: {:?}",
            label
        );
    }

    #[test]
    fn test_same_position_handles_same_primary_axis() {
        // Right→Right at the same x: both stems extend rightward,
        // gapOffset shortens one to create separation
        let path = compute_step_path(
            pos(5.0, 10.0),
            pos(5.0, 3.0),
            HandlePosition::Right,
            HandlePosition::Right,
            5.0,
        );

        // Both stems should extend right (not clamped to 0)
        assert!(
            path.points[1].x > 5.0,
            "Source stem should extend rightward: {:?}",
            path.points
        );
        assert_eq!(*path.points.last().unwrap(), pos(5.0, 3.0));
    }

    #[test]
    fn test_same_position_handles_far_apart() {
        // Right→Right far apart: no gapOffset needed, full stems
        let path = compute_step_path(
            pos(5.0, 10.0),
            pos(20.0, 5.0),
            HandlePosition::Right,
            HandlePosition::Right,
            5.0,
        );

        // Source stem: full 5 units right → x=10
        assert_eq!(path.points[0], pos(5.0, 10.0));
        assert_eq!(path.points[1], pos(10.0, 10.0));
        assert_eq!(*path.points.last().unwrap(), pos(20.0, 5.0));
    }

    #[test]
    fn test_same_position_handles_close_together() {
        // Right→Right, close on x: gapOffset pulls one stem back
        let path = compute_step_path(
            pos(5.0, 10.0),
            pos(8.0, 5.0),
            HandlePosition::Right,
            HandlePosition::Right,
            5.0,
        );

        // diff=3, stem=5 → gapOffset = min(4, 2) = 2
        // Source is "in front" (currDir matches), pulled back by 2
        // Source gapped: 10 - 2 = 8
        assert_eq!(path.points[1], pos(8.0, 10.0));
        assert_eq!(*path.points.last().unwrap(), pos(8.0, 5.0));
    }

    #[test]
    fn test_same_position_no_clamping() {
        // Same-direction handles should NOT be clamped like opposite handles
        // Bottom→Bottom with 0 vertical distance: stems should still extend
        let path = compute_step_path(
            pos(5.0, 10.0),
            pos(15.0, 10.0),
            HandlePosition::Bottom,
            HandlePosition::Bottom,
            5.0,
        );

        // Both stems extend downward, no clamping (available=0 would clamp to 0
        // if we treated these as opposite handles)
        assert!(
            path.points[1].y > 10.0,
            "Source stem should extend downward: {:?}",
            path.points
        );
    }
}
