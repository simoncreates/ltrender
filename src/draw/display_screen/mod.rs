use std::sync::{Arc, Mutex};

use crate::draw::{
    self, DrawError, DrawObject, DrawObjectLibrary, DrawableId, ScreenBuffer, SpriteRegistry,
};
use common_stdx::Rect;
use log::info;

pub type ScreenKey = usize;

#[derive(Debug)]
pub struct Screen {
    layer: usize,
    id: ScreenKey,
    area: Rect<u16>,
    draw_objects: Vec<DrawableId>,
}

impl Screen {
    pub fn new(area: Rect<u16>, layer: usize, id: ScreenKey) -> Self {
        Screen {
            layer,
            id,
            area,
            draw_objects: Vec::new(),
        }
    }

    fn shift_obj_to_local_cords(&self, obj: &DrawObject) -> DrawObject {
        let shifted_drawable = obj.drawable.lock().unwrap().shifted(self.area.p1);
        DrawObject {
            layer: obj.layer,
            drawable: Arc::new(Mutex::new(shifted_drawable)),
        }
    }

    pub fn register_drawable(&mut self, obj_id: DrawableId) {
        self.draw_objects.push(obj_id);
    }

    pub fn render_drawable<T>(
        &mut self,
        obj_id: DrawableId,
        screen_buffer: &mut T,
        obj_library: &DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        T: ScreenBuffer,
    {
        let opt_obj = obj_library.find_drawable(self.id, obj_id);
        let obj = if let Some(obj) = opt_obj {
            self.shift_obj_to_local_cords(obj)
        } else {
            return Err(DrawError::DrawableHandleNotFound {
                screen_id: self.id,
                obj_id,
            });
        };
        screen_buffer.add_to_buffer(&obj, obj_id, self.layer, &self.area, sprites)?;
        Ok(())
    }
    pub fn remove_drawable<T>(
        &mut self,
        obj_id: DrawableId,
        screen_buffer: &mut T,
        obj_library: &DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        T: ScreenBuffer,
    {
        let opt_obj = obj_library.find_drawable(self.id, obj_id);
        let obj = if let Some(obj) = opt_obj {
            self.shift_obj_to_local_cords(obj)
        } else {
            return Err(DrawError::DrawableHandleNotFound {
                screen_id: self.id,
                obj_id,
            });
        };
        screen_buffer.remove_from_buffer(&obj, obj_id, sprites);
        Ok(())
    }

    pub fn render_all<T>(
        &mut self,
        screen_buffer: &mut T,
        obj_library: &DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        T: ScreenBuffer,
    {
        let obj_ids: Vec<DrawableId> = self.draw_objects.to_vec();
        for obj_id in obj_ids {
            self.render_drawable(obj_id, screen_buffer, obj_library, sprites)?;
        }
        Ok(())
    }
}
