use std::collections::HashMap;

use log::info;

use crate::draw::{DrawableHandle, ScreenKey, terminal_buffer::Drawable};

pub type DrawableKey = (ScreenKey, DrawableHandle);

#[derive(Debug, Clone)]
pub struct DrawObject {
    pub layer: usize,
    pub drawable: Box<dyn Drawable>,
}

#[derive(Debug, Clone, Default)]
pub struct DrawObjectLibrary {
    all_objects: HashMap<(ScreenKey, DrawableHandle), DrawObject>,
}

impl DrawObjectLibrary {
    pub fn new() -> Self {
        info!("Creating a new DrawObjectLibrary");
        Self {
            all_objects: HashMap::new(),
        }
    }

    pub fn add_obj(&mut self, screen_id: ScreenKey, object: DrawObject) -> DrawableHandle {
        let new_id = self.generate_drawable_id();
        self.all_objects.insert((screen_id, new_id), object);
        new_id
    }

    pub fn update_drawable(&mut self, id: DrawableKey, new_object: DrawObject) {
        if self.all_objects.contains_key(&id) {
            self.all_objects.insert(id, new_object);
        }
    }

    pub fn find_drawable(
        &self,
        screen_id: ScreenKey,
        obj_id: DrawableHandle,
    ) -> Option<&DrawObject> {
        self.all_objects.get(&(screen_id, obj_id))
    }

    fn generate_drawable_id(&self) -> DrawableHandle {
        self.all_objects.len()
    }
}
