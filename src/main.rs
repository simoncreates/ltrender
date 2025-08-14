pub mod draw;

use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};
use crossterm::event::{Event, KeyCode, poll, read};
use crossterm::style::Color;
use crossterm::terminal::size;
use draw::{DrawError, Renderer, init_terminal, restore_terminal};
use env_logger::Builder;
use log::info;
use rand::Rng as _;
use std::fs::File;
use std::io::Write;
use std::thread;
use std::time::{Duration, Instant};
mod temp_sprite_creation;
use temp_sprite_creation::generate_sprites;

use crate::draw::error::AppError;
use crate::draw::renderer::RenderMode;
use crate::draw::terminal_buffer::{CrosstermScreenBuffer, LineDrawable};
use crate::draw::{DrawObject, SpriteDrawable};

fn main() -> Result<(), AppError> {
    init_terminal()?;
    generate_sprites()?;
    let sprite_path = "assets/debugging/test_video.ascv";
    setup_logger("./log.txt".to_string())?;
    let mut rng = rand::thread_rng();

    let term_size = size()?;
    let mut r = Renderer::<CrosstermScreenBuffer>::create_renderer(term_size);
    let sprite_id = r.register_sprite_from_source(sprite_path, None)?;
    r.set_render_mode(RenderMode::Buffered);

    let screen_id = r.create_screen(Rect::from_coords(0, 0, term_size.0, term_size.1), 7);

    let layer = 1;
    let position = Point { x: 0, y: 0 };
    let obj = DrawObject {
        layer,
        drawable: Box::new(SpriteDrawable {
            position,
            sprite_id,
        }),
    };
    let line_obj = DrawObject {
        layer: 100,
        drawable: Box::new(LineDrawable {
            start: Point { x: 0, y: 0 },
            end: Point { x: 10, y: 10 },
            chr: TerminalChar {
                chr: '#',
                fg_color: Some(Color::Blue),
                bg_color: Some(Color::Black),
            },
        }),
    };
    let line_id = r.register_drawable(screen_id, line_obj.clone())?;
    let line_id_2 = r.register_drawable(screen_id, line_obj)?;
    let obj_id = r.register_drawable(screen_id, obj)?;

    let mut running = true;

    while running {
        let start = Instant::now();
        if poll(Duration::from_millis(10))? {
            match read()? {
                Event::Key(k) => {
                    if k.code == KeyCode::Esc || k.code == KeyCode::Char('q') {
                        running = false;
                    }
                    if k.code == KeyCode::Right {
                        r.move_drawable_by(obj_id, 1, 0)?;
                    }
                }
                Event::Resize(new_cols, new_rows) => {
                    r.handle_resize((new_cols, new_rows))?;
                }
                _ => {}
            }
        }
        let current_size: (u16, u16) = size()?;

        r.move_drawable_point(
            line_id,
            rng.gen_range(0..2),
            Point {
                x: rng.gen_range(0..current_size.0),
                y: rng.gen_range(0..current_size.1),
            },
        )?;
        r.move_drawable_point(
            line_id_2,
            rng.gen_range(0..2),
            Point {
                x: rng.gen_range(0..current_size.0),
                y: rng.gen_range(0..current_size.1),
            },
        )?;
        r.render_drawable(line_id)?;
        r.render_drawable(line_id_2)?;
        r.render_drawable(obj_id)?;
        let duration = start.elapsed();
        info!(
            "handling drawing took: {:.3} ms",
            duration.as_secs_f64() * 1000.0
        );
        r.render_frame()?;
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
