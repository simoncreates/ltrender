use std::collections::{HashMap, HashSet};

use ascii_assets::TerminalChar;
use common_stdx::Point;

use crate::draw::{
    terminal_buffer::{
        Drawable,
        drawable::{BasicDraw, MultiPointed},
    },
    update_interval_handler::UpdateIntervalType,
};

#[derive(Debug, Clone)]
pub struct PolygonDrawable {
    pub points: Vec<Point<u16>>,
    pub border_style: TerminalChar,
    pub fill_style: Option<TerminalChar>,
}

// this ones stolen from chatgpt
fn scanline_intersection_x(p0: Point<u16>, p1: Point<u16>, y: i32) -> Option<f32> {
    let y0 = p0.y as i32;
    let y1 = p1.y as i32;
    if y0 == y1 {
        return None;
    }
    let y_min = std::cmp::min(y0, y1);
    let y_max = std::cmp::max(y0, y1);
    if y < y_min || y >= y_max {
        return None;
    }
    let x0 = p0.x as f32;
    let x1 = p1.x as f32;
    let t = (y as f32 - y0 as f32) / (y1 as f32 - y0 as f32);
    Some(x0 + t * (x1 - x0))
}

fn rasterize_border(points: &[Point<u16>]) -> HashMap<u16, Vec<u16>> {
    let mut map: HashMap<u16, Vec<u16>> = HashMap::new();
    if points.is_empty() {
        return map;
    }

    for i in 0..points.len() {
        let mut x0 = points[i].x as i32;
        let mut y0 = points[i].y as i32;
        let x1 = points[(i + 1) % points.len()].x as i32;
        let y1 = points[(i + 1) % points.len()].y as i32;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();

        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx + dy;

        loop {
            let yk = y0 as u16;
            let xk = x0 as u16;
            map.entry(yk).or_default().push(xk);

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    // dedupe & sort per row
    for xs in map.values_mut() {
        xs.sort_unstable();
        xs.dedup();
    }

    map
}

// this ones chatgpt too
fn compute_scanline_intervals(points: &[Point<u16>]) -> IvHashMap {
    let mut intervals: IvHashMap = HashMap::new();
    if points.is_empty() {
        return intervals;
    }

    let mut y_min = u16::MAX as i32;
    let mut y_max = u16::MIN as i32;
    for p in points {
        let py = p.y as i32;
        if py < y_min {
            y_min = py;
        }
        if py > y_max {
            y_max = py;
        }
    }

    for y in y_min..=y_max {
        let mut xs: Vec<f32> = Vec::new();
        for i in 0..points.len() {
            let p0 = points[i];
            let p1 = points[(i + 1) % points.len()];
            if let Some(ix) = scanline_intersection_x(p0, p1, y) {
                xs.push(ix);
            }
        }
        if xs.is_empty() {
            continue;
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut row: Vec<(u16, u16)> = Vec::new();
        for pair in xs.chunks(2) {
            if pair.len() != 2 {
                break;
            }
            let start = pair[0].ceil() as i32;
            let end = pair[1].floor() as i32;
            if end <= start {
                continue;
            }
            row.push((start as u16, end as u16));
        }

        if !row.is_empty() {
            intervals.insert(y as u16, row);
        }
    }

    intervals
}

type IvHashMap = HashMap<u16, Vec<(u16, u16)>>;

/// Compute both border_set (pixel positions) and bounding intervals
/// - border_set: for skipping border pixels during fill
/// - bounding_intervals: intervals including the border
fn compute_border_and_bounding(points: &[Point<u16>]) -> (HashSet<Point<u16>>, IvHashMap) {
    let border_map = rasterize_border(points);

    let mut border_set: HashSet<Point<u16>> = HashSet::new();
    for (&y, xs) in &border_map {
        for &x in xs {
            border_set.insert(Point { x, y });
        }
    }

    let mut fill_map = compute_scanline_intervals(points);

    // collect Y values from both maps
    let mut ys: Vec<u16> = fill_map.keys().cloned().collect();
    for &y in border_map.keys() {
        if !ys.contains(&y) {
            ys.push(y);
        }
    }
    ys.sort_unstable();

    let mut ivs: IvHashMap = HashMap::new();
    for y in ys {
        let mut ints: Vec<(u16, u16)> = Vec::new();

        if let Some(row) = fill_map.remove(&y) {
            ints.extend(row);
        }

        if let Some(bxs) = border_map.get(&y) {
            for &x in bxs {
                ints.push((x, x));
            }
        }

        if !ints.is_empty() {
            ivs.insert(y, ints);
        }
    }

    (border_set, ivs)
}

impl MultiPointed for PolygonDrawable {
    fn points(&self) -> &[Point<u16>] {
        &self.points
    }

    fn set_points(&mut self, points: Vec<Point<u16>>) {
        self.points = points;
    }

    fn set_point(&mut self, idx: usize, p: Point<u16>) {
        if idx < self.points.len() {
            self.points[idx] = p;
        }
    }

    fn get_point(&self, idx: usize) -> Option<Point<u16>> {
        self.points.get(idx).cloned()
    }
}

impl Drawable for PolygonDrawable {
    fn size(
        &self,
        _sprites: &crate::draw::SpriteRegistry,
    ) -> Result<(u16, u16), crate::draw::DrawError> {
        let mut low_x = u16::MAX;
        let mut low_y = u16::MAX;
        let mut high_x = u16::MIN;
        let mut high_y = u16::MIN;
        for p in &self.points {
            if p.x < low_x {
                low_x = p.x
            }
            if p.y < low_y {
                low_y = p.y
            }
            if p.x > high_x {
                high_x = p.x
            }
            if p.y > high_y {
                high_y = p.y
            }
        }
        let size = (high_x - low_x, high_y - low_y);

        Ok(size)
    }
    fn as_multi_pointed_mut(&mut self) -> Option<&mut dyn MultiPointed> {
        Some(self)
    }
    fn as_multi_pointed(&self) -> Option<&dyn MultiPointed> {
        Some(self)
    }

    fn shifted(&self, offset: Point<u16>) -> Box<dyn Drawable> {
        let new_points = self.points.iter().map(|p| *p + offset).collect();
        Box::new(PolygonDrawable {
            points: new_points,
            border_style: self.border_style,
            fill_style: self.fill_style,
        })
    }

    fn draw(
        &mut self,
        _: &crate::draw::SpriteRegistry,
    ) -> Result<Vec<crate::draw::terminal_buffer::drawable::BasicDraw>, crate::draw::DrawError>
    {
        let (border_set, _ivs) = compute_border_and_bounding(&self.points);

        let mut out: Vec<BasicDraw> = Vec::new();

        let border_map = rasterize_border(&self.points);
        for (&y, xs) in &border_map {
            for &x in xs {
                out.push(BasicDraw {
                    pos: Point { x, y },
                    chr: self.border_style,
                });
            }
        }

        let fill_chr = if let Some(chr) = self.fill_style {
            chr
        } else {
            return Ok(out);
        };

        for (y, row_intervals) in compute_scanline_intervals(&self.points) {
            for (start, end) in row_intervals {
                for x in start..=end {
                    let pos = Point { x, y };
                    if border_set.contains(&pos) {
                        continue;
                    }
                    out.push(BasicDraw { pos, chr: fill_chr });
                }
            }
        }

        Ok(out)
    }

    fn bounding_iv(
        &self,
        _: &crate::draw::SpriteRegistry,
    ) -> std::collections::HashMap<u16, Vec<crate::draw::UpdateInterval>> {
        let mut intervals: HashMap<u16, Vec<crate::draw::UpdateInterval>> = HashMap::new();

        if self.points.is_empty() {
            return intervals;
        }

        let (_border_set, merged_bounding) = compute_border_and_bounding(&self.points);

        for (y, row_intervals) in merged_bounding {
            let ivs = row_intervals
                .into_iter()
                .map(|(start, end)| crate::draw::UpdateInterval {
                    interval: (start as usize, end as usize),
                    iv_type: UpdateIntervalType::Optimized,
                })
                .collect();
            intervals.insert(y, ivs);
        }

        intervals
    }
}
