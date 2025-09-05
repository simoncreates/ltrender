use crate::{
    DrawError, SpriteRegistry,
    terminal_buffer::{BasicDrawCreator, Drawable, drawable::DoublePointed},
    update_interval_handler::UpdateIntervalCreator,
};
use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};

#[derive(Clone, Debug)]
pub struct RectDrawable {
    pub rect: Rect<i32>,
    pub border_thickness: usize,
    pub border_style: TerminalChar,
    pub fill_style: Option<TerminalChar>,
}

impl DoublePointed for RectDrawable {
    fn start(&self) -> Point<i32> {
        Point {
            x: self.rect.p1.x,
            y: self.rect.p1.y,
        }
    }
    fn end(&self) -> Point<i32> {
        Point {
            x: self.rect.p2.x,
            y: self.rect.p2.y,
        }
    }

    fn set_start(&mut self, p: Point<i32>) {
        self.rect.p1 = p;
    }
    fn set_end(&mut self, p: Point<i32>) {
        self.rect.p2 = p;
    }
}

impl Drawable for RectDrawable {
    fn size(&self, _sprites: &SpriteRegistry) -> Result<(u16, u16), DrawError> {
        let size = (self.rect.p2 - self.rect.p1) + Point::new(1, 1);
        Ok((size.x as u16, size.y as u16))
    }
    fn as_double_pointed_mut(&mut self) -> Option<&mut dyn DoublePointed> {
        Some(self)
    }
    fn draw(&mut self, _sprites: &SpriteRegistry) -> Result<BasicDrawCreator, DrawError> {
        if self.rect.p1.x > self.rect.p2.x || self.rect.p1.y > self.rect.p2.y {
            return Ok(BasicDrawCreator::new());
        }

        let expected = if self.fill_style.is_some() {
            (self.rect.p2.x - self.rect.p1.x + 1) * (self.rect.p2.y - self.rect.p1.y + 1)
        } else {
            2 * (self.rect.p2.x - self.rect.p1.x + 1 + self.rect.p2.y - self.rect.p1.y + 1)
        };

        let cap = expected.clamp(0, i32::MAX) as usize;
        let mut out = BasicDrawCreator::new_with_capacity(cap);

        for y in self.rect.p1.y..=self.rect.p2.y {
            for x in self.rect.p1.x..=self.rect.p2.x {
                let left = x - self.rect.p1.x;
                let right = self.rect.p2.x - x;
                let top = y - self.rect.p1.y;
                let bottom = self.rect.p2.y - y;

                let min_dist = *[left, right, top, bottom].iter().min().unwrap();

                let chr = if min_dist < self.border_thickness as i32 {
                    self.border_style
                } else {
                    match self.fill_style {
                        Some(ch) => ch,
                        None => continue,
                    }
                };
                out.draw_char((x, y), chr);
            }
        }

        Ok(out)
    }

    fn bounding_iv(&self, _sprites: &SpriteRegistry) -> Option<UpdateIntervalCreator> {
        let mut c = UpdateIntervalCreator::new();
        if self.border_thickness == 0 {
            return Some(c);
        }

        if self.fill_style.is_some() {
            let rect = self.rect;
            c.register_redraw_region(Rect {
                p1: Point {
                    x: rect.p1.x,
                    y: rect.p1.y,
                },
                p2: Point {
                    x: rect.p2.x.saturating_add(1),
                    y: rect.p2.y.saturating_add(1),
                },
            });
            return Some(c);
        }
        let t = self.border_thickness;
        let rect = self.rect;
        for y in rect.p1.y..=rect.p2.y {
            let is_top = y < rect.p1.y + t as i32;
            let is_bottom = y > rect.p2.y - t as i32;

            if is_top || is_bottom {
                c.add_interval(y, (rect.p1.x, rect.p2.x.saturating_add(1)));
                continue;
            }

            let left_start = rect.p1.x;
            let left_end = (rect.p1.x + t as i32).min(rect.p2.x.saturating_add(1));

            let right_start_opt = rect.p2.x.checked_sub(t as i32 - 1);
            if let Some(right_start) = right_start_opt {
                let right_end = rect.p2.x.saturating_add(1);
                if left_start < right_start {
                    c.add_interval(y, (left_start, left_end));
                    c.add_interval(y, (right_start, right_end));
                } else {
                    let start = left_start.min(right_start);
                    let end = left_end.max(right_end);
                    c.add_interval(y, (start, end));
                }
            }
        }

        Some(c)
    }
}
