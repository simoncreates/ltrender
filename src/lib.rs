pub mod renderer;
pub use renderer::{ObjectId, Renderer, SpriteId};

pub mod update_interval_handler;
pub use update_interval_handler::{UpdateInterval, UpdateIntervalHandler};

pub mod terminal_buffer;
pub use terminal_buffer::{ScreenBuffer, SpriteDrawable};

pub mod sprite_register;
pub use sprite_register::{SpriteEntry, SpriteRegistry};

pub mod term_utils;
pub use term_utils::{initial_terminal_state, restore_terminal};

pub mod error;
pub use error::{DrawError, FileError};

pub mod display_screen;
pub use display_screen::{Screen, ScreenKey};

pub mod drawable_register;
pub use drawable_register::{DrawObject, DrawObjectKey, DrawObjectLibrary};

pub mod draw_object_builder;
pub use draw_object_builder::DrawObjectBuilder;

pub mod input_handler;
pub use input_handler::CrosstermEventManager;

// for testing purposes only
use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::{
    error::AppError,
    terminal_buffer::buffer_and_celldrawer::standard_celldrawer::test_celldrawer::TerminalContentInformation,
};

static TEST_DATA: Lazy<Mutex<Option<TerminalContentInformation>>> = Lazy::new(|| Mutex::new(None));

pub fn set_test_data(data: TerminalContentInformation) {
    let mut guard = TEST_DATA.lock().unwrap();
    *guard = Some(data);
}

pub fn get_test_data() -> Option<TerminalContentInformation> {
    let guard = TEST_DATA.lock().unwrap();
    guard.clone()
}

use env_logger::Builder;
use std::fs::File;
use std::io::Write;

pub fn init_logger(path: &str) -> Result<(), AppError> {
    let f = File::create(path)?;
    Builder::new()
        .format_timestamp_secs()
        .format(|buf, record| writeln!(buf, "[{}] {}", record.level(), record.args()))
        .target(env_logger::Target::Pipe(Box::new(f)))
        .filter_level(log::LevelFilter::Info)
        .init();
    Ok(())
}
