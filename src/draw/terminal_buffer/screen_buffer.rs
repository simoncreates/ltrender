use common_stdx::Rect;

use crate::draw::{DrawError, DrawObject, DrawableHandle, SpriteRegistry};

pub trait ScreenBuffer: std::fmt::Debug {
    fn new(size: (u16, u16)) -> Self
    where
        Self: Sized;

    fn add_to_buffer(
        &mut self,
        obj: &DrawObject,
        obj_id: DrawableHandle,
        screen_layer: usize,
        screen_bounds: &Rect<u16>,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>;
    fn remove_from_buffer(
        &mut self,
        obj: &DrawObject,
        obj_id: DrawableHandle,
        screen_bounds: &Rect<u16>,
        sprites: &SpriteRegistry,
    );

    fn update_terminal(&mut self) -> Result<(), DrawError>;

    fn mark_all_dirty(&mut self, new_size: (u16, u16));
}
