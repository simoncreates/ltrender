use std::i32;

use crate::{
    DrawError, DrawObjectKey, DrawObjectLibrary, ObjectId, ScreenBuffer, SpriteRegistry,
    terminal_buffer::CellDrawer,
};
use common_stdx::{Point, Rect};
pub type ScreenKey = usize;

pub mod area_rect;
pub use area_rect::{AreaPoint, AreaRect};

#[derive(Debug)]
pub struct Screen {
    layer: usize,
    id: ScreenKey,
    area: AreaRect,
    pub terminal_size: (u16, u16),
    pub draw_objects: Vec<ObjectId>,
}

impl Screen {
    pub fn new(area: AreaRect, layer: usize, id: ScreenKey, terminal_size: (u16, u16)) -> Self {
        Screen {
            layer,
            id,
            area,
            terminal_size,
            draw_objects: Vec::new(),
        }
    }

    pub fn change_screen_area(&mut self, new_area: AreaRect) {
        self.area = new_area;
    }

    /// sets the screens area to fit its contents
    pub fn change_screen_area_contents(
        &mut self,
        obj_library: &mut DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) {
        let mut combined_bbox: Option<Rect<i32>> = None;

        for object_id in &self.draw_objects {
            let key = DrawObjectKey {
                screen_id: self.id,
                object_id: *object_id,
            };

            let obj = match obj_library.get_mut(&key) {
                Some(obj) => obj,
                None => continue,
            };

            let bdc = match obj.drawable.draw(sprites) {
                Ok(bdc) => bdc,
                Err(_) => continue,
            };

            let bbox = bdc.get_bounding_box().normalized();

            combined_bbox = Some(match combined_bbox {
                None => bbox,
                Some(acc) => Rect {
                    p1: Point {
                        x: acc.p1.x.min(bbox.p1.x),
                        y: acc.p1.y.min(bbox.p1.y),
                    },
                    p2: Point {
                        x: acc.p2.x.max(bbox.p2.x),
                        y: acc.p2.y.max(bbox.p2.y),
                    },
                },
            });
        }

        if let Some(bbox) = combined_bbox {
            self.area = AreaRect::FromPoints(AreaPoint::Point(bbox.p1), AreaPoint::Point(bbox.p2))
        }
    }
    pub fn change_screen_layer(&mut self, new_layer: usize) {
        self.layer = new_layer;
    }

    pub fn layer(&self) -> usize {
        self.layer
    }

    /// register a drawable, so it can be drawn on this screen
    pub fn register_drawable(&mut self, obj_id: ObjectId, obj_library: &DrawObjectLibrary) {
        if !self.draw_objects.contains(&obj_id)
            && obj_library.all_objects.contains_key(&DrawObjectKey {
                screen_id: self.id,
                object_id: obj_id,
            })
        {
            self.draw_objects.push(obj_id);
        }
    }

    /// Remove a drawable from the screen, if it exists
    pub fn deregister_drawable(&mut self, obj_id: ObjectId) {
        if let Some(pos) = self.draw_objects.iter().position(|&id| id == obj_id) {
            self.draw_objects.remove(pos);
        }
    }
    /// render a drawable to the screen
    pub fn render_drawable<B>(
        &mut self,
        object_id: ObjectId,
        screen_buffer: &mut B,
        obj_library: &mut DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        B: ScreenBuffer,
        B::Drawer: CellDrawer,
    {
        if !self.draw_objects.contains(&object_id) {
            return Ok(());
        }

        let opt_obj = obj_library.get_mut(&DrawObjectKey {
            screen_id: self.id,
            object_id,
        });
        if let Some(obj) = opt_obj {
            let rect = &self.area.area_to_rect(&self.terminal_size);
            screen_buffer.add_to_buffer(obj, object_id, self.layer, rect, sprites)?;
        } else {
            return Err(DrawError::DrawableHandleNotFound {
                screen_id: self.id,
                obj_id: object_id,
            });
        };

        Ok(())
    }

    pub fn rect(&self) -> Rect<i32> {
        self.area.area_to_rect(&self.terminal_size)
    }

    /// Remove a drawable object from the screen.
    pub fn remove_drawable<B>(
        &mut self,
        object_id: ObjectId,
        screen_buffer: &mut B,
        obj_library: &mut DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        B: ScreenBuffer,
        B::Drawer: CellDrawer,
    {
        let opt_obj = obj_library.get_mut(&DrawObjectKey {
            screen_id: self.id,
            object_id,
        });
        if let Some(obj) = opt_obj {
            let rect = &self.area.area_to_rect(&self.terminal_size);
            screen_buffer.remove_from_buffer(obj, object_id, sprites, rect);
        } else {
            return Err(DrawError::DrawableHandleNotFound {
                screen_id: self.id,
                obj_id: object_id,
            });
        };
        Ok(())
    }

    pub fn remove_all<B>(
        &mut self,
        screen_buffer: &mut B,
        obj_library: &mut DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        B: ScreenBuffer,
        B::Drawer: CellDrawer,
    {
        let obj_ids: Vec<_> = self.draw_objects.to_vec();
        for obj_id in obj_ids {
            self.remove_drawable(obj_id, screen_buffer, obj_library, sprites)?;
        }

        Ok(())
    }

    /// uncoditionally renders all owned drawables
    pub fn render_all<B>(
        &mut self,
        screen_buffer: &mut B,
        obj_library: &mut DrawObjectLibrary,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError>
    where
        B: ScreenBuffer,
        B::Drawer: CellDrawer,
    {
        let obj_ids: Vec<_> = self.draw_objects.to_vec();
        for obj_id in obj_ids {
            self.render_drawable(obj_id, screen_buffer, obj_library, sprites)?;
        }
        Ok(())
    }
}
