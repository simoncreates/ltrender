use crate::{
    DrawError, SpriteRegistry,
    terminal_buffer::{
        Drawable,
        drawable::{BasicDraw, DoublePointed},
    },
    update_interval_handler::UpdateIntervalCreator,
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

    fn bounding_iv(&self, _sprites: &SpriteRegistry) -> Option<UpdateIntervalCreator> {
        let mut c = UpdateIntervalCreator::new();

        let r = self.radius as i32;
        let center = Point {
            x: self.center.x,
            y: self.center.y,
        };

        for dy in 0..=2 * r {
            let y = center.y + dy.saturating_sub(r);
            let dist_y = (dy - r).pow(2);
            if r.pow(2) >= dist_y {
                let dx = (((r * r) - dist_y) as f64).sqrt() as i32;
                let start = center.x.saturating_sub(dx);
                let end = center.x + dx + 1;
                c.add_interval(y, (start, end));
            }
        }

        Some(c)
    }
}
