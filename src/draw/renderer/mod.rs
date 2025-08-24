use ascii_assets::AsciiVideo;
use common_stdx::{Point, Rect};
use log::info;
use std::collections::HashMap;

use crate::draw::ScreenBuffer;
use crate::draw::drawable_register::ObjectLifetime;
use crate::draw::terminal_buffer::drawable::Drawable;
use crate::draw::terminal_buffer::standard_buffers::crossterm_buffer::DefaultScreenBuffer;
use crate::draw::{
    DrawError, DrawObject, DrawObjectKey, DrawObjectLibrary, Screen, ScreenKey, SpriteEntry,
    SpriteRegistry, error::AppError,
}; // bring the trait into scope

use std::thread;
use std::time::{Duration, Instant};
pub mod render_handle;
pub use render_handle::RendererHandle;

pub type SpriteId = usize;
pub type ObjectId = usize;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderMode {
    Instant,
    Buffered,
}
pub struct Renderer<B = DefaultScreenBuffer>
where
    B: ScreenBuffer,
{
    screens: HashMap<ScreenKey, Screen>,
    obj_library: DrawObjectLibrary,
    screen_buffer: B,
    sprites: SpriteRegistry,
    pub render_mode: RenderMode,
    update_interval_expand_amount: usize,
}

impl<B> Renderer<B>
where
    B: ScreenBuffer,
{
    pub fn set_render_mode(&mut self, mode: RenderMode) {
        self.render_mode = mode;
    }

    /// set how aggrisive draw batches should try to merge
    pub fn set_update_interval_expand_amount(&mut self, amount: usize) {
        self.update_interval_expand_amount = amount;
    }

    /// Create a new renderer with an initial terminal size.
    pub fn create_renderer(size: (u16, u16)) -> Self {
        Renderer {
            obj_library: DrawObjectLibrary::new(),
            screens: HashMap::new(),
            screen_buffer: B::new(size),
            sprites: SpriteRegistry::new(),
            render_mode: RenderMode::Buffered,
            update_interval_expand_amount: 50000,
        }
    }

    /// Create a new screen and return its key.
    pub fn create_screen(&mut self, rect: Rect<i32>, layer: usize) -> ScreenKey {
        let new_id = self.generate_screen_key();
        self.screens
            .insert(new_id, Screen::new(rect, layer, new_id));
        new_id
    }

    pub fn change_screen_area(
        &mut self,
        screen_id: ScreenKey,
        new_area: Rect<i32>,
    ) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&screen_id) {
            s.remove_all(
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;
            s.change_screen_area(new_area);
            s.render_all(
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;
            self.refresh(false)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(screen_id))
        }
    }

    pub fn change_screen_layer(
        &mut self,
        screen_id: ScreenKey,
        new_layer: usize,
    ) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&screen_id) {
            s.remove_all(
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;
            s.change_screen_layer(new_layer);
            self.refresh(false)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(screen_id))
        }
    }

    /// Register a drawable object on a screen.
    pub fn register_drawable(
        &mut self,
        screen_id: ScreenKey,
        obj: DrawObject,
    ) -> Result<DrawObjectKey, DrawError> {
        if let Some(s) = self.screens.get_mut(&screen_id) {
            let new_obj_id = self.obj_library.add_obj(screen_id, obj);
            s.register_drawable(new_obj_id);
            Ok(DrawObjectKey {
                screen_id,
                object_id: new_obj_id,
            })
        } else {
            Err(DrawError::DisplayKeyNotFound(screen_id))
        }
    }

    pub fn remove_drawable(&mut self, id: DrawObjectKey) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&id.screen_id) {
            s.remove_drawable(
                id.object_id,
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;
            self.refresh(false)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(id.screen_id))
        }
    }

    pub fn explicit_remove_drawable(&mut self, id: &DrawObjectKey) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&id.screen_id) {
            s.remove_drawable(
                id.object_id,
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;
            s.deregister_drawable(id.object_id);
            self.refresh(false)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(id.screen_id))
        }
    }

    /// checks, if any of the currently existing drawobjects should be removed,
    /// because its duration on screen has ended
    pub fn check_if_object_lifetime_ended(&mut self) -> Result<(), DrawError> {
        let now = Instant::now();

        let expired_keys: Vec<DrawObjectKey> = self
            .obj_library
            .all_objects
            .iter()
            .filter_map(|(k, obj)| {
                if let ObjectLifetime::ForTime(dur) = obj.lifetime {
                    if now >= obj.creation_time + dur {
                        Some(*k)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        for key in expired_keys {
            self.explicit_remove_drawable(&key)?;
        }

        Ok(())
    }

    /// removes all the objects from screen, that have framebased removal, due to their lifetime
    pub fn remove_all_framebased_objects(&mut self) -> Result<(), DrawError> {
        let expired_keys: Vec<DrawObjectKey> = self
            .obj_library
            .all_objects
            .iter()
            .filter_map(|(k, obj)| {
                if let ObjectLifetime::RemoveNextFrame = obj.lifetime {
                    Some(*k)
                } else {
                    None
                }
            })
            .collect();

        for key in expired_keys {
            self.explicit_remove_drawable(&key)?;
        }

        Ok(())
    }

    /// Load a sprite from an ASCII video file.
    pub fn register_sprite_from_source(&mut self, path: &str) -> Result<SpriteId, AppError> {
        let video = AsciiVideo::read_from_file(path)?;

        let sprite_id = self.sprites.add(SpriteEntry { info: video });

        Ok(sprite_id)
    }

    /// Render a single drawable object.
    pub fn render_drawable(&mut self, id: DrawObjectKey) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&id.screen_id) {
            s.register_drawable(id.object_id);
            s.render_drawable(
                id.object_id,
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;

            self.refresh(false)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(id.screen_id))
        }
    }
    /// Render all objects on all screens.
    fn render_all(&mut self) -> Result<(), DrawError> {
        for screen in self.screens.values_mut() {
            screen.render_all(
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;
        }
        self.refresh(false)?;
        Ok(())
    }

    pub fn handle_resize(&mut self, new_size: (u16, u16)) -> Result<(), DrawError> {
        self.screen_buffer = B::new(new_size);
        B::mark_all_dirty(&mut self.screen_buffer, new_size);
        self.render_all()?;
        Ok(())
    }

    /// Flush the buffer to the terminal.
    pub fn refresh(&mut self, forced: bool) -> Result<(), DrawError> {
        if RenderMode::Instant == self.render_mode || forced {
            B::update_terminal(&mut self.screen_buffer, self.update_interval_expand_amount)?;
        }
        Ok(())
    }

    pub fn render_frame(&mut self) -> Result<(), DrawError> {
        self.remove_all_framebased_objects()?;
        self.render_all()?;

        self.refresh(true)?;
        Ok(())
    }

    /// Generate a unique screen key.
    pub fn generate_screen_key(&self) -> ScreenKey {
        let mut id = self.screens.len();
        while self.screens.contains_key(&id) {
            id = id.saturating_add(1);
        }
        id
    }

    pub fn clear_terminal(&mut self) {
        for screen in self.screens.values_mut() {
            screen.switch_obj_ids_after_frame();
        }
    }

    fn get_drawable<F, T>(&self, object_key: DrawObjectKey, mut get_fn: F) -> Result<T, DrawError>
    where
        F: FnMut(&dyn Drawable) -> Result<T, DrawError>,
    {
        if let Some(obj) = self.obj_library.find_drawable(&object_key) {
            get_fn(&*obj.drawable)
        } else {
            Err(DrawError::DrawableHandleNotFound {
                screen_id: object_key.screen_id,
                obj_id: object_key.object_id,
            })
        }
    }

    /// general update function
    fn update_drawable<F>(
        &mut self,
        object_key: DrawObjectKey,
        mut update_fn: F,
    ) -> Result<(), DrawError>
    where
        F: FnMut(&mut dyn Drawable),
    {
        self.remove_drawable(object_key)?;

        {
            let obj =
                self.obj_library
                    .get_mut(&object_key)
                    .ok_or(DrawError::DrawableHandleNotFound {
                        screen_id: object_key.screen_id,
                        obj_id: object_key.object_id,
                    })?;
            update_fn(&mut *obj.drawable);
        }

        if self.render_mode == RenderMode::Instant {
            self.render_drawable(object_key)?;
        }

        Ok(())
    }

    pub fn move_drawable_to(
        &mut self,
        handle: DrawObjectKey,
        new_pos: Point<i32>,
    ) -> Result<(), DrawError> {
        self.update_drawable(handle, |drawable| {
            if let Some(dp) = drawable.as_double_pointed_mut() {
                let old_start = dp.start();
                let old_end = dp.end();
                let dx = new_pos.x - old_start.x;
                let dy = new_pos.y - old_start.y;
                dp.set_start(new_pos);
                dp.set_end(Point {
                    x: (old_end.x + dx).clamp(0, u16::MAX as i32),
                    y: (old_end.y + dy).clamp(0, u16::MAX as i32),
                });
            } else if let Some(sp) = drawable.as_single_pointed_mut() {
                sp.set_position(new_pos);
            }
        })
    }

    pub fn move_drawable_by(
        &mut self,
        handle: DrawObjectKey,
        dx: i32,
        dy: i32,
    ) -> Result<(), DrawError> {
        self.update_drawable(handle, |drawable| {
            if let Some(dp) = drawable.as_double_pointed_mut() {
                let start = dp.start();
                let end = dp.end();
                dp.set_start(Point {
                    x: (start.x + dx).clamp(0, i32::MAX),
                    y: (start.y + dy).clamp(0, i32::MAX),
                });
                dp.set_end(Point {
                    x: (end.x + dx).clamp(0, i32::MAX),
                    y: (end.y + dy).clamp(0, i32::MAX),
                });
            } else if let Some(sp) = drawable.as_single_pointed_mut() {
                let pos = sp.position();
                sp.set_position(Point {
                    x: (pos.x + dx).clamp(0, i32::MAX),
                    y: (pos.y + dy).clamp(0, i32::MAX),
                });
            }
        })
    }

    pub fn move_drawable_point(
        &mut self,
        handle: DrawObjectKey,
        point_index: usize,
        new_pos: Point<i32>,
    ) -> Result<(), DrawError> {
        self.update_drawable(handle, |drawable| {
            if let Some(dp) = drawable.as_double_pointed_mut() {
                let clamped = Point {
                    x: (new_pos.x).clamp(0, i32::MAX),
                    y: (new_pos.y).clamp(0, i32::MAX),
                };
                match point_index {
                    0 => dp.set_start(clamped),
                    1 => dp.set_end(clamped),
                    _ => {}
                }
            }
        })
    }
    pub fn move_multipoint_drawable_point(
        &mut self,
        handle: DrawObjectKey,
        point_index: usize,
        new_pos: Point<i32>,
    ) -> Result<(), DrawError> {
        self.update_drawable(handle, |drawable| {
            if let Some(mp) = drawable.as_multi_pointed_mut() {
                let clamped = Point {
                    x: (new_pos.x).clamp(0, i32::MAX),
                    y: (new_pos.y).clamp(0, i32::MAX),
                };
                mp.set_point(point_index, clamped);
            }
        })
    }
    /// replace a drawables points
    pub fn replace_drawable_points(
        &mut self,
        handle: DrawObjectKey,
        new_points: Vec<Point<i32>>,
    ) -> Result<(), DrawError> {
        self.update_drawable(handle, |drawable| {
            if let Some(mp) = drawable.as_multi_pointed_mut() {
                mp.set_points(new_points.clone());
            } else if let Some(dp) = drawable.as_double_pointed_mut() {
                if new_points.len() == 2 {
                    dp.set_start(new_points[0]);
                    dp.set_end(new_points[1]);
                } else {
                    info!(
                        "Expected exactly two points for double-pointed drawable, got {}",
                        new_points.len()
                    );
                }
            } else if new_points.len() == 1 {
                if let Some(sp) = drawable.as_single_pointed_mut() {
                    sp.set_position(new_points[0]);
                } else {
                    info!("Drawable is not single-pointed");
                }
            } else {
                info!(
                    "Expected exactly one point for single-pointed drawable, got {}",
                    new_points.len()
                );
            }
        })
    }

    pub fn get_amount_of_points(&self, handle: DrawObjectKey) -> Result<Option<usize>, DrawError> {
        self.get_drawable(handle, |drawable| {
            if let Some(mp) = drawable.as_multi_pointed() {
                Ok(Some(mp.points().len()))
            } else {
                info!("Drawable is not multipointed");
                Ok(None)
            }
        })
    }
}
