use std::collections::HashMap;

use ascii_assets::{Color, TerminalChar};
use common_stdx::{Point, Rect};

use crate::BasicDraw;

#[derive(Debug, Clone)]
pub struct VerticalBarGraphConfig {
    /// Height of the graph in terminal cells
    pub height: i32,

    /// Width of each bar in characters
    pub bar_width: i32,

    /// Horizontal spacing between bars
    pub spacing: i32,

    /// Character used to fill bars
    pub fill_char: char,

    /// Bar color
    pub color: Color,
}

impl Default for VerticalBarGraphConfig {
    fn default() -> Self {
        Self {
            height: 10,
            bar_width: 1,
            spacing: 1,
            fill_char: '█',
            color: Color::White,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LineGraphConfig {
    /// Height of the graph in terminal cells
    pub height: i32,

    /// Horizontal spacing between sample points
    pub spacing: i32,

    /// Character used to draw sample points
    pub point_char: char,

    /// Character used to draw connecting lines
    pub line_char: char,

    /// Color for sample points
    pub point_color: Color,

    /// Color for connecting lines
    pub line_color: Color,
}

impl Default for LineGraphConfig {
    fn default() -> Self {
        Self {
            height: 10,
            spacing: 3,
            point_char: '•',
            line_char: '─',
            point_color: Color::White,
            line_color: Color::White,
        }
    }
}

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

    /// Draw a horizontal text string starting at `pos`.
    /// Each character advances x by 1. Newlines are interpreted (advances y and resets x to start.x).
    /// If `fg` is `Some(Color)` it will be used
    pub fn draw_text(&mut self, pos: impl Into<Point<i32>>, text: &str, fg: Option<Color>) {
        let start = pos.into();
        let mut x = start.x;
        let mut y = start.y;

        for ch in text.chars() {
            match ch {
                '\n' => {
                    y += 1;
                    x = start.x;
                }
                _ => {
                    let mut t = TerminalChar::from_char(ch);
                    if let Some(col) = fg {
                        t = t.set_fg(col);
                    }
                    self.draws.insert(Point::new(x, y), t);
                    x += 1;
                }
            }
        }
    }

    /// Convert a string into a Vec<BasicDraw>
    pub fn string_to_basic_draws(
        pos: impl Into<Point<i32>>,
        text: &str,
        fg: Option<Color>,
    ) -> Vec<BasicDraw> {
        let mut tmp = BasicDrawCreator::new_with_capacity(text.len());
        tmp.draw_text(pos.into(), text, fg);
        tmp.dump_draws()
    }

    pub fn draw_vertical_bar_graph_simple(
        &mut self,
        start: Point<i32>,
        values: &[usize],
        height: i32,
        color: Color,
    ) -> i32 {
        let cfg = VerticalBarGraphConfig {
            height,
            color,
            ..Default::default()
        };

        self.draw_vertical_bar_graph(start, values, &cfg)
    }

    /// Draw a vertical bar graph of the supplied config
    pub fn draw_vertical_bar_graph(
        &mut self,
        start: Point<i32>,
        values: &[usize],
        cfg: &VerticalBarGraphConfig,
    ) -> i32 {
        if values.is_empty() || cfg.height <= 0 || cfg.bar_width <= 0 {
            return start.y;
        }

        let mut max_val = 1usize;
        for &v in values {
            max_val = max_val.max(v);
        }

        let fill = TerminalChar::from_char(cfg.fill_char).set_fg(cfg.color);

        for (i, &v) in values.iter().enumerate() {
            let x = start.x + (i as i32) * (cfg.bar_width + cfg.spacing);
            let bar_h = ((v as f32 / max_val as f32) * (cfg.height as f32)).round() as i32;

            for bx in 0..cfg.bar_width {
                for dy in 0..bar_h {
                    let pos = Point::new(x + bx, start.y + cfg.height - 1 - dy);
                    self.draws.insert(pos, fill);
                }
            }
        }

        start.y + cfg.height
    }

    /// Draw a line graph using a config struct.
    /// Returns `start.y + height`
    pub fn draw_line_graph(
        &mut self,
        start: Point<i32>,
        values: &[usize],
        cfg: &LineGraphConfig,
    ) -> i32 {
        if values.is_empty() || cfg.height <= 0 || cfg.spacing <= 0 {
            return start.y;
        }

        let mut max_val = 1usize;
        for &v in values {
            max_val = max_val.max(v);
        }

        let point_t = TerminalChar::from_char(cfg.point_char).set_fg(cfg.point_color);
        let line_t = TerminalChar::from_char(cfg.line_char).set_fg(cfg.line_color);

        let mut prev_point: Option<Point<i32>> = None;

        for (i, &v) in values.iter().enumerate() {
            let x = start.x + (i as i32) * cfg.spacing;
            let y_off = ((v as f32 / max_val as f32) * (cfg.height as f32)).round() as i32;
            let y = start.y + cfg.height - 1 - y_off;
            let pt = Point::new(x, y);

            self.draws.insert(pt, point_t);

            if let Some(prev) = prev_point {
                self.draw_line(prev, pt, line_t);
            }

            prev_point = Some(pt);
        }

        start.y + cfg.height
    }
    /// Draw a single cell
    pub fn draw_cell(
        &mut self,
        pos: impl Into<Point<i32>>,
        ch: char,
        fg: Color,
        bg: Option<Color>,
    ) {
        let mut t = TerminalChar::from_char(ch).set_fg(fg);
        if let Some(bgcol) = bg {
            t = t.set_bg(bgcol);
        }
        self.draws.insert(pos.into(), t);
    }

    /// the cell fn should return char/fg/bg
    pub fn draw_grid_from_fn<F>(
        &mut self,
        origin: Point<i32>,
        cols: usize,
        rows: usize,
        stride_x: i32,
        stride_y: i32,
        mut cell_fn: F,
    ) where
        F: FnMut(usize, usize) -> (char, Color, Option<Color>),
    {
        for row in 0..rows {
            let y = origin.y + (row as i32) * stride_y;
            for col in 0..cols {
                let x = origin.x + (col as i32) * stride_x;
                let (ch, fg, bg) = cell_fn(col, row);
                self.draw_cell(Point::new(x, y), ch, fg, bg);

                if stride_x >= 2 {
                    let mut t = TerminalChar::from_char(' ');
                    if let Some(bgcol) = bg {
                        t = t.set_bg(bgcol);
                    }
                    self.draws.insert(Point::new(x + 1, y), t);
                }
            }
        }
    }

    /// Draw a horizontal bar starting at `pos` with `len` characters of `chr`.
    pub fn draw_horizontal_bar(
        &mut self,
        pos: impl Into<Point<i32>>,
        len: i32,
        chr: char,
        fg: Color,
    ) {
        let p = pos.into();
        let t = TerminalChar::from_char(chr).set_fg(fg);
        for dx in 0..len {
            self.draws.insert(Point::new(p.x + dx, p.y), t);
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
