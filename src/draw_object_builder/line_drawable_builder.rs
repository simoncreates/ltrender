use ascii_assets::TerminalChar;
use common_stdx::Point;

use crate::{error::DrawObjectBuilderError, terminal_buffer::LineDrawable};

use crate::{Drawable, handle_char_field, handle_pointed_field};

#[derive(Default)]
pub struct LineDrawableBuilder {
    start: Option<Point<i32>>,
    end: Option<Point<i32>>,
    chr: Option<TerminalChar>,
}

impl LineDrawableBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    handle_pointed_field!(start, start);
    handle_pointed_field!(end, end);

    handle_char_field!(chr, chr);

    pub fn build(self) -> Result<Box<dyn Drawable>, DrawObjectBuilderError> {
        Ok(Box::new(LineDrawable {
            start: self
                .start
                .ok_or(DrawObjectBuilderError::FailedToBuildLineObject())?,
            end: self
                .end
                .ok_or(DrawObjectBuilderError::FailedToBuildLineObject())?,
            chr: self
                .chr
                .ok_or(DrawObjectBuilderError::FailedToBuildLineObject())?,
        }))
    }
}
