use std::collections::HashMap;

use crate::draw::{
    DrawError, SpriteRegistry, UpdateInterval,
    terminal_buffer::{
        Drawable,
        drawable::{BasicDraw, DoublePointed, convert_rect_to_update_intervals},
    },
    update_interval_handler::UpdateIntervalType,
};
use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};

#[derive(Clone, Debug)]
pub struct RectDrawable {
    pub rect: Rect<u16>,
    pub border_thickness: u16,
    pub border_style: TerminalChar,
    pub fill_style: Option<TerminalChar>,
}

impl DoublePointed for RectDrawable {
    fn start(&self) -> Point<u16> {
        self.rect.p1
    }
    fn end(&self) -> Point<u16> {
        self.rect.p2
    }

    fn set_start(&mut self, p: Point<u16>) {
        self.rect.p1 = p;
    }
    fn set_end(&mut self, p: Point<u16>) {
        self.rect.p2 = p;
    }
}

impl Drawable for RectDrawable {
    fn as_double_pointed_mut(&mut self) -> Option<&mut dyn DoublePointed> {
        Some(self)
    }
    fn draw(&self, _sprites: &SpriteRegistry) -> Result<Vec<BasicDraw>, DrawError> {
        if self.rect.p1.x > self.rect.p2.x || self.rect.p1.y > self.rect.p2.y {
            return Ok(Vec::new());
        }

        let mut out = Vec::with_capacity(
            ((self.rect.p2.x - self.rect.p1.x + 1) as usize)
                * ((self.rect.p2.y - self.rect.p1.y + 1) as usize),
        );

        for y in self.rect.p1.y..=self.rect.p2.y {
            for x in self.rect.p1.x..=self.rect.p2.x {
                let left = x - self.rect.p1.x;
                let right = self.rect.p2.x - x;
                let top = y - self.rect.p1.y;
                let bottom = self.rect.p2.y - y;

                let min_dist = *[left, right, top, bottom].iter().min().unwrap();

                let chr = if min_dist < self.border_thickness {
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

        Ok(out)
    }

    fn bounding_iv(&self, _sprites: &SpriteRegistry) -> HashMap<u16, Vec<UpdateInterval>> {
        if self.border_thickness == 0 {
            return HashMap::new();
        }

        if self.fill_style.is_some() {
            let rect = self.rect;
            return convert_rect_to_update_intervals(Rect {
                p1: rect.p1,
                p2: Point {
                    x: rect.p2.x.saturating_add(1),
                    y: rect.p2.y.saturating_add(1),
                },
            });
        }

        let mut intervals = HashMap::new();

        fn push_interval(
            map: &mut HashMap<u16, Vec<UpdateInterval>>,
            y: u16,
            start: u16,
            end_exclusive: u16,
        ) {
            if start < end_exclusive {
                map.entry(y).or_default().push(UpdateInterval {
                    interval: (start, end_exclusive),
                    iv_type: UpdateIntervalType::Optimized,
                });
            }
        }

        let t = self.border_thickness;
        let rect = self.rect;
        for y in rect.p1.y..=rect.p2.y {
            let is_top = y < rect.p1.y + t;
            let is_bottom = y > rect.p2.y - t;

            if is_top || is_bottom {
                push_interval(&mut intervals, y, rect.p1.x, rect.p2.x.saturating_add(1));
                continue;
            }

            let left_start = rect.p1.x;
            let left_end = (rect.p1.x + t).min(rect.p2.x.saturating_add(1));

            let right_start_opt = rect.p2.x.checked_sub(t - 1);
            if let Some(right_start) = right_start_opt {
                let right_end = rect.p2.x.saturating_add(1);
                if left_start < right_start {
                    push_interval(&mut intervals, y, left_start, left_end);
                    push_interval(&mut intervals, y, right_start, right_end);
                } else {
                    let start = left_start.min(right_start);
                    let end = left_end.max(right_end);
                    push_interval(&mut intervals, y, start, end);
                }
            }
        }

        intervals
    }

    fn shifted(&self, offset: Point<u16>) -> Box<dyn Drawable> {
        Box::new(RectDrawable {
            rect: Rect {
                p1: self.rect.p1 + offset,
                p2: self.rect.p2 + offset,
            },
            border_thickness: self.border_thickness,
            border_style: self.border_style,
            fill_style: self.fill_style,
        })
    }
}
