use crate::draw::{DrawableId, ScreenKey, terminal_buffer::Drawable};
use std::collections::HashMap;

pub type DrawableKey = (ScreenKey, DrawableId);

#[derive(Debug, Clone)]
pub struct DrawObject {
    pub layer: usize,
    pub drawable: Box<dyn Drawable + 'static>,
}

#[derive(Debug, Clone, Default)]
pub struct DrawObjectLibrary {
    pub all_objects: HashMap<(ScreenKey, DrawableId), DrawObject>,
}

impl DrawObjectLibrary {
    pub fn new() -> Self {
        Self {
            all_objects: HashMap::new(),
        }
    }

    pub fn add_obj(&mut self, screen_id: ScreenKey, object: DrawObject) -> DrawableId {
        let new_id = self.generate_drawable_id();
        self.all_objects.insert((screen_id, new_id), object);
        new_id
    }

    pub fn update_drawable(&mut self, id: DrawableKey, new_object: DrawObject) {
        self.all_objects.insert(id, new_object);
    }

    pub fn find_drawable(&self, screen_id: ScreenKey, obj_id: DrawableId) -> Option<&DrawObject> {
        self.all_objects.get(&(screen_id, obj_id))
    }

    pub fn get_mut(&mut self, id: &DrawableKey) -> Option<&mut DrawObject> {
        self.all_objects.get_mut(id)
    }

    fn generate_drawable_id(&self) -> DrawableId {
        self.all_objects.len()
    }
}
