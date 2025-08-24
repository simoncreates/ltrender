use crate::draw::{
    ObjectId, ScreenKey,
    terminal_buffer::{Drawable, screen_buffer::Shader},
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct DrawObjectKey {
    pub screen_id: ScreenKey,
    pub object_id: ObjectId,
}

#[derive(Debug)]

pub enum ObjectLifetime {
    RemoveNextFrame,
    ExplicitRemove,
    ForTime(Duration),
}

#[derive(Debug)]
pub struct DrawObject {
    pub lifetime: ObjectLifetime,
    pub creation_time: Instant,
    pub layer: usize,
    pub shaders: Vec<Box<dyn Shader>>,
    pub drawable: Box<dyn Drawable + 'static>,
}

#[derive(Debug, Default)]
pub struct DrawObjectLibrary {
    pub all_objects: HashMap<DrawObjectKey, DrawObject>,
}

impl DrawObjectLibrary {
    pub fn new() -> Self {
        Self {
            all_objects: HashMap::new(),
        }
    }

    pub fn add_obj(&mut self, screen_id: ScreenKey, object: DrawObject) -> ObjectId {
        let new_id = self.generate_drawable_id();
        self.all_objects.insert(
            DrawObjectKey {
                screen_id,
                object_id: new_id,
            },
            object,
        );
        new_id
    }

    pub fn update_drawable(&mut self, id: DrawObjectKey, new_object: DrawObject) {
        self.all_objects.insert(id, new_object);
    }

    pub fn find_drawable(&self, key: &DrawObjectKey) -> Option<&DrawObject> {
        self.all_objects.get(key)
    }

    pub fn get_mut(&mut self, id: &DrawObjectKey) -> Option<&mut DrawObject> {
        self.all_objects.get_mut(id)
    }

    fn generate_drawable_id(&self) -> ObjectId {
        self.all_objects.len()
    }
}
