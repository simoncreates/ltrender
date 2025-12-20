//! Utilities for collecting simple drawable terminal operations and computing their bounding box.
//!
//! `BasicDrawCreator` stores a list of `BasicDraw` operations (single characters at integer points)
//! and provides helpers to push single characters or lines (Bresenham) and to compute the bounding
//! rectangle covering all drawn points.

use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};

use crate::BasicDraw;

/// A small collector of drawable terminal operations.
///
/// `BasicDrawCreator` accumulates `BasicDraw` items (position + character). It is intended for
/// building up a batch of terminal draws in memory, then retrieving them with `dump_draws`.
///
/// ## Info
/// The field draws is exposed, so you can manually add them, if necessary
#[derive(Debug, Default)]
pub struct BasicDrawCreator {
    pub draws: Vec<BasicDraw>,
}

impl BasicDrawCreator {
    /// Create a new, empty `BasicDrawCreator`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let c = BasicDrawCreator::new();
    /// ```
    pub fn new() -> Self {
        BasicDrawCreator::default()
    }

    /// Create a new `BasicDrawCreator` with space reserved for `cap` draws.
    ///
    /// Useful when you can predict how many draws you'll push to avoid reallocations.
    pub fn new_with_capacity(cap: usize) -> Self {
        BasicDrawCreator {
            draws: Vec::with_capacity(cap),
        }
    }

    /// Add a single character draw at `pos`.
    ///
    /// `pos` accepts any type convertible into `Point<i32>` via `Into<Point<i32>>`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// creator.draw_char((10, 5), some_terminal_char);
    /// ```
    pub fn draw_char(&mut self, pos: impl Into<Point<i32>>, chr: TerminalChar) {
        self.draws.push(BasicDraw {
            pos: pos.into(),
            chr,
        });
    }

    /// Draw a straight line from `p1` to `p2` (inclusive) using an integer Bresenham algorithm.
    ///
    /// Both endpoints are included. `chr` is copied for each point on the line.
    ///
    /// # Notes
    ///
    /// - This is an integer-only implementation (no anti-aliasing).
    /// - Complexity is O(n) where `n` is the number of points on the line (max of dx or dy).
    /// - `p1` and `p2` may be the same point; that single point will be pushed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// creator.draw_line((0, 0), (4, 2), some_terminal_char);
    /// ```
    pub fn draw_line(
        &mut self,
        p1: impl Into<Point<i32>>,
        p2: impl Into<Point<i32>>,
        chr: TerminalChar,
    ) {
        let p1 = p1.into();
        let p2 = p2.into();

        let mut x0 = p1.x;
        let mut y0 = p1.y;
        let x1 = p2.x;
        let y1 = p2.y;

        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        loop {
            self.draws.push(BasicDraw {
                pos: Point { x: x0, y: y0 },
                chr,
            });

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if e2 < dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Compute the axis-aligned bounding box (minimum rectangle) that contains all drawn points.
    ///
    /// The returned `Rect` is built from the minimum and maximum coordinates found among `draws`.
    /// Behavior for an empty `BasicDrawCreator`:
    /// - Currently returns `Rect::from_coords(0, 0, 0, 0)`.
    ///
    /// # Important
    ///
    /// The semantics of `Rect::from_coords(x1, y1, x2, y2)` are assumed to accept *inclusive*
    /// coordinates for corners. If your `Rect` type expects width/height or half-open coordinates,
    /// adapt this function accordingly.
    pub fn get_bounding_box(&self) -> Option<Rect<i32>> {
        if self.draws.is_empty() {
            return None;
        }

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for d in &self.draws {
            min_x = min_x.min(d.pos.x);
            min_y = min_y.min(d.pos.y);
            max_x = max_x.max(d.pos.x);
            max_y = max_y.max(d.pos.y);
        }

        Some(Rect::from_coords(min_x, min_y, max_x, max_y))
    }

    /// Consume the stored draws and return them.
    ///
    /// This uses `mem::take` to replace the internal vector with an empty one and return the old
    /// contents, avoiding a clone.
    pub fn dump_draws(&mut self) -> Vec<BasicDraw> {
        std::mem::take(&mut self.draws)
    }
}
