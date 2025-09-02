pub mod renderer;
pub use renderer::{ObjectId, Renderer, SpriteId};

pub mod update_interval_handler;
pub use update_interval_handler::{UpdateInterval, UpdateIntervalHandler};

pub mod terminal_buffer;
pub use terminal_buffer::{ScreenBuffer, SpriteDrawable};

pub mod sprite_register;
pub use sprite_register::{SpriteEntry, SpriteRegistry};

pub mod term_utils;
pub use term_utils::{init_terminal, restore_terminal};

pub mod error;
pub use error::{DrawError, FileError};

pub mod display_screen;
pub use display_screen::{Screen, ScreenKey};

pub mod drawable_register;
pub use drawable_register::{DrawObject, DrawObjectKey, DrawObjectLibrary};

pub mod draw_object_builder;
pub use draw_object_builder::DrawObjectBuilder;

// for testing purposes only
use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::terminal_buffer::buffer_and_celldrawer::standard_celldrawer::test_buffer::TerminalContentInformation;

static TEST_DATA: Lazy<Mutex<Option<TerminalContentInformation>>> = Lazy::new(|| Mutex::new(None));

pub fn set_test_data(data: TerminalContentInformation) {
    let mut guard = TEST_DATA.lock().unwrap();
    *guard = Some(data);
}

pub fn get_test_data() -> Option<TerminalContentInformation> {
    let guard = TEST_DATA.lock().unwrap();
    guard.clone()
}
