use ascii_assets::{Color, TerminalChar};
use common_stdx::{Point, Rect};
use log::info;

use crate::{
    DrawError, Drawable, ScreenFitting, ScreenKey,
    input_handler::hook::EventHook,
    rendering::{render_handle::RenderHandle, renderer::RenderModeBehavior},
    terminal_buffer::{RectDrawable, standard_drawables::rect_drawable::ScreenFitType},
};

#[derive(Debug)]
pub enum SelectMenuFitting {
    FitToContents,
    FitToScreen,
}

#[derive(Debug)]
pub struct SelectMenuDrawable {
    fields: Vec<Box<dyn Drawable>>,
    selected_field: u16,
    pub border_rect: RectDrawable,
    fitting: SelectMenuFitting,
    event_hook: EventHook,
}

impl SelectMenuDrawable {
    pub fn new(
        fields: Vec<Box<dyn Drawable>>,
        border_rect: RectDrawable,
        fitting: SelectMenuFitting,
        event_hook: EventHook,
    ) -> Self {
        SelectMenuDrawable {
            fields,
            selected_field: 0,
            border_rect,
            fitting,
            event_hook,
        }
    }
}

impl Drawable for SelectMenuDrawable {
    fn size(&self, sprites: &crate::SpriteRegistry) -> Result<(u16, u16), crate::DrawError> {
        let mut current_biggest = (0, 0);
        for obj in &self.fields {
            let size = obj.size(sprites)?;
            if size.0 > current_biggest.0 {
                current_biggest.0 = size.0
            }
            if size.1 > current_biggest.1 {
                current_biggest.1 = size.1
            }
        }
        Ok(current_biggest)
    }
    fn draw(
        &mut self,
        sprites: &crate::SpriteRegistry,
    ) -> Result<crate::drawable_traits::basic_draw_creator::BasicDrawCreator, crate::DrawError>
    {
        let mut current_offset = Point::from((1, 1));

        if let SelectMenuFitting::FitToContents = self.fitting {
            let size = self.size(sprites)?;
            self.border_rect.rect = Rect::from_coords(0, 0, size.0 as i32 + 1, size.1 as i32 + 1)
        }

        let mut initial = self
            .border_rect
            .draw(sprites)
            .map_err(|_| DrawError::FailedDrawing("unable to draw rect drawable".into()))?;
        for (idx, obj) in self.fields.iter_mut().enumerate() {
            let mut bdc = if let Ok(bdc) = obj.draw(sprites) {
                bdc
            } else {
                return Err(DrawError::FailedDrawing(format!(
                    "unable to draw field with idx: {} in SelectMenuDrawable",
                    idx
                )));
            };
            let bbox = bdc.get_bounding_box();
            info!("size of unaligned fresh rect: {:?}", bbox);
            let height = bbox.p2.y - bbox.p1.y;
            bdc.align_to_origin(current_offset);
            info!("size of aligned rect: {:?}", bdc.get_bounding_box());
            if self.selected_field == idx as u16 {
                initial.draw_line(
                    current_offset + Point::from((-1, 0)),
                    current_offset + Point::from((-1, height)),
                    TerminalChar::from_char('>').set_fg(Color::rgb(255, 50, 50)),
                );
            }
            current_offset.y += height;
            initial.merge_creator(bdc);
        }
        Ok(initial)
    }
    fn as_screen_fitting_mut(&mut self) -> Option<&mut dyn crate::ScreenFitting> {
        Some(self)
    }
}

impl ScreenFitting for SelectMenuDrawable {
    fn fit_to_screen(&mut self, rect: common_stdx::Rect<i32>) {
        if let SelectMenuFitting::FitToScreen = self.fitting {
            self.border_rect.screen_fit = Some(ScreenFitType::Full);
            self.border_rect.fit_to_screen(rect);
        };
    }
}
