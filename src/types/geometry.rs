//! Core geometry types for positions, dimensions, and bounds.
//!
//! Uses f64 for internal calculations to support external layout algorithms
//! and smooth panning.

use std::ops::{Add, Sub};

/// A 2D position in the world coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl Position {
    /// Creates a new position.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Creates a position at the origin (0, 0).
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Returns the distance to another position.
    pub fn distance_to(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Clamps the position within the given extent.
    pub fn clamp(&self, extent: &CoordinateExtent) -> Self {
        Self {
            x: self.x.clamp(extent.min.x, extent.max.x),
            y: self.y.clamp(extent.min.y, extent.max.y),
        }
    }
}

impl Add for Position {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Position {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl From<(f64, f64)> for Position {
    fn from((x, y): (f64, f64)) -> Self {
        Self { x, y }
    }
}

impl From<(i32, i32)> for Position {
    fn from((x, y): (i32, i32)) -> Self {
        Self {
            x: x as f64,
            y: y as f64,
        }
    }
}

impl From<(u16, u16)> for Position {
    fn from((x, y): (u16, u16)) -> Self {
        Self {
            x: x as f64,
            y: y as f64,
        }
    }
}

/// Dimensions (width and height) of a rectangular area.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dimensions {
    pub width: f64,
    pub height: f64,
}

impl Dimensions {
    /// Creates new dimensions.
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Creates zero dimensions.
    pub const fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

impl From<(f64, f64)> for Dimensions {
    fn from((width, height): (f64, f64)) -> Self {
        Self { width, height }
    }
}

impl From<(u16, u16)> for Dimensions {
    fn from((width, height): (u16, u16)) -> Self {
        Self {
            width: width as f64,
            height: height as f64,
        }
    }
}

/// A rectangle defined by position and dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    pub position: Position,
    pub dimensions: Dimensions,
}

impl Rect {
    /// Creates a new rectangle.
    pub const fn new(position: Position, dimensions: Dimensions) -> Self {
        Self {
            position,
            dimensions,
        }
    }

    /// Creates a rectangle from individual coordinates.
    pub const fn from_coords(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            position: Position { x, y },
            dimensions: Dimensions { width, height },
        }
    }

    /// Creates a zero-sized rectangle at the origin.
    pub const fn zero() -> Self {
        Self {
            position: Position::zero(),
            dimensions: Dimensions::zero(),
        }
    }

    /// Returns the x coordinate.
    pub fn x(&self) -> f64 {
        self.position.x
    }

    /// Returns the y coordinate.
    pub fn y(&self) -> f64 {
        self.position.y
    }

    /// Returns the width.
    pub fn width(&self) -> f64 {
        self.dimensions.width
    }

    /// Returns the height.
    pub fn height(&self) -> f64 {
        self.dimensions.height
    }

    /// Returns the right edge (x + width).
    pub fn right(&self) -> f64 {
        self.position.x + self.dimensions.width
    }

    /// Returns the bottom edge (y + height).
    pub fn bottom(&self) -> f64 {
        self.position.y + self.dimensions.height
    }

    /// Returns the center position.
    pub fn center(&self) -> Position {
        Position {
            x: self.position.x + self.dimensions.width / 2.0,
            y: self.position.y + self.dimensions.height / 2.0,
        }
    }

    /// Checks if this rectangle contains a point.
    pub fn contains_point(&self, point: &Position) -> bool {
        point.x >= self.position.x
            && point.x <= self.right()
            && point.y >= self.position.y
            && point.y <= self.bottom()
    }

    /// Checks if this rectangle fully encloses another.
    ///
    /// Touching edges count as enclosed, matching
    /// [`contains_point`](Self::contains_point). Use
    /// [`intersects`](Self::intersects) for mere overlap.
    pub fn contains(&self, other: &Rect) -> bool {
        other.position.x >= self.position.x
            && other.position.y >= self.position.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    /// Checks if this rectangle intersects with another.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.position.x < other.right()
            && self.right() > other.position.x
            && self.position.y < other.bottom()
            && self.bottom() > other.position.y
    }

    /// Returns the intersection of two rectangles, if any.
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }

        let x = self.position.x.max(other.position.x);
        let y = self.position.y.max(other.position.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        Some(Rect::from_coords(x, y, right - x, bottom - y))
    }

    /// Returns the bounding box that contains both rectangles.
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.position.x.min(other.position.x);
        let y = self.position.y.min(other.position.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());

        Rect::from_coords(x, y, right - x, bottom - y)
    }
}

