pub mod draw;

use ascii_assets::AsciiVideo;
use common_stdx::{Point, Rect};
use crossterm::terminal::size;
use draw::{DrawError, Renderer, init_terminal, restore_terminal};
use env_logger::Builder;
use std::fs::File;
use std::io::Write;
use std::{thread, time};

mod temp_sprite_creation;
use temp_sprite_creation::generate_sprites;

use crate::draw::error::AppError;

fn main() -> Result<(), AppError> {
    init_terminal()?;
    generate_sprites()?;
    let sprite_path = "assets/debugging/test_video.ascv";
    setup_logger("./log.txt".to_string())?;

    let mut r = Renderer::new(size()?);
    let sprite_id = r.register_sprite(sprite_path, None)?;
    let sprite_id_2 = r.register_sprite(sprite_path, None)?;

    let screen_id = r.add_screen(Rect::from_coords(10, 10, 12, 12), 7);
    let screen_id_2 = r.add_screen(Rect::from_coords(5, 5, 8, 8), 0);

    let layer = 1;
    let position = Point { x: 0, y: 0 };
    let obj_id = r.add_sprite_object(screen_id, layer, position, sprite_id)?;
    let obj_id_2 = r.add_sprite_object(screen_id_2, layer, position, sprite_id_2)?;

    r.render(obj_id)?;
    let duration = time::Duration::from_secs(2);
    thread::sleep(duration);

    r.render(obj_id_2)?;
    thread::sleep(duration);
    restore_terminal()?;

    Ok(())
}

fn setup_logger(logfilepath: String) -> Result<(), DrawError> {
    let log_file = File::create(logfilepath)?;
    Builder::new()
        .format_timestamp_secs()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}] - {} - {}",
                record.level(),
                record.target(),
                record.args()
            )
        })
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .filter_level(log::LevelFilter::Debug)
        .init();
    Ok(())
}
