use std::sync::{
    Arc,
    atomic::{AtomicU16, Ordering},
};

use ascii_assets::{Color, TerminalChar};
use common_stdx::{Point, Rect};
use crossterm::event::KeyCode;
use log::info;

use crate::{
    DrawError, Drawable, ScreenFitting, ScreenKey,
    error::AppError,
    input_handler::{
        hook::EventHook,
        manager::{
            KeyAction, KeyMessage, KeySubscriptionTypes, SubscriptionMessage, SubscriptionType,
        },
    },
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
    selected_field: Arc<AtomicU16>,
    pub border_rect: RectDrawable,
    fitting: SelectMenuFitting,

    _event_hook: EventHook,
}

impl SelectMenuDrawable {
    pub fn new<M>(
        fields: Vec<Box<dyn Drawable>>,
        border_rect: RectDrawable,
        fitting: SelectMenuFitting,
        menu_screen: ScreenKey,
        mut event_hook: EventHook,
        r: RenderHandle<M>,
    ) -> Result<Self, AppError>
    where
        M: RenderModeBehavior + Sized + Send + 'static,
    {
        let global_selected_field = Arc::new(AtomicU16::new(0));
        let selected_field = global_selected_field.clone();
        let amount_of_fields = Arc::new(AtomicU16::new(fields.len() as u16));
        event_hook.subscribe(
            SubscriptionType::Key(KeySubscriptionTypes::All, KeyAction::Pressed),
            move |k| {
                info!("we received sub message: {}", k);
                if let SubscriptionMessage::Key { msg, screen } = k
                    && screen.targeting(menu_screen)
                {
                    info!("this is actually us lmao");
                    let current = global_selected_field.load(Ordering::Relaxed);
                    let max = amount_of_fields.load(Ordering::Relaxed);

                    if let KeyMessage::Pressed(KeyCode::Char('a'), _) = msg {
                        info!("going left");
                        if current < max {
                            global_selected_field.fetch_add(1, Ordering::Relaxed);
                        }
                        let _ = r.render_screen(menu_screen);
                    } else if let KeyMessage::Pressed(KeyCode::Char('d'), _) = msg
                        && current > 0
                    {
                        // todo why does it only update when i resize the terminal?
                        info!("going right");
                        global_selected_field.fetch_sub(1, Ordering::Relaxed);
                        let _ = r.render_screen(menu_screen);
                    }
                }
            },
        )?;
        Ok(SelectMenuDrawable {
            fields,
            selected_field,
            border_rect,
            fitting,
            _event_hook: event_hook,
        })
    }
}

impl Drawable for SelectMenuDrawable {
    fn size(&self, sprites: &crate::SpriteRegistry) -> Result<(u16, u16), crate::DrawError> {
        let mut max_width: u16 = 0;
        let mut total_height: u16 = 0;

        for obj in &self.fields {
            let (w, h) = obj.size(sprites)?;
            if w > max_width {
                max_width = w;
            }
            total_height = total_height.saturating_add(h)
        }

        Ok((max_width + 1, total_height))
    }
    fn draw(
        &mut self,
        sprites: &crate::SpriteRegistry,
    ) -> Result<crate::drawable_traits::basic_draw_creator::BasicDrawCreator, crate::DrawError>
    {
        let mut current_offset = Point::from((1, 1));

        if let SelectMenuFitting::FitToContents = self.fitting {
            let size = self.size(sprites)?;
            self.border_rect.rect = Rect::from_coords(0, 0, size.0 as i32 + 1, size.1 as i32)
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
            if self.selected_field.load(Ordering::Relaxed) == idx as u16 {
                info!("drawing selector at: {:?}", current_offset);
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
