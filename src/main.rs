pub mod draw;

use ascii_assets::{Color, TerminalChar};
use common_stdx::{Point, Rect};
use crossterm::event::{Event, KeyCode, poll, read};
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
use crate::draw::terminal_buffer::CrosstermScreenBuffer;
use crate::draw::terminal_buffer::standard_drawables::PolygonDrawable;
use crate::draw::{DrawObject, DrawableKey, SpriteDrawable};

struct PolyAnim {
    id: DrawableKey,
    center: Point<f64>,
    radius: f64,
    sides: usize,
    angle: f64,
    speed: f64,
}

fn generate_regular_polygon(cx: f64, cy: f64, r: f64, sides: usize) -> Vec<Point<u16>> {
    let mut pts = Vec::with_capacity(sides);
    for i in 0..sides {
        let theta = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
        let x = cx + r * theta.cos();
        let y = cy + r * theta.sin();
        pts.push(Point {
            x: x.round() as u16,
            y: y.round() as u16,
        });
    }
    pts
}

fn main() -> Result<(), AppError> {
    init_terminal()?;
    generate_sprites()?;

    let sprite_path = "assets/debugging/test_video.ascv";
    setup_logger("./log.txt".to_string())?;
    let mut rng = rand::thread_rng();

    let term_size = size()?;
    let mut r = Renderer::<CrosstermScreenBuffer>::create_renderer(term_size);
    r.set_render_mode(RenderMode::Instant);
    let sprite_id = r.register_sprite_from_source(sprite_path, None)?;

    // create screen
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

    let obj_id = r.register_drawable(screen_id, obj)?;

    let mut poly_anims: Vec<PolyAnim> = Vec::new();
    for i in 0..30 {
        let cx = rng.gen_range(10..term_size.0 - 10) as f64;
        let cy = rng.gen_range(5..term_size.1 - 5) as f64;
        let radius = rng.gen_range(3..8) as f64;
        let sides = rng.gen_range(3..7);
        let speed = rng.gen_range(0.5..1.5);

        let colors = [
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Grey,
            Color::Teal,
            Color::White,
        ];
        let border_style = TerminalChar::with_fg('#', colors[i % colors.len()]);

        let obj = DrawObject {
            layer: 1338,
            drawable: Box::new(PolygonDrawable {
                points: vec![],
                fill_style: None,
                border_style,
            }),
        };
        let poly_id = r.register_drawable(screen_id, obj)?;

        let base_points = generate_regular_polygon(cx, cy, radius, sides);
        r.replace_drawable_points(poly_id, base_points.clone())?;

        poly_anims.push(PolyAnim {
            id: poly_id,
            center: Point { x: cx, y: cy },
            radius,
            sides,
            angle: 0.0,
            speed,
        });
    }

    let mut frame_time = Vec::new();
    let program_start = Instant::now();
    let mut last_report = Instant::now();
    let mut frames_since_last = 0;

    let mut running = true;
    while running {
        let start = Instant::now();

        if poll(Duration::from_millis(10))? {
            match read()? {
                Event::Key(k) => {
                    if k.code == KeyCode::Esc || k.code == KeyCode::Char('q') {
                        running = false;
                        info!(
                            "average frametime: {:.3} ms",
                            frame_time.iter().sum::<Duration>().as_millis() as f64
                                / frame_time.len() as f64
                        );
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

        let elapsed_since_start = program_start.elapsed().as_secs_f64();
        for anim in &mut poly_anims {
            anim.angle += anim.speed * elapsed_since_start;
            if anim.angle > 2.0 * std::f64::consts::PI {
                anim.angle -= 2.0 * std::f64::consts::PI;
            }

            let mut new_pts = Vec::with_capacity(anim.sides);
            for i in 0..anim.sides {
                let theta =
                    2.0 * std::f64::consts::PI * (i as f64) / (anim.sides as f64) + anim.angle;
                let x = anim.center.x + anim.radius * theta.cos();
                let y = anim.center.y + anim.radius * theta.sin();

                new_pts.push(Point {
                    x: x.clamp(0.0, (term_size.0 - 1) as f64).round() as u16,
                    y: y.clamp(0.0, (term_size.1 - 1) as f64).round() as u16,
                });
            }
            r.replace_drawable_points(anim.id, new_pts)?;
        }

        r.render_drawable(obj_id)?;

        let frame_duration = start.elapsed();
        frame_time.push(frame_duration);

        thread::sleep(Duration::from_millis(200));

        frames_since_last += 1;
        if last_report.elapsed() >= Duration::from_secs(1) {
            let fps = frames_since_last as f64 / last_report.elapsed().as_secs_f64();
            info!(
                "FPS: {:.2} | Avg frame time: {:.3} ms",
                fps,
                (frame_time.iter().map(|d| d.as_millis() as f64).sum::<f64>()
                    / frame_time.len() as f64)
            );
            last_report = Instant::now();
            frames_since_last = 0;
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
