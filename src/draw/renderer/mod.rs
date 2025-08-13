use ascii_assets::AsciiVideo;
use common_stdx::{Point, Rect};
use log::info;
use std::collections::HashMap;

use crate::draw::ScreenBuffer;
use crate::draw::{
    DrawError, DrawObject, DrawObjectLibrary, DrawableKey, FileError, Screen, ScreenKey,
    SpriteData, SpriteDrawable, SpriteEntry, SpriteRegistry, error::AppError,
    terminal_buffer::CrosstermScreenBuffer,
}; // bring the trait into scope

pub type SpriteId = usize;
pub type DrawableHandle = usize;
pub struct Renderer<B = CrosstermScreenBuffer>
where
    B: ScreenBuffer,
{
    screens: HashMap<ScreenKey, Screen>,
    obj_library: DrawObjectLibrary,
    screen_buffer: B,
    pub sprites: SpriteRegistry,
}

impl<B> Renderer<B>
where
    B: ScreenBuffer,
{
    /// Create a new renderer with an initial terminal size.
    pub fn create_renderer(size: (u16, u16)) -> Self {
        Renderer {
            obj_library: DrawObjectLibrary::new(),
            screens: HashMap::new(),
            screen_buffer: B::new(size),
            sprites: SpriteRegistry::new(),
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
            info!("Rendering drawable with id: {:?}", id);
            s.render_drawable(
                id.1,
                &mut self.screen_buffer,
                &self.obj_library,
                &self.sprites,
            )?;
            self.refresh()?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(id.0))
        }
    }

    /// Render all objects on all screens.
    pub fn render_all(&mut self) -> Result<(), DrawError> {
        for screen in self.screens.values_mut() {
            screen.render_all(&mut self.screen_buffer, &self.obj_library, &self.sprites)?;
        }
        self.refresh()?;
        Ok(())
    }

    pub fn handle_resize(&mut self, new_size: (u16, u16)) -> Result<(), DrawError> {
        self.screen_buffer = B::new(new_size);
        B::mark_all_dirty(&mut self.screen_buffer, new_size);
        self.render_all()?;
        Ok(())
    }

    /// Flush the buffer to the terminal.
    pub fn refresh(&mut self) -> Result<(), DrawError> {
        B::update_terminal(&mut self.screen_buffer)?;
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
}
