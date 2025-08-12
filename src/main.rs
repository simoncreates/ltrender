pub mod draw;

use ascii_assets::{AsciiVideo, TerminalChar};
use common_stdx::{Point, Rect};
use crossterm::event::{Event, KeyCode, poll, read};
use crossterm::style::Color;
use crossterm::terminal::size;
use draw::{DrawError, Renderer, init_terminal, restore_terminal};
use env_logger::Builder;
use std::fs::File;
use std::io::Write;
use std::time::Duration;
mod temp_sprite_creation;
use temp_sprite_creation::generate_sprites;

use crate::draw::error::AppError;
use crate::draw::terminal_buffer::standard_drawables::{LineDrawable, RectDrawable};
use crate::draw::{DrawObject, DrawableKey};

fn main() -> Result<(), AppError> {
    init_terminal()?;
    generate_sprites()?;
    let sprite_path = "assets/debugging/test_video.ascv";
    setup_logger("./log.txt".to_string())?;

    let size = size()?;
    let mut r = Renderer::create_renderer(size);
    let sprite_id = r.register_sprite_from_source(sprite_path, None)?;

    let screen_id = r.create_screen(Rect::from_coords(0, 0, 100, 100), 7);

    let layer = 1;
    let position = Point { x: 0, y: 0 };
    let obj_id = r.register_sprite_drawable(screen_id, layer, position, sprite_id)?;

    let line_obj = DrawObject {
        layer: 10,
        drawable: Box::new(LineDrawable {
            start: Point { x: 5, y: 5 },
            end: Point { x: 100, y: 20 },
            chr: TerminalChar {
                chr: '#',
                fg_color: Some(Color::Black),
                bg_color: Some(Color::Yellow),
            },
        }),
    };

    let rect_obj = DrawObject {
        layer: 0,
        drawable: Box::new(RectDrawable {
            rect: Rect::from_coords(0, 0, 500, 500),
            border_thickness: 0,
            border_style: TerminalChar {
                chr: '#',
                fg_color: Some(Color::White),
                bg_color: Some(Color::White),
            },
            fill_style: Some(TerminalChar {
                chr: ' ',
                fg_color: Some(Color::Black),
                bg_color: Some(Color::Black),
            }),
        }),
    };

    let line_obj_id = r.register_drawable(screen_id, line_obj)?;
    let rect_obj_id = r.register_drawable(screen_id, rect_obj)?;

    let mut running = true;
    render_all(line_obj_id, rect_obj_id, obj_id, &mut r)?;

    while running {
        if poll(Duration::from_millis(10))? {
            match read()? {
                Event::Key(k) => {
                    if k.code == KeyCode::Esc || k.code == KeyCode::Char('q') {
                        running = false;
                    }
                }
                Event::Resize(new_cols, new_rows) => {
                    r.handle_resize((new_cols, new_rows))?;
                }
                _ => {}
            }
        }
    }

    restore_terminal()?;
    Ok(())
}

fn render_all(
    line: DrawableKey,
    rect: DrawableKey,
    sprite: DrawableKey,
    r: &mut Renderer,
) -> Result<(), DrawError> {
    r.render_drawable(line)?;
    r.render_drawable(rect)?;
    r.render_drawable(sprite)?;
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
