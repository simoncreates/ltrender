use crate::draw::{
    DrawError, DrawObjectKey, DrawObjectLibrary, ObjectId, ScreenBuffer, SpriteRegistry,
};
use common_stdx::Rect;
pub type ScreenKey = usize;
use std::mem;

#[derive(Debug)]
pub struct Screen {
    layer: usize,
    id: ScreenKey,
    pub area: Rect<i32>,
    draw_objects: Vec<ObjectId>,
}

impl Screen {
    pub fn new(area: Rect<i32>, layer: usize, id: ScreenKey) -> Self {
        Screen {
            layer,
            id,
            area,
            draw_objects: Vec::new(),
        }
    }

    pub fn change_screen_area(&mut self, new_area: Rect<i32>) {
        self.area = new_area;
    }

    pub fn change_screen_layer(&mut self, new_layer: usize) {
        self.layer = new_layer;
    }

    /// register a drawable, so it can be drawn on this screen
    pub fn register_drawable(&mut self, obj_id: ObjectId) {
        if !self.draw_objects.contains(&obj_id) {
            self.draw_objects.push(obj_id);
        }
    }

    /// Remove a drawable from the screen, if it exists
    pub fn deregister_drawable(&mut self, obj_id: ObjectId) {
        if let Some(pos) = self.draw_objects.iter().position(|&id| id == obj_id) {
            self.draw_objects.remove(pos);
        }
    }
    /// render a drawable to the screen
    pub fn render_drawable<T>(
        &mut self,
        object_id: ObjectId,
        screen_buffer: &mut T,
        obj_library: &mut DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        T: ScreenBuffer,
    {
        self.register_drawable(object_id);

        let opt_obj = obj_library.get_mut(&DrawObjectKey {
            screen_id: self.id,
            object_id,
        });
        if let Some(obj) = opt_obj {
            screen_buffer.add_to_buffer(obj, object_id, self.layer, &self.area, sprites)?;
        } else {
            return Err(DrawError::DrawableHandleNotFound {
                screen_id: self.id,
                obj_id: object_id,
            });
        };

        Ok(())
    }
    /// Remove a drawable object from the screen.
    pub fn remove_drawable<T>(
        &mut self,
        object_id: ObjectId,
        screen_buffer: &mut T,
        obj_library: &mut DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        T: ScreenBuffer,
    {
        let opt_obj = obj_library.get_mut(&DrawObjectKey {
            screen_id: self.id,
            object_id,
        });
        if let Some(obj) = opt_obj {
            screen_buffer.remove_from_buffer(obj, object_id, sprites, &self.area);
        } else {
            return Err(DrawError::DrawableHandleNotFound {
                screen_id: self.id,
                obj_id: object_id,
            });
        };
        Ok(())
    }

    pub fn remove_all<T>(
        &mut self,
        screen_buffer: &mut T,
        obj_library: &mut DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        T: ScreenBuffer,
    {
        let obj_ids: Vec<_> = self.draw_objects.to_vec();
        for obj_id in obj_ids {
            self.remove_drawable(obj_id, screen_buffer, obj_library, sprites)?;
        }

        Ok(())
    }

    pub fn switch_obj_ids_after_frame(&mut self) {
        self.draw_objects = mem::take(&mut self.draw_objects);
    }
    /// uncoditionally renders all owned drawables
    pub fn render_all<T>(
        &mut self,
        screen_buffer: &mut T,
        obj_library: &mut DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        T: ScreenBuffer,
    {
        let obj_ids: Vec<ObjectId> = self.draw_objects.to_vec();
        for obj_id in obj_ids {
            self.render_drawable(obj_id, screen_buffer, obj_library, sprites)?;
        }
        Ok(())
    }
}
