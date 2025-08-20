use ascii_assets::TerminalChar;
use common_stdx::Rect;

use crate::draw::{
    error::DrawObjectBuilderError,
    terminal_buffer::{Drawable, standard_drawables::RectDrawable},
};

use crate::handle_char_field;

#[derive(Default)]
pub struct RectDrawableBuilder {
    rect: Option<Rect<i32>>,
    border_thickness: Option<usize>,
    border_style: Option<TerminalChar>,
    fill_style: Option<TerminalChar>,
}

impl RectDrawableBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rect(mut self, rect: Rect<i32>) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn border_thickness(mut self, thickness: usize) -> Self {
        self.border_thickness = Some(thickness);
        self
    }

    handle_char_field!(border_style, border_style);
    handle_char_field!(fill_style, fill_style);

    pub fn build(self) -> Result<Box<dyn Drawable>, DrawObjectBuilderError> {
        Ok(Box::new(RectDrawable {
            rect: self
                .rect
                .ok_or(DrawObjectBuilderError::FailedToBuildRectObject())?,
            border_thickness: self
                .border_thickness
                .ok_or(DrawObjectBuilderError::FailedToBuildRectObject())?,
            border_style: self
                .border_style
                .ok_or(DrawObjectBuilderError::FailedToBuildRectObject())?,
            fill_style: self.fill_style,
        }))
    }
}
