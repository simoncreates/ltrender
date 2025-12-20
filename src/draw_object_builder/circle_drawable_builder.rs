use crate::{
    Drawable, error::DrawObjectBuilderError, terminal_buffer::standard_drawables::CircleDrawable,
};
use ascii_assets::TerminalChar;
use common_stdx::Point;

use crate::{handle_char_field, handle_pointed_field};

#[derive(Default)]
pub struct CircleDrawableBuilder {
    center: Option<Point<i32>>,
    radius: Option<u16>,
    border_style: Option<TerminalChar>,
    fill_style: Option<TerminalChar>,
}

impl CircleDrawableBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    handle_pointed_field!(center, center);

    pub fn radius(mut self, radius: u16) -> Self {
        self.radius = Some(radius);
        self
    }

    handle_char_field!(border_style, border_style);
    handle_char_field!(fill_style, fill_style);

    pub fn build(self) -> Result<Box<dyn Drawable>, DrawObjectBuilderError> {
        Ok(Box::new(CircleDrawable {
            center: self.center.unwrap_or(Point { x: 0, y: 0 }),
            radius: self
                .radius
                .ok_or(DrawObjectBuilderError::FailedToBuildCircleObject())?,
            border_style: self
                .border_style
                .unwrap_or_else(|| TerminalChar::from_char('#')),
            fill_style: self.fill_style,
        }))
    }
}
