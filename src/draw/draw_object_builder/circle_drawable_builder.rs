use crate::draw::{
    error::DrawObjectBuilderError,
    terminal_buffer::{drawable::Drawable, standard_drawables::CircleDrawable},
};
use ascii_assets::TerminalChar;
use common_stdx::Point;

#[derive(Default)]
pub struct CircleDrawableBuilder {
    center: Option<Point<u16>>,
    radius: Option<u16>,
    border_style: Option<TerminalChar>,
    fill_style: Option<TerminalChar>,
}

impl CircleDrawableBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn center(mut self, center: Point<u16>) -> Self {
        self.center = Some(center);
        self
    }

    pub fn radius(mut self, radius: u16) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn border_style(mut self, style: TerminalChar) -> Self {
        self.border_style = Some(style);
        self
    }

    pub fn fill_style(mut self, style: TerminalChar) -> Self {
        self.fill_style = Some(style);
        self
    }

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
