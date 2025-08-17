use ascii_assets::AsciiVideo;
use common_stdx::{Point, Rect};
use log::info;
use std::collections::HashMap;

use crate::draw::ScreenBuffer;
use crate::draw::terminal_buffer::drawable::Drawable;
use crate::draw::{
    DrawError, DrawObject, DrawObjectLibrary, DrawableKey, FileError, Screen, ScreenKey,
    SpriteData, SpriteDrawable, SpriteEntry, SpriteRegistry, error::AppError,
    terminal_buffer::CrosstermScreenBuffer,
}; // bring the trait into scope

pub type SpriteId = usize;
pub type DrawableId = usize;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderMode {
    Instant,
    Buffered,
}
pub struct Renderer<B = CrosstermScreenBuffer>
where
    B: ScreenBuffer,
{
    screens: HashMap<ScreenKey, Screen>,
    obj_library: DrawObjectLibrary,
    screen_buffer: B,
    sprites: SpriteRegistry,
    render_mode: RenderMode,
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
    pub fn create_screen(&mut self, rect: Rect<u16>, layer: usize) -> ScreenKey {
        let new_id = self.generate_screen_key();
        self.screens
            .insert(new_id, Screen::new(rect, layer, new_id));
        new_id
    }

    /// Register a drawable object on a screen.
    pub fn register_drawable(
        &mut self,
        screen_id: ScreenKey,
        obj: DrawObject,
    ) -> Result<DrawableKey, DrawError> {
        if let Some(s) = self.screens.get_mut(&screen_id) {
            let new_obj_id = self.obj_library.add_obj(screen_id, obj);
            s.register_drawable(new_obj_id);
            Ok((screen_id, new_obj_id))
        } else {
            Err(DrawError::DisplayKeyNotFound(screen_id))
        }
    }

    pub fn remove_drawable(&mut self, id: DrawableKey) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&id.0) {
            s.remove_drawable(
                id.1,
                &mut self.screen_buffer,
                &self.obj_library,
                &self.sprites,
            )?;
            self.refresh(false)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(id.0))
        }
    }

    /// Register a sprite drawable on a screen.
    pub fn register_sprite_drawable(
        &mut self,
        screen_id: ScreenKey,
        layer: usize,
        position: Point<u16>,
        sprite_id: SpriteId,
    ) -> Result<DrawableKey, DrawError> {
        let obj = DrawObject {
            layer,
            drawable: Box::new(SpriteDrawable {
                position,
                sprite_id,
            }),
        };
        self.register_drawable(screen_id, obj)
    }

    /// Load a sprite from an ASCII video file.
    pub fn register_sprite_from_source(
        &mut self,
        path: &str,
        frame: Option<usize>,
    ) -> Result<SpriteId, AppError> {
        let video = AsciiVideo::read_from_file(path)?;
        let opt_sprite = if let Some(frame) = frame {
            video.frames.get(frame).cloned()
        } else {
            video.frames.first().cloned()
        };
        let sprite_id = if let Some(sprite) = opt_sprite {
            self.sprites.add(SpriteEntry {
                info: SpriteData::Sprite(sprite),
            })
        } else {
            return Err(AppError::File(FileError::VideoFrameNotFound {
                video_path: path.to_string(),
                frame_id: frame.unwrap_or(0),
            }));
        };
        Ok(sprite_id)
    }

    /// Render a single drawable object.
    pub fn render_drawable(&mut self, id: DrawableKey) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&id.0) {
            s.render_drawable(
                id.1,
                &mut self.screen_buffer,
                &self.obj_library,
                &self.sprites,
            )?;
            self.refresh(false)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(id.0))
        }
    }

    /// first removes the older version of the Drawable
    /// then replaces annd draws it to the terminalbuffer
    pub fn replace_drawable(
        &mut self,
        id: DrawableKey,
        new_obj: DrawObject,
    ) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&id.0) {
            s.remove_drawable(
                id.1,
                &mut self.screen_buffer,
                &self.obj_library,
                &self.sprites,
            )?;
            self.obj_library.update_drawable(id, new_obj);
            s.render_drawable(
                id.1,
                &mut self.screen_buffer,
                &self.obj_library,
                &self.sprites,
            )?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(id.0))
        }
    }
    /// Render all objects on all screens.
    fn render_all(&mut self) -> Result<(), DrawError> {
        for screen in self.screens.values_mut() {
            screen.render_all(&mut self.screen_buffer, &self.obj_library, &self.sprites)?;
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
        self.render_all()?;
        self.refresh(true)?;
        for screen in self.screens.values_mut() {
            screen.dump_after_frame();
        }
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

    fn get_drawable<F, T>(&self, handle: DrawableKey, mut get_fn: F) -> Result<T, DrawError>
    where
        F: FnMut(&dyn Drawable) -> Result<T, DrawError>,
    {
        if let Some(obj) = self.obj_library.find_drawable(handle.0, handle.1) {
            get_fn(&*obj.drawable)
        } else {
            Err(DrawError::DrawableHandleNotFound {
                screen_id: handle.0,
                obj_id: handle.1,
            })
        }
    }

    /// general update function
    fn update_drawable<F>(&mut self, handle: DrawableKey, mut update_fn: F) -> Result<(), DrawError>
    where
        F: FnMut(&mut dyn Drawable),
    {
        self.remove_drawable(handle)?;

        {
            let obj =
                self.obj_library
                    .get_mut(&handle)
                    .ok_or(DrawError::DrawableHandleNotFound {
                        screen_id: handle.0,
                        obj_id: handle.1,
                    })?;
            update_fn(&mut *obj.drawable);
        }

        if self.render_mode == RenderMode::Instant {
            self.render_drawable(handle)?;
        }

        Ok(())
    }

    pub fn move_drawable_to(
        &mut self,
        handle: DrawableKey,
        new_pos: Point<u16>,
    ) -> Result<(), DrawError> {
        self.update_drawable(handle, |drawable| {
            if let Some(dp) = drawable.as_double_pointed_mut() {
                let old_start = dp.start();
                let old_end = dp.end();
                let dx = new_pos.x as i32 - old_start.x as i32;
                let dy = new_pos.y as i32 - old_start.y as i32;
                dp.set_start(new_pos);
                dp.set_end(Point {
                    x: (old_end.x as i32 + dx).clamp(0, u16::MAX as i32) as u16,
                    y: (old_end.y as i32 + dy).clamp(0, u16::MAX as i32) as u16,
                });
            } else if let Some(sp) = drawable.as_single_pointed_mut() {
                sp.set_position(new_pos);
            }
        })
    }

    pub fn move_drawable_by(
        &mut self,
        handle: DrawableKey,
        dx: i32,
        dy: i32,
    ) -> Result<(), DrawError> {
        self.update_drawable(handle, |drawable| {
            if let Some(dp) = drawable.as_double_pointed_mut() {
                let start = dp.start();
                let end = dp.end();
                dp.set_start(Point {
                    x: (start.x as i32 + dx).clamp(0, u16::MAX as i32) as u16,
                    y: (start.y as i32 + dy).clamp(0, u16::MAX as i32) as u16,
                });
                dp.set_end(Point {
                    x: (end.x as i32 + dx).clamp(0, u16::MAX as i32) as u16,
                    y: (end.y as i32 + dy).clamp(0, u16::MAX as i32) as u16,
                });
            } else if let Some(sp) = drawable.as_single_pointed_mut() {
                let pos = sp.position();
                sp.set_position(Point {
                    x: (pos.x as i32 + dx).clamp(0, u16::MAX as i32) as u16,
                    y: (pos.y as i32 + dy).clamp(0, u16::MAX as i32) as u16,
                });
            }
        })
    }

    pub fn move_drawable_point(
        &mut self,
        handle: DrawableKey,
        point_index: usize,
        new_pos: Point<u16>,
    ) -> Result<(), DrawError> {
        self.update_drawable(handle, |drawable| {
            if let Some(dp) = drawable.as_double_pointed_mut() {
                let clamped = Point {
                    x: (new_pos.x as i32).clamp(0, u16::MAX as i32) as u16,
                    y: (new_pos.y as i32).clamp(0, u16::MAX as i32) as u16,
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
        handle: DrawableKey,
        point_index: usize,
        new_pos: Point<u16>,
    ) -> Result<(), DrawError> {
        self.update_drawable(handle, |drawable| {
            if let Some(mp) = drawable.as_multi_pointed_mut() {
                let clamped = Point {
                    x: (new_pos.x as i32).clamp(0, u16::MAX as i32) as u16,
                    y: (new_pos.y as i32).clamp(0, u16::MAX as i32) as u16,
                };
                mp.set_point(point_index, clamped);
            }
        })
    }
    /// replace a drawables points
    pub fn replace_drawable_points(
        &mut self,
        handle: DrawableKey,
        new_points: Vec<Point<u16>>,
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

    pub fn get_amount_of_points(&self, handle: DrawableKey) -> Result<Option<usize>, DrawError> {
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
