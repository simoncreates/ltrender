use std::{any::Any, collections::HashMap};

use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};

use crate::draw::{
    DrawError, SpriteRegistry, UpdateInterval,
    terminal_buffer::{
        Drawable,
        drawable::{BasicDraw, convert_rect_to_update_intervals},
    },
};

#[derive(Clone, Debug)]
pub struct LineDrawable {
    pub start: Point<u16>,
    pub end: Point<u16>,
    pub chr: TerminalChar,
}

impl Drawable for LineDrawable {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn draw(&self, _sprites: &SpriteRegistry) -> Result<Vec<BasicDraw>, DrawError> {
        let mut x0 = self.start.x as i32;
        let mut y0 = self.start.y as i32;
        let x1 = self.end.x as i32;
        let y1 = self.end.y as i32;

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();

        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx + dy;

        let mut out: Vec<BasicDraw> = Vec::new();

        loop {
            out.push(BasicDraw {
                pos: Point {
                    x: x0 as u16,
                    y: y0 as u16,
                },
                chr: self.chr,
            });

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

        Ok(out)
    }

    fn bounding_iv(&self, _sprites: &SpriteRegistry) -> HashMap<u16, Vec<UpdateInterval>> {
        let min_x = self.start.x.min(self.end.x);
        let max_x = self.start.x.max(self.end.x);
        let min_y = self.start.y.min(self.end.y);
        let max_y = self.start.y.max(self.end.y);

        convert_rect_to_update_intervals(Rect {
            p1: Point { x: min_x, y: min_y },
            p2: Point {
                x: max_x.saturating_add(1),
                y: max_y.saturating_add(1),
            },
        })
    }

    fn shifted(&self, offset: Point<u16>) -> Box<dyn Drawable> {
        Box::new(LineDrawable {
            start: self.start + offset,
            end: self.end + offset,
            chr: self.chr,
        })
    }
}
