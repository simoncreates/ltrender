use ascii_assets::AsciiVideo;
use common_stdx::{Point, Rect};
use log::info;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::draw::ScreenBuffer;
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
    pub sprites: SpriteRegistry,
    pub render_mode: RenderMode,
}

impl<B> Renderer<B>
where
    B: ScreenBuffer,
{
    pub fn set_render_mode(&mut self, mode: RenderMode) {
        self.render_mode = mode;
    }

    /// Create a new renderer with an initial terminal size.
    pub fn create_renderer(size: (u16, u16)) -> Self {
        Renderer {
            obj_library: DrawObjectLibrary::new(),
            screens: HashMap::new(),
            screen_buffer: B::new(size),
            sprites: SpriteRegistry::new(),
            render_mode: RenderMode::Buffered,
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
            drawable: Arc::new(Mutex::new(Box::new(SpriteDrawable {
                position: RefCell::new(position),
                sprite_id: RefCell::new(sprite_id),
            }))),
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
        if RenderMode::Buffered == self.render_mode || forced {
            B::update_terminal(&mut self.screen_buffer)?;
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

    pub fn move_drawable_to(
        &mut self,
        handle: DrawableKey,
        new_pos: Point<u16>,
    ) -> Result<(), DrawError> {
        self.remove_drawable(handle)?;
        {
            let obj =
                self.obj_library
                    .get_mut(&handle)
                    .ok_or(DrawError::DrawableHandleNotFound {
                        screen_id: handle.0,
                        obj_id: handle.1,
                    })?;

            let mut guard = obj
                .drawable
                .lock()
                .map_err(|_| DrawError::DrawableLockFailed)?;

            let drawable_obj: &mut dyn crate::draw::terminal_buffer::drawable::Drawable =
                &mut **guard;

            if let Some(dp) = drawable_obj.as_double_pointed_mut() {
                let old_start = dp.start();
                let old_end = dp.end();

                let ox = old_start.x as i32;
                let oy = old_start.y as i32;
                let nx = new_pos.x as i32;
                let ny = new_pos.y as i32;

                let dx = nx - ox;
                let dy = ny - oy;

                dp.set_start(new_pos);
                let new_end = Point {
                    x: (old_end.x as i32 + dx).clamp(0, u16::MAX as i32) as u16,
                    y: (old_end.y as i32 + dy).clamp(0, u16::MAX as i32) as u16,
                };
                dp.set_end(new_end);
            } else if let Some(sp) = drawable_obj.as_single_pointed_mut() {
                sp.set_position(new_pos);
            } else {
                info!(
                    "Drawable {:?} does not implement a point trait; nothing moved",
                    handle
                );
            }
        }
        Ok(())
    }

    pub fn move_drawable_by(
        &mut self,
        handle: DrawableKey,
        dx: i32,
        dy: i32,
    ) -> Result<(), DrawError> {
        self.remove_drawable(handle)?;
        {
            let obj =
                self.obj_library
                    .get_mut(&handle)
                    .ok_or(DrawError::DrawableHandleNotFound {
                        screen_id: handle.0,
                        obj_id: handle.1,
                    })?;

            let mut guard = obj
                .drawable
                .lock()
                .map_err(|_| DrawError::DrawableLockFailed)?;

            let drawable_obj: &mut dyn crate::draw::terminal_buffer::drawable::Drawable =
                &mut **guard;

            if let Some(dp) = drawable_obj.as_double_pointed_mut() {
                let start = dp.start();
                let end = dp.end();

                let new_start = Point {
                    x: (start.x as i32 + dx).clamp(0, u16::MAX as i32) as u16,
                    y: (start.y as i32 + dy).clamp(0, u16::MAX as i32) as u16,
                };
                let new_end = Point {
                    x: (end.x as i32 + dx).clamp(0, u16::MAX as i32) as u16,
                    y: (end.y as i32 + dy).clamp(0, u16::MAX as i32) as u16,
                };

                dp.set_start(new_start);
                dp.set_end(new_end);
            } else if let Some(sp) = drawable_obj.as_single_pointed_mut() {
                let pos = sp.position();
                let new_pos = Point {
                    x: (pos.x as i32 + dx).clamp(0, u16::MAX as i32) as u16,
                    y: (pos.y as i32 + dy).clamp(0, u16::MAX as i32) as u16,
                };
                sp.set_position(new_pos);
            } else {
                info!(
                    "Drawable {:?} does not implement a point trait; nothing moved",
                    handle
                );
            }
        }
        Ok(())
    }

    pub fn move_drawable_point(
        &mut self,
        handle: DrawableKey,
        point_index: usize, // 0 => start, 1 => end
        new_pos: Point<u16>,
    ) -> Result<(), DrawError> {
        self.remove_drawable(handle)?;
        {
            let obj =
                self.obj_library
                    .get_mut(&handle)
                    .ok_or(DrawError::DrawableHandleNotFound {
                        screen_id: handle.0,
                        obj_id: handle.1,
                    })?;

            let mut guard = obj
                .drawable
                .lock()
                .map_err(|_| DrawError::DrawableLockFailed)?;

            let drawable_obj: &mut dyn crate::draw::terminal_buffer::drawable::Drawable =
                &mut **guard;

            if let Some(dp) = drawable_obj.as_double_pointed_mut() {
                let clamped = Point {
                    x: (new_pos.x as i32).clamp(0, u16::MAX as i32) as u16,
                    y: (new_pos.y as i32).clamp(0, u16::MAX as i32) as u16,
                };

                match point_index {
                    0 => dp.set_start(clamped),
                    1 => dp.set_end(clamped),
                    invalid => {
                        info!(
                            "move_drawable_point: invalid point_index {} for {:?}; nothing moved",
                            invalid, handle
                        );
                    }
                }
            } else {
                info!(
                    "Drawable {:?} does not implement a point trait; nothing moved",
                    handle
                );
            }
        }
        Ok(())
    }
}