/// Coordinate extent defining boundaries as [[minX, minY], [maxX, maxY]].
///
/// Used to constrain node positions, viewport panning, etc.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CoordinateExtent {
    pub min: Position,
    pub max: Position,
}

impl CoordinateExtent {
    /// Creates a new coordinate extent.
    pub const fn new(min: Position, max: Position) -> Self {
        Self { min, max }
    }

    /// Creates an extent from individual coordinates.
    pub const fn from_coords(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min: Position { x: min_x, y: min_y },
            max: Position { x: max_x, y: max_y },
        }
    }

    /// Creates an infinite extent (no constraints).
    pub fn infinite() -> Self {
        Self {
            min: Position::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            max: Position::new(f64::INFINITY, f64::INFINITY),
        }
    }

    /// Creates an extent from a rectangle.
    pub fn from_rect(rect: &Rect) -> Self {
        Self {
            min: rect.position,
            max: Position::new(rect.right(), rect.bottom()),
        }
    }

    /// Returns the width of the extent.
    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    /// Returns the height of the extent.
    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    /// Converts to a Rect.
    pub fn to_rect(&self) -> Rect {
        Rect::from_coords(self.min.x, self.min.y, self.width(), self.height())
    }
}

impl Default for CoordinateExtent {
    fn default() -> Self {
        Self::infinite()
    }
}

/// Node origin point, determining how the node position relates to its bounds.
///
/// Values are in the range [0.0, 1.0]:
/// - (0.0, 0.0) = top-left corner
/// - (0.5, 0.5) = center
/// - (1.0, 1.0) = bottom-right corner
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeOrigin {
    pub x: f64,
    pub y: f64,
}

impl NodeOrigin {
    /// Creates a new node origin.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Top-left origin (default).
    pub const TOP_LEFT: Self = Self { x: 0.0, y: 0.0 };

    /// Center origin.
    pub const CENTER: Self = Self { x: 0.5, y: 0.5 };

    /// Bottom-right origin.
    pub const BOTTOM_RIGHT: Self = Self { x: 1.0, y: 1.0 };

    /// Calculates the offset to apply based on node dimensions.
    pub(crate) fn offset(&self, dimensions: &Dimensions) -> Position {
        Position {
            x: -dimensions.width * self.x,
            y: -dimensions.height * self.y,
        }
    }
}

impl Default for NodeOrigin {
    fn default() -> Self {
        Self::TOP_LEFT
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn contains_requires_full_enclosure_where_intersects_does_not() {
        let outer = Rect::from_coords(0.0, 0.0, 100.0, 100.0);
        let inside = Rect::from_coords(10.0, 10.0, 20.0, 20.0);
        let straddling = Rect::from_coords(90.0, 90.0, 20.0, 20.0);

        assert!(outer.contains(&inside));
        assert!(outer.intersects(&straddling));
        assert!(!outer.contains(&straddling));

        // Flush against the edges still counts, matching `contains_point`.
        assert!(outer.contains(&Rect::from_coords(0.0, 0.0, 100.0, 100.0)));
        assert!(!inside.contains(&outer));
    }

    use super::*;

    #[test]
    fn test_rect_contains_point() {
        let rect = Rect::from_coords(10.0, 10.0, 20.0, 20.0);
        assert!(rect.contains_point(&Position::new(15.0, 15.0)));
        assert!(rect.contains_point(&Position::new(10.0, 10.0))); // edge
        assert!(!rect.contains_point(&Position::new(5.0, 15.0)));
    }

    #[test]
    fn test_rect_intersection() {
        let r1 = Rect::from_coords(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::from_coords(5.0, 5.0, 10.0, 10.0);

        let intersection = r1.intersection(&r2).unwrap();
        assert_eq!(intersection.position, Position::new(5.0, 5.0));
        assert_eq!(intersection.dimensions, Dimensions::new(5.0, 5.0));
    }

    #[test]
    fn test_rect_union() {
        let r1 = Rect::from_coords(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::from_coords(5.0, 5.0, 10.0, 10.0);

        let union = r1.union(&r2);
        assert_eq!(union.position, Position::new(0.0, 0.0));
        assert_eq!(union.dimensions, Dimensions::new(15.0, 15.0));
    }

    #[test]
    fn test_node_origin_offset() {
        let dims = Dimensions::new(20.0, 10.0);

        assert_eq!(NodeOrigin::TOP_LEFT.offset(&dims), Position::new(0.0, 0.0));
        assert_eq!(NodeOrigin::CENTER.offset(&dims), Position::new(-10.0, -5.0));
        assert_eq!(
            NodeOrigin::BOTTOM_RIGHT.offset(&dims),
            Position::new(-20.0, -10.0)
        );
    }
}
