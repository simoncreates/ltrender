use crate::draw::{AllSprites, DrawError, DrawObject, DrawObjectLibrary, ObjId, ScreenBuffer};
use common_stdx::Rect;

pub type ScreenId = usize;

#[derive(Debug)]
pub struct Screen {
    layer: usize,
    id: ScreenId,
    area: Rect<u16>,
    draw_objects: Vec<ObjId>,
}

impl Screen {
    pub fn new(area: Rect<u16>, layer: usize, id: ScreenId) -> Self {
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
        }
    }

    pub fn add_object(&mut self, obj_id: ObjId) {
        self.draw_objects.push(obj_id);
    }

    pub fn draw_object(
        &mut self,
        obj_id: ObjId,
        screen_buffer: &mut ScreenBuffer,
        obj_library: &DrawObjectLibrary,
        sprites: &AllSprites,
    ) -> Result<(), DrawError> {
        let opt_obj = obj_library.get(self.id, obj_id);
        let obj = if let Some(obj) = opt_obj {
            self.shift_obj_to_local_cords(obj)
        } else {
            return Err(DrawError::ObjectNotFound {
                screen_id: self.id,
                obj_id,
            });
        };
        screen_buffer.add_to_buffer(&obj, obj_id, self.layer, &self.area, sprites)?;
        Ok(())
    }
}
