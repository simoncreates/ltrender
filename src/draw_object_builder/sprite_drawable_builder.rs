use common_stdx::Point;

use crate::{
    SpriteDrawable, SpriteId,
    error::DrawObjectBuilderError,
    terminal_buffer::{
        Drawable,
        standard_drawables::sprite_drawable::{AnimationInfo, FrameIdent},
    },
};

use crate::handle_pointed_field;

#[derive(Default)]
pub struct SpriteDrawableBuilder {
    position: Option<Point<i32>>,
    sprite_id: Option<SpriteId>,
    animation_type: Option<AnimationInfo>,
}

impl SpriteDrawableBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    handle_pointed_field!(position, position);

    pub fn sprite_id(mut self, id: SpriteId) -> Self {
        self.sprite_id = Some(id);
        self
    }

    pub fn animation_type(mut self, anim: AnimationInfo) -> Self {
        self.animation_type = Some(anim);
        self
    }

    pub fn build(self) -> Result<Box<dyn Drawable>, DrawObjectBuilderError> {
        Ok(Box::new(SpriteDrawable {
            position: self.position.unwrap_or(Point { x: 0, y: 0 }),
            sprite_id: self
                .sprite_id
                .ok_or(DrawObjectBuilderError::FailedToBuildSpriteObject())?,
            last_state_change: std::time::Instant::now(),
            animation_type: self.animation_type.unwrap_or(AnimationInfo::Image {
                frame: FrameIdent::FirstFrame,
            }),
        }))
    }
}
