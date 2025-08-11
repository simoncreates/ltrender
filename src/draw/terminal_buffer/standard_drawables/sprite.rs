use std::collections::HashMap;

use common_stdx::{Point, Rect};

use crate::draw::{
    AllSprites, DrawError, SpriteId, SpriteObjectType, UpdateInterval,
    terminal_buffer::{
        Drawable,
        drawable::{BasicDraw, convert_rect_to_update_intervals},
    },
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SpriteDrawable {
    pub position: Point<u16>,
    pub sprite_id: SpriteId,
}

impl Drawable for SpriteDrawable {
    fn draw(&self, sprites: &AllSprites) -> Result<Vec<BasicDraw>, DrawError> {
        let sprite = match sprites.get(&self.sprite_id) {
            Some(s) => s,
            None => return Err(DrawError::SpriteNotFound(self.sprite_id)),
        };

        match &sprite.info {
            SpriteObjectType::Sprite(content) => {
                let width = content.width as usize;
                let origin_x = self.position.x;
                let origin_y = self.position.y;

                let mut out = Vec::with_capacity(content.pixels.len());
                for (i, ch) in content.pixels.iter().enumerate() {
                    let dx = (i % width) as u16;
                    let dy = (i / width) as u16;

                    let abs_x = origin_x.saturating_add(dx);
                    let abs_y = origin_y.saturating_add(dy);

                    out.push(BasicDraw {
                        pos: Point { x: abs_x, y: abs_y },
                        chr: *ch,
                    });
                }
                Ok(out)
            }
            SpriteObjectType::SpriteVideo(content) => {
                unimplemented!("Sprite video rendering not implemented yet");
            }
        }
    }

    fn bounding_iv(&self, sprites: &AllSprites) -> HashMap<u16, Vec<UpdateInterval>> {
        let size = sprites
            .get(&self.sprite_id)
            .map(|s| s.size())
            .unwrap_or((0, 0, 0));
        convert_rect_to_update_intervals(Rect {
            p1: self.position,
            p2: Point {
                x: self.position.x + size.1 as u16,
                y: self.position.y + size.2 as u16,
            },
        })
    }

    fn shifted(&self, offset: Point<u16>) -> Box<dyn Drawable> {
        Box::new(SpriteDrawable {
            position: self.position + offset,
            sprite_id: self.sprite_id,
        })
    }
}
