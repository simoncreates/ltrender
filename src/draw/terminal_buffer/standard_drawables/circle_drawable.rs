use std::collections::HashMap;

use crate::draw::{
    DrawError, SpriteRegistry, UpdateInterval,
    terminal_buffer::{
        Drawable,
        drawable::{BasicDraw, DoublePointed},
    },
    update_interval_handler::UpdateIntervalType,
};
use ascii_assets::TerminalChar;
use common_stdx::Point;

#[derive(Clone, Debug)]
pub struct CircleDrawable {
    pub center: Point<i32>,
    pub radius: u16,
    pub border_style: TerminalChar,
    pub fill_style: Option<TerminalChar>,
}

impl DoublePointed for CircleDrawable {
    fn start(&self) -> Point<i32> {
        Point {
            x: self.center.x.saturating_sub(self.radius as i32),
            y: self.center.y.saturating_sub(self.radius as i32),
        }
    }

    fn end(&self) -> Point<i32> {
        Point {
            x: self.center.x.saturating_add(self.radius as i32),
            y: self.center.y.saturating_add(self.radius as i32),
        }
    }

    fn set_start(&mut self, p: Point<i32>) {
        let end = self.end();
        self.center = Point {
            x: (p.x + end.x) / 2,
            y: (p.y + end.y) / 2,
        };
        self.radius = ((end.x.saturating_sub(p.x)).max(end.y.saturating_sub(p.y))) as u16 / 2;
    }

    fn set_end(&mut self, p: Point<i32>) {
        let start = self.start();
        self.center = Point {
            x: (p.x + start.x) / 2,
            y: (p.y + start.y) / 2,
        };
        self.radius = ((p.x.saturating_sub(start.x)).max(p.y.saturating_sub(start.y))) as u16 / 2;
    }
}

impl Drawable for CircleDrawable {
    fn size(&self, _sprites: &SpriteRegistry) -> Result<(u16, u16), DrawError> {
        Ok((self.radius * 2, self.radius * 2))
    }
    fn as_double_pointed_mut(&mut self) -> Option<&mut dyn DoublePointed> {
        Some(self)
    }

    fn draw(&mut self, _sprites: &SpriteRegistry) -> Result<Vec<BasicDraw>, DrawError> {
        let mut out = Vec::new();

        let r2 = (self.radius as i32).pow(2);
        let r_inner2 = (self.radius.saturating_sub(1) as i32).pow(2);

        for dy in -(self.radius as i32)..=(self.radius as i32) {
            for dx in -(self.radius as i32)..=(self.radius as i32) {
                let dist2 = dx * dx + dy * dy;
                let x = self.center.x + dx;
                let y = self.center.y + dy;

                if dist2 <= r2 {
                    let chr = if dist2 >= r_inner2 {
                        self.border_style
                    } else {
                        match self.fill_style {
                            Some(ch) => ch,
                            None => continue,
                        }
                    };

                    out.push(BasicDraw {
                        pos: Point { x, y },
                        chr,
                    });
                }
            }
        }

        Ok(out)
    }

    fn bounding_iv(&self, _sprites: &SpriteRegistry) -> HashMap<i32, Vec<UpdateInterval>> {
        let mut intervals: HashMap<i32, Vec<UpdateInterval>> = HashMap::new();
        let r = self.radius as usize;
        let center = Point {
            x: self.center.x as usize,
            y: self.center.y as usize,
        };

        fn push_interval(
            map: &mut HashMap<i32, Vec<UpdateInterval>>,
            y: usize,
            start: usize,
            end_exclusive: usize,
        ) {
            if start < end_exclusive {
                map.entry(y as i32).or_default().push(UpdateInterval {
                    interval: (start, end_exclusive),
                    iv_type: UpdateIntervalType::Optimized,
                });
            }
        }

        for dy in 0..=2 * r {
            let y = center.y + dy.saturating_sub(r);
            let dist_y = (dy as i32 - r as i32).pow(2);
            if (r as i32).pow(2) >= dist_y {
                let dx = (((r * r) as i32 - dist_y) as f64).sqrt() as usize;
                let start = center.x.saturating_sub(dx);
                let end = center.x + dx + 1;
                push_interval(&mut intervals, y, start, end);
            }
        }

        intervals
    }

    fn shifted(&self, offset: Point<i32>) -> Box<dyn Drawable> {
        Box::new(CircleDrawable {
            center: Point {
                x: self.center.x + offset.x,
                y: self.center.y + offset.y,
            },
            radius: self.radius,
            border_style: self.border_style,
            fill_style: self.fill_style,
        })
    }
}
