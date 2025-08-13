use std::{any::Any, cell::RefCell, collections::HashMap};

use common_stdx::{Point, Rect};

use crate::draw::{
    DrawError, SpriteData, SpriteId, SpriteRegistry, UpdateInterval,
    terminal_buffer::{
        Drawable,
        drawable::{BasicDraw, convert_rect_to_update_intervals},
    },
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SpriteDrawable {
    pub position: RefCell<Point<u16>>,
    pub sprite_id: RefCell<SpriteId>,
}

impl Drawable for SpriteDrawable {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn draw(&self, sprites: &SpriteRegistry) -> Result<Vec<BasicDraw>, DrawError> {
        let sprite = sprites
            .get(&self.sprite_id.borrow())
            .ok_or(DrawError::SpriteNotFound(self.sprite_id.borrow().clone()))?;

        match &sprite.info {
            SpriteData::Sprite(content) => {
                let width = content.width as usize;
                let origin_x = self.position.borrow().x;
                let origin_y = self.position.borrow().y;

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
            SpriteData::SpriteVideo(_content) => {
                unimplemented!("Sprite video rendering not implemented yet");
            }
        }
    }

    fn bounding_iv(&self, sprites: &SpriteRegistry) -> HashMap<u16, Vec<UpdateInterval>> {
        let size = sprites
            .get(&self.sprite_id.borrow())
            .map(|s| s.size())
            .unwrap_or((0, 0, 0));
        convert_rect_to_update_intervals(Rect {
            p1: self.position.borrow().clone(),
            p2: Point {
                x: self.position.borrow().x + size.1 as u16,
                y: self.position.borrow().y + size.2 as u16,
            },
        })
    }

    fn shifted(&self, offset: Point<u16>) -> Box<dyn Drawable> {
        Box::new(SpriteDrawable {
            position: RefCell::new(*self.position.borrow() + offset),
            sprite_id: RefCell::new(*self.sprite_id.borrow()),
        })
    }
}
