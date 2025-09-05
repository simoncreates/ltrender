use crate::{
    DrawError, DrawObjectKey, DrawObjectLibrary, ObjectId, ScreenBuffer, SpriteRegistry,
    terminal_buffer::CellDrawer,
};
use common_stdx::{Point, Rect};
pub type ScreenKey = usize;

#[derive(Debug, Clone, Copy)]
pub enum ScreenPoint {
    Point(Point<i32>),
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}
#[derive(Debug, Clone, Copy)]
pub enum ScreenAreaRect {
    FromPoints(ScreenPoint, ScreenPoint),
    FullScreen,
}

#[derive(Debug)]
pub struct Screen {
    layer: usize,
    id: ScreenKey,
    area: ScreenAreaRect,
    pub terminal_size: (u16, u16),
    pub draw_objects: Vec<ObjectId>,
}

impl Screen {
    pub fn new(
        area: ScreenAreaRect,
        layer: usize,
        id: ScreenKey,
        terminal_size: (u16, u16),
    ) -> Self {
        Screen {
            layer,
            id,
            area,
            terminal_size,
            draw_objects: Vec::new(),
        }
    }

    pub fn change_screen_area(&mut self, new_area: ScreenAreaRect) {
        self.area = new_area;
    }

    pub fn change_screen_layer(&mut self, new_layer: usize) {
        self.layer = new_layer;
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
            let rect = self.area_to_rect(&self.terminal_size, &self.area);
            screen_buffer.add_to_buffer(obj, object_id, self.layer, &rect, sprites)?;
        } else {
            return Err(DrawError::DrawableHandleNotFound {
                screen_id: self.id,
                obj_id: object_id,
            });
        };

        Ok(())
    }

    fn area_to_rect(&self, terminal_size: &(u16, u16), area: &ScreenAreaRect) -> Rect<i32> {
        match area {
            ScreenAreaRect::FromPoints(p1, p2) => Rect {
                p1: self.screen_point_to_point(terminal_size, *p1),
                p2: self.screen_point_to_point(terminal_size, *p2),
            },
            ScreenAreaRect::FullScreen => Rect {
                p1: self.screen_point_to_point(terminal_size, ScreenPoint::TopLeft),
                p2: self.screen_point_to_point(terminal_size, ScreenPoint::BottomRight),
            },
        }
    }

    fn screen_point_to_point(
        &self,
        terminal_size: &(u16, u16),
        screen_point: ScreenPoint,
    ) -> Point<i32> {
        match screen_point {
            ScreenPoint::Point(p) => p,
            ScreenPoint::TopLeft => Point::from((0, 0)),
            ScreenPoint::TopRight => Point::from((terminal_size.0 as i32, 0)),
            ScreenPoint::BottomLeft => Point::from((0, terminal_size.1 as i32)),
            ScreenPoint::BottomRight => {
                Point::from((terminal_size.0 as i32, terminal_size.1 as i32))
            }
        }
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
            let rect = self.area_to_rect(&self.terminal_size, &self.area);
            screen_buffer.remove_from_buffer(obj, object_id, sprites, &rect);
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
