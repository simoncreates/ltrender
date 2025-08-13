pub mod draw;

use ascii_assets::AsciiVideo;
use common_stdx::{Point, Rect};
use crossterm::event::{Event, KeyCode, poll, read};
use crossterm::terminal::size;
use draw::{DrawError, Renderer, init_terminal, restore_terminal};
use env_logger::Builder;
use log::info;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
mod temp_sprite_creation;
use temp_sprite_creation::generate_sprites;

use crate::draw::error::AppError;
use crate::draw::terminal_buffer::CrosstermScreenBuffer;
use crate::draw::{DrawObject, SpriteDrawable};

fn main() -> Result<(), AppError> {
    init_terminal()?;
    generate_sprites()?;
    let sprite_path = "assets/debugging/test_video.ascv";
    setup_logger("./log.txt".to_string())?;

    let size = size()?;
    let mut r = Renderer::<CrosstermScreenBuffer>::create_renderer(size);
    let sprite_id = r.register_sprite_from_source(sprite_path, None)?;

    let screen_id = r.create_screen(Rect::from_coords(0, 0, 100, 100), 7);

    let layer = 1;
    let position = Point { x: 0, y: 0 };
    let obj = DrawObject {
        layer,
        drawable: Arc::new(Mutex::new(Box::new(SpriteDrawable {
            position: RefCell::new(position),
            sprite_id: RefCell::new(sprite_id),
        }))),
    };

    let obj_id = r.register_drawable(screen_id, obj.clone())?;
    r.render_drawable(obj_id)?;

    let mut running = true;

    while running {
        if poll(Duration::from_millis(10))? {
            match read()? {
                Event::Key(k) => {
                    if k.code == KeyCode::Esc || k.code == KeyCode::Char('q') {
                        running = false;
                    }
                    if k.code == KeyCode::Right {
                        r.remove_drawable(obj_id)?;
                        r.with_drawable(obj_id, |sprite: &mut SpriteDrawable| {
                            let mut pos = *sprite.position.borrow();
                            pos.x += 1;
                            sprite.position.replace(pos);
                        })?;

                        r.render_drawable(obj_id)?;
                    }
                }
                Event::Resize(new_cols, new_rows) => {
                    let start = Instant::now();
                    r.handle_resize((new_cols, new_rows))?;
                    let duration = start.elapsed();
                    info!(
                        "handling a resize took: {:.3} ms",
                        duration.as_secs_f64() * 1000.0
                    );
                }
                _ => {}
            }
        }
    }
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
