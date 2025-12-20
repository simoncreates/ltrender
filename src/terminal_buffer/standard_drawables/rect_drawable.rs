use crate::{
    DoublePointed, DrawError, Drawable, ScreenFitting, SpriteRegistry,
    drawable_traits::basic_draw_creator::BasicDrawCreator,
    update_interval_handler::UpdateIntervalCreator,
};
use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};

#[derive(Clone, Debug)]
pub enum BorderStyle {
    AllRound(TerminalChar),
    Custom {
        top: TerminalChar,
        bottom: TerminalChar,
        left: TerminalChar,
        right: TerminalChar,
    },
    Basic,
}
#[derive(Clone, Debug)]
pub enum BorderStyleCustomFields {
    Top,
    Bottom,
    Left,
    Right,
}
#[derive(Clone, Debug)]
pub enum ScreenFitType {
    // the rect always fills out the whole screen
    Full,
    // only certain sides of the rect will be fitted to the scree
    Partial(Vec<BorderStyleCustomFields>),
}

#[derive(Clone, Debug)]
pub struct RectDrawable {
    pub rect: Rect<i32>,
    pub border_thickness: usize,
    pub border_style: BorderStyle,
    pub fill_style: Option<TerminalChar>,
    pub screen_fit: Option<ScreenFitType>,
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

impl RectDrawable {
    fn get_borderchar(&self, opt_style: Option<BorderStyleCustomFields>) -> TerminalChar {
        match self.border_style {
            BorderStyle::Basic => TerminalChar::from_char('#'),
            BorderStyle::AllRound(chr) => chr,
            BorderStyle::Custom {
                top,
                bottom,
                left,
                right,
            } => {
                if let Some(style) = opt_style {
                    match style {
                        BorderStyleCustomFields::Top => top,
                        BorderStyleCustomFields::Left => left,
                        BorderStyleCustomFields::Right => right,
                        BorderStyleCustomFields::Bottom => bottom,
                    }
                } else {
                    panic!("this is an internal function error, it should be impossible to reach")
                }
            }
        }
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
        let rect = &self.rect;
        for y in rect.p1.y..=rect.p2.y {
            if y < (rect.p1.y + self.border_thickness as i32) {
                let field = Some(BorderStyleCustomFields::Top);
                let chr = self.get_borderchar(field);
                out.draw_line((rect.p1.x, y), (rect.p2.x, y), chr);
            } else if y > (rect.p2.y - self.border_thickness as i32) {
                let field = Some(BorderStyleCustomFields::Bottom);
                let chr = self.get_borderchar(field);
                out.draw_line((rect.p1.x, y), (rect.p2.x, y), chr);
            } else {
                let left_chr = self.get_borderchar(Some(BorderStyleCustomFields::Left));
                let right_chr = self.get_borderchar(Some(BorderStyleCustomFields::Right));
                // left border
                out.draw_line(
                    (rect.p1.x, y),
                    (rect.p1.x + self.border_thickness as i32, y),
                    left_chr,
                );
                // filling
                if let Some(style) = self.fill_style {
                    let fill_start = rect.p1.x + self.border_thickness as i32;
                    let fill_end = rect.p2.x - self.border_thickness as i32;
                    if fill_start <= fill_end {
                        out.draw_line((fill_start, y), (fill_end, y), style);
                    }
                }
                // right border
                out.draw_line(
                    (rect.p2.x - self.border_thickness as i32 + 1, y),
                    (rect.p2.x, y),
                    right_chr,
                );
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
    fn as_screen_fitting_mut(&mut self) -> Option<&mut dyn ScreenFitting> {
        Some(self)
    }
}

impl ScreenFitting for RectDrawable {
    fn fit_to_screen(&mut self, rect: Rect<i32>) {
        if let Some(fit) = &self.screen_fit {
            match fit {
                ScreenFitType::Full => {
                    self.rect = rect;
                }
                ScreenFitType::Partial(fits) => {
                    for border_fit in fits {
                        match border_fit {
                            BorderStyleCustomFields::Top => {
                                self.rect.p1.y = rect.p1.y;
                            }
                            BorderStyleCustomFields::Bottom => {
                                self.rect.p2.y = rect.p2.y;
                            }
                            BorderStyleCustomFields::Left => {
                                self.rect.p1.x = rect.p1.x;
                            }
                            BorderStyleCustomFields::Right => {
                                self.rect.p2.x = rect.p2.x;
                            }
                        }
                    }
                }
            }
        }
    }
}
