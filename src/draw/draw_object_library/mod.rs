use std::collections::HashMap;

use crate::draw::{ObjId, ScreenId, terminal_buffer::Drawable};

pub type ObjIdent = (ScreenId, ObjId);

#[derive(Debug, Clone)]
pub struct DrawObject {
    pub layer: usize,
    pub drawable: Box<dyn Drawable>,
}

#[derive(Debug, Clone)]
pub struct DrawObjectLibrary {
    all_objects: HashMap<(ScreenId, ObjId), DrawObject>,
}

impl Default for DrawObjectLibrary {
    fn default() -> Self {
        DrawObjectLibrary {
            all_objects: HashMap::new(),
        }
    }
}

impl DrawObjectLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_obj(&mut self, screen_id: ScreenId, object: DrawObject) -> ObjId {
        let new_id = self.new_obj_id();
        self.all_objects.insert((screen_id, new_id), object);
        new_id
    }

    pub fn replace_object(&mut self, id: ObjIdent, new_object: DrawObject) {
        if self.all_objects.contains_key(&id) {
            self.all_objects.insert(id, new_object);
        }
    }

    pub fn get(&self, screen_id: ScreenId, obj_id: ObjId) -> Option<&DrawObject> {
        self.all_objects.get(&(screen_id, obj_id))
    }

    fn new_obj_id(&self) -> ObjId {
        self.all_objects.len()
    }
}
