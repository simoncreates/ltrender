use ascii_assets::TerminalChar;
use common_stdx::Point;

use crate::draw::{
    error::DrawObjectBuilderError,
    terminal_buffer::{Drawable, LineDrawable},
};

#[derive(Default)]
pub struct LineDrawableBuilder {
    start: Option<Point<u16>>,
    end: Option<Point<u16>>,
    chr: Option<TerminalChar>,
}

impl LineDrawableBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(mut self, start: Point<u16>) -> Self {
        self.start = Some(start);
        self
    }

    pub fn end(mut self, end: Point<u16>) -> Self {
        self.end = Some(end);
        self
    }

    pub fn chr(mut self, chr: TerminalChar) -> Self {
        self.chr = Some(chr);
        self
    }

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
