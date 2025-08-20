use ascii_assets::TerminalChar;
use common_stdx::Point;

use crate::draw::{
    error::DrawObjectBuilderError,
    terminal_buffer::{Drawable, standard_drawables::PolygonDrawable},
};

use crate::handle_char_field;

#[derive(Default)]
pub struct PolygonDrawableBuilder {
    points: Option<Vec<Point<i32>>>,
    border_style: Option<TerminalChar>,
    fill_style: Option<TerminalChar>,
}

impl PolygonDrawableBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn points(mut self, points: Vec<impl Into<Point<i32>>>) -> Self {
        let mut out = Vec::new();
        for point in points {
            out.push(point.into());
        }
        self.points = Some(out);
        self
    }

    handle_char_field!(border_style, border_style);
    handle_char_field!(fill_style, fill_style);

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
