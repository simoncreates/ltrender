pub mod sprite_structs;
use ascii_assets::AsciiVideo;
pub use sprite_structs::{CharacterInfo, CharacterInfoList};
pub mod update_interval_handler;
pub use update_interval_handler::{UpdateInterval, UpdateIntervalHandler};
pub mod terminal_buffer;
pub use terminal_buffer::{ScreenBuffer, drawables::SpriteDrawable};
pub mod all_sprites;
pub use all_sprites::{AllSprites, SpriteObject, SpriteObjectType};
pub mod term_utils;
pub use term_utils::{init_terminal, restore_terminal};
pub mod error;
pub use error::{DrawError, FileError};
pub mod screen;
pub use screen::{Screen, ScreenId};
pub mod draw_object_library;
pub use draw_object_library::{DrawObject, DrawObjectLibrary, ObjIdent};

use common_stdx::{Point, Rect};
use std::collections::HashMap;

use crate::draw::error::AppError;

pub type SpriteId = usize;
pub type ObjId = usize;

#[derive(Debug)]
pub struct Renderer {
    screens: HashMap<ScreenId, Screen>,
    obj_library: DrawObjectLibrary,
    screen_buffer: ScreenBuffer,
    pub sprites: AllSprites,
}

impl Renderer {
    pub fn new(size: (u16, u16)) -> Self {
        Renderer {
            obj_library: DrawObjectLibrary::new(),
            screens: HashMap::new(),
            screen_buffer: ScreenBuffer::new(size),
            sprites: AllSprites::new(),
        }
    }

    pub fn add_screen(&mut self, rect: Rect<u16>, layer: usize) -> ScreenId {
        let new_id = self.new_screen_id();
        self.screens
            .insert(new_id, Screen::new(rect, layer, new_id));
        new_id
    }

    pub fn register_object(
        &mut self,
        screen_id: ScreenId,
        obj: DrawObject,
    ) -> Result<ObjIdent, DrawError> {
        if let Some(s) = self.screens.get_mut(&screen_id) {
            let new_obj_id = self.obj_library.add_obj(screen_id, obj);
            s.add_object(new_obj_id);
            Ok((screen_id, new_obj_id))
        } else {
            Err(DrawError::ScreenNotFound(screen_id))
        }
    }

    pub fn add_sprite_object(
        &mut self,
        screen_id: ScreenId,
        layer: usize,
        position: Point<u16>,
        sprite_id: SpriteId,
    ) -> Result<ObjIdent, DrawError> {
        let obj = DrawObject {
            layer,
            drawable: Box::new(SpriteDrawable {
                position,
                sprite_id,
            }),
        };
        self.register_object(screen_id, obj)
    }

    pub fn register_sprite(
        &mut self,
        path: &str,
        frame: Option<usize>,
    ) -> Result<SpriteId, AppError> {
        let video = AsciiVideo::read_from_file(path)?;
        let opt_sprite = if let Some(frame) = frame {
            video.frames.get(frame).cloned()
        } else {
            video.frames.get(0).cloned()
        };
        let sprite_id = if let Some(sprite) = opt_sprite {
            self.sprites.add(SpriteObject {
                info: SpriteObjectType::Sprite(sprite),
            })
        } else {
            return Err(AppError::File(FileError::VideoFrameNotFound {
                video_path: path.to_string(),
                frame_id: frame.unwrap_or(0),
            }));
        };
        Ok(sprite_id)
    }

    pub fn render(&mut self, id: ObjIdent) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&id.0) {
            s.draw_object(
                id.1,
                &mut self.screen_buffer,
                &self.obj_library,
                &self.sprites,
            )?;
            self.update_terminal()?;
            Ok(())
        } else {
            Err(DrawError::ScreenNotFound(id.0))
        }
    }

    pub fn update_terminal(&mut self) -> Result<(), DrawError> {
        self.screen_buffer.update_terminal()?;
        Ok(())
    }

    pub fn new_screen_id(&self) -> ScreenId {
        let mut id = self.screens.len();
        while self.screens.contains_key(&id) {
            id = id.saturating_add(1);
        }
        id
    }
}
