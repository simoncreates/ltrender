use ascii_assets::TerminalChar;
use common_stdx::Point;

use crate::draw::{
    error::DrawObjectBuilderError,
    terminal_buffer::{Drawable, standard_drawables::PolygonDrawable},
};

#[derive(Default)]
pub struct PolygonDrawableBuilder {
    points: Option<Vec<Point<u16>>>,
    border_style: Option<TerminalChar>,
    fill_style: Option<TerminalChar>,
}

impl PolygonDrawableBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn points(mut self, points: Vec<Point<u16>>) -> Self {
        self.points = Some(points);
        self
    }

    pub fn border_style(mut self, chr: TerminalChar) -> Self {
        self.border_style = Some(chr);
        self
    }

    pub fn fill_style(mut self, chr: TerminalChar) -> Self {
        self.fill_style = Some(chr);
        self
    }

    pub fn build(self) -> Result<Box<dyn Drawable>, DrawObjectBuilderError> {
        let points = self
            .points
            .ok_or(DrawObjectBuilderError::FailedToBuildPolygonObject())?;
        if points.is_empty() {
            return Err(DrawObjectBuilderError::FailedToBuildPolygonObject());
        }

        Ok(Box::new(PolygonDrawable {
            points,
            border_style: self
                .border_style
                .ok_or(DrawObjectBuilderError::FailedToBuildPolygonObject())?,
            fill_style: self.fill_style,
        }))
    }
}
