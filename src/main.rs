pub mod draw;

use ascii_assets::{AsciiVideo, TerminalChar};
use common_stdx::{Point, Rect};
use crossterm::style::Color;
use crossterm::terminal::size;
use draw::{DrawError, Renderer, init_terminal, restore_terminal};
use env_logger::Builder;
use std::fs::File;
use std::io::Write;
use std::{thread, time};

mod temp_sprite_creation;
use temp_sprite_creation::generate_sprites;

use crate::draw::DrawObject;
use crate::draw::error::AppError;
use crate::draw::terminal_buffer::standard_drawables::{LineDrawable, RectDrawable};

fn main() -> Result<(), AppError> {
    init_terminal()?;
    generate_sprites()?;
    let sprite_path = "assets/debugging/test_video.ascv";
    setup_logger("./log.txt".to_string())?;

    let mut r = Renderer::new(size()?);
    let sprite_id = r.register_sprite(sprite_path, None)?;

    let screen_id = r.add_screen(Rect::from_coords(0, 0, 100, 100), 7);

    let layer = 1;
    let position = Point { x: 0, y: 0 };
    let obj_id = r.add_sprite_object(screen_id, layer, position, sprite_id)?;
    let line_obj = DrawObject {
        layer: 10,
        drawable: Box::new(LineDrawable {
            start: Point { x: 5, y: 5 },
            end: Point { x: 7, y: 10 },
            chr: TerminalChar {
                chr: '#',
                fg_color: Some(Color::Black),
                bg_color: Some(Color::Black),
            },
        }),
    };

    let rect_obj = DrawObject {
        layer: 10,
        drawable: Box::new(RectDrawable {
            rect: Rect::from_coords(5, 5, 20, 20),
            border_thickness: 2,
            border_style: TerminalChar {
                chr: '#',
                fg_color: Some(Color::Black),
                bg_color: Some(Color::Black),
            },
            fill_style: None,
        }),
    };

    let line_obj_id = r.register_object(screen_id, line_obj)?;
    let rect_obj_id = r.register_object(screen_id, rect_obj)?;

    r.render(obj_id)?;
    let duration = time::Duration::from_secs(1);
    thread::sleep(duration);

    r.render(line_obj_id)?;
    thread::sleep(duration);
    r.render(rect_obj_id)?;
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
