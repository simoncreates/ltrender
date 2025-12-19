use ascii_assets::TerminalChar;
use common_stdx::Rect;

use crate::{
    error::DrawObjectBuilderError,
    terminal_buffer::{
        Drawable,
        standard_drawables::{
            RectDrawable,
            rect_drawable::{BorderStyle, ScreenFitType},
        },
    },
};

use crate::handle_char_field;

#[derive(Default)]
pub struct RectDrawableBuilder {
    rect: Option<Rect<i32>>,
    border_thickness: Option<usize>,
    border_style: Option<BorderStyle>,
    fill_style: Option<TerminalChar>,
    screen_fit: Option<ScreenFitType>,
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

    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = Some(style);
        self
    }

    pub fn screen_fit(mut self, fit_type: ScreenFitType) -> Self {
        self.screen_fit = Some(fit_type);
        self
    }

    handle_char_field!(fill_style, fill_style);

    pub fn build(self) -> Result<Box<dyn Drawable>, DrawObjectBuilderError> {
        Ok(Box::new(RectDrawable {
            rect: self
                .rect
                .ok_or(DrawObjectBuilderError::FailedToBuildRectObject())?,
            border_thickness: self.border_thickness.unwrap_or(0),
            border_style: self.border_style.unwrap_or(BorderStyle::Basic),
            fill_style: self.fill_style,
            screen_fit: self.screen_fit,
        }))
    }
}
