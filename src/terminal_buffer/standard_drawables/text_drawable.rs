use std::ops::Range;

use crate::{
    DoublePointed, DrawError, Drawable, SpriteRegistry,
    drawable_traits::basic_draw_creator::BasicDrawCreator,
    update_interval_handler::UpdateIntervalCreator,
};
use ascii_assets::{Color, TerminalChar};
use common_stdx::{Point, Rect};

#[derive(Clone, Debug)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, Default)]
pub struct TextStyle {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct StyledSpan {
    pub range: Range<usize>,
    pub style: TextStyle,
}

#[derive(Clone, Debug)]
pub struct LineInfo {
    pub text: String,
    pub spans: Vec<StyledSpan>,
    pub alignment: TextAlignment,
    pub default_style: TextStyle,
}

#[derive(Clone, Debug)]
pub struct TextDrawable {
    pub area: Rect<i32>,
    pub lines: Vec<LineInfo>,
    pub wrapping: bool,
    pub scroll_y: u16,
}

impl DoublePointed for TextDrawable {
    fn start(&self) -> Point<i32> {
        self.area.p1
    }

    fn end(&self) -> Point<i32> {
        self.area.p2
    }

    fn set_start(&mut self, p: Point<i32>) {
        self.area.p1 = p;
    }

    fn set_end(&mut self, p: Point<i32>) {
        self.area.p2 = p;
    }
}

impl Drawable for TextDrawable {
    fn size(&self, _sprites: &SpriteRegistry) -> Result<(u16, u16), DrawError> {
        let size = (self.area.p2 - self.area.p1) + Point::new(1, 1);
        Ok((size.x as u16, size.y as u16))
    }
    fn as_double_pointed_mut(&mut self) -> Option<&mut dyn DoublePointed> {
        Some(self)
    }
    fn draw(&mut self, _sprites: &SpriteRegistry) -> Result<BasicDrawCreator, DrawError> {
        if self.area.p1.x > self.area.p2.x || self.area.p1.y > self.area.p2.y {
            return Ok(BasicDrawCreator::new());
        }

        let max_width = self.area.width() as usize;
        let max_height = self.area.height() as usize;
        let mut out = BasicDrawCreator::new();
        let mut visual_row = 0;

        for line in self.lines.iter().skip(self.scroll_y as usize) {
            // carry both char and original index in line.text
            let indexed_chars: Vec<(usize, char)> = line.text.char_indices().collect();

            // wrap into chunks of (usize, char)
            let wrapped_lines = if self.wrapping && max_width > 0 {
                indexed_chars
                    .chunks(max_width)
                    .map(|chunk| chunk.to_vec())
                    .collect::<Vec<_>>()
            } else {
                vec![indexed_chars]
            };

            for visual in wrapped_lines {
                if visual_row >= max_height {
                    break;
                }

                let y = self.area.p1.y + visual_row as i32;
                let line_len = visual.len() as i32;
                let mut x = self.area.p1.x;

                match line.alignment {
                    TextAlignment::Left => {}
                    TextAlignment::Center => {
                        x += (self.area.width() - line_len) / 2;
                    }
                    TextAlignment::Right => {
                        x += self.area.width() - line_len;
                    }
                }

                for &(idx, ch) in &visual {
                    if x >= self.area.p2.x {
                        break;
                    }
                    if x >= self.area.p1.x {
                        // determine applicable style
                        let style = line
                            .spans
                            .iter()
                            .find(|span| span.range.contains(&idx))
                            .map(|s| &s.style)
                            .unwrap_or(&line.default_style);

                        let mut tc = TerminalChar::from_char(ch);
                        if let Some(fg) = style.foreground {
                            tc.fg_color = Some(fg);
                        }
                        if let Some(bg) = style.background {
                            tc.bg_color = Some(bg);
                        }
                        // todo: add bold and italic _somehow_
                        // tc.bold = style.bold.unwrap_or(false);
                        // tc.italic = style.italic.unwrap_or(false);

                        out.draw_char((x, y), tc);
                    }
                    x += 1;
                }

                visual_row += 1;
                if visual_row >= max_height {
                    break;
                }
            }

            if visual_row >= max_height {
                break;
            }
        }

        Ok(out)
    }

    fn bounding_iv(&self, _sprites: &SpriteRegistry) -> Option<UpdateIntervalCreator> {
        let mut c = UpdateIntervalCreator::new();
        c.register_redraw_region(self.area);
        Some(c)
    }
}
