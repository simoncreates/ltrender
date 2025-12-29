use std::collections::HashMap;

use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};
use log::info;

use crate::BasicDraw;

/// Collector for drawable terminal operations.
#[derive(Debug, Default)]
pub struct BasicDrawCreator {
    pub draws: HashMap<Point<i32>, TerminalChar>,
}

impl BasicDrawCreator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn new_with_capacity(cap: usize) -> Self {
        Self {
            draws: HashMap::with_capacity(cap),
        }
    }

    /// Draw a single character at `pos`.
    ///
    /// If a character already exists at this position, it is replaced.
    pub fn draw_char(&mut self, pos: impl Into<Point<i32>>, chr: TerminalChar) {
        self.draws.insert(pos.into(), chr);
    }

    /// Draw a straight line from `p1` to `p2` (inclusive) using Bresenham’s algorithm.
    ///
    /// Both endpoints are included. Each point on the line receives `chr`.
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
            self.draws.insert(Point { x: x0, y: y0 }, chr);

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

    /// Compute the axis-aligned bounding box containing all drawn points.
    ///
    /// Returns `(0, 0),(0, 0)` if empty.
    pub fn get_bounding_box(&self) -> Rect<i32> {
        if self.draws.is_empty() {
            return Rect::from_coords(0, 0, 0, 0);
        }

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for p in self.draws.keys() {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }

        Rect::from_coords(min_x, min_y, max_x, max_y)
    }

    /// Merge another creator into this one.
    ///
    /// Incoming characters overwrite existing positions.
    pub fn merge_creator(&mut self, to_merge: BasicDrawCreator) {
        self.draws.extend(to_merge.draws);
    }

    /// Merge another creator, offsetting all positions by `offset`.
    pub fn merge_creator_offset(&mut self, to_merge: BasicDrawCreator, offset: Point<i32>) {
        for (pos, chr) in to_merge.draws {
            self.draws.insert(pos + offset, chr);
        }
    }

    /// Translate all draws so the bounding box top-left aligns with `origin`.
    pub fn align_to_origin(&mut self, origin: Point<i32>) {
        let tl = self.get_bounding_box().p1;
        let offset = origin - tl;

        let mut new_draws = HashMap::with_capacity(self.draws.len());
        for (pos, chr) in std::mem::take(&mut self.draws) {
            new_draws.insert(pos + offset, chr);
        }

        self.draws = new_draws;
    }

    /// Consume the creator and return all draws as a `Vec<BasicDraw>`.
    pub fn dump_draws(&mut self) -> Vec<BasicDraw> {
        std::mem::take(&mut self.draws)
            .into_iter()
            .map(|(pos, chr)| BasicDraw { pos, chr })
            .collect()
    }
}
