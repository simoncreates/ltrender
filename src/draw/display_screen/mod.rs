use crate::draw::{
    DrawError, DrawObject, DrawObjectKey, DrawObjectLibrary, ObjectId, ScreenBuffer, SpriteRegistry,
};
use common_stdx::Rect;
pub type ScreenKey = usize;

#[derive(Debug)]
pub struct Screen {
    layer: usize,
    id: ScreenKey,
    area: Rect<i32>,
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

    fn shift_obj_to_local_cords(&self, obj: &DrawObject) -> DrawObject {
        let shifted_drawable = obj.drawable.shifted(self.area.p1);
        DrawObject {
            layer: obj.layer,
            drawable: shifted_drawable,
            shaders: obj.shaders.clone(),
        }
    }

    /// register a drawable, so it can be drawn on this screen
    pub fn register_drawable(&mut self, obj_id: ObjectId) {
        self.draw_objects.push(obj_id);
    }

    /// render a drawable to the screen
    pub fn render_drawable<T>(
        &mut self,
        object_id: ObjectId,
        screen_buffer: &mut T,
        obj_library: &DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        T: ScreenBuffer,
    {
        let opt_obj = obj_library.find_drawable(&DrawObjectKey {
            screen_id: self.id,
            object_id,
        });
        let mut obj = if let Some(obj) = opt_obj {
            self.shift_obj_to_local_cords(obj)
        } else {
            return Err(DrawError::DrawableHandleNotFound {
                screen_id: self.id,
                obj_id: object_id,
            });
        };
        screen_buffer.add_to_buffer(&mut obj, object_id, self.layer, &self.area, sprites)?;
        Ok(())
    }

    /// Remove a drawable object from the screen.
    pub fn remove_drawable<T>(
        &mut self,
        object_id: ObjectId,
        screen_buffer: &mut T,
        obj_library: &DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        T: ScreenBuffer,
    {
        let opt_obj = obj_library.find_drawable(&DrawObjectKey {
            screen_id: self.id,
            object_id,
        });
        let obj = if let Some(obj) = opt_obj {
            self.shift_obj_to_local_cords(obj)
        } else {
            return Err(DrawError::DrawableHandleNotFound {
                screen_id: self.id,
                obj_id: object_id,
            });
        };
        screen_buffer.remove_from_buffer(&obj, object_id, sprites);
        Ok(())
    }

    /// remembers all the old drawables, so later diffing can be done
    pub fn dump_after_frame(&mut self) {
        self.draw_objects.clear();
    }

    /// uncoditionally renders all owned drawables
    pub fn render_all<T>(
        &mut self,
        screen_buffer: &mut T,
        obj_library: &DrawObjectLibrary,
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
