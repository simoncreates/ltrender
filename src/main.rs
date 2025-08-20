use ascii_assets::{Color, TerminalChar};
use common_stdx::{Point, Rect};
use crossterm::event::{Event, KeyCode, poll, read};
use crossterm::terminal::size;
use env_logger::Builder;
use log::info;
use rand::Rng as _;
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};

use crate::draw::draw_object_builder::SpriteDrawableBuilder;
use crate::draw::error::AppError;
use crate::draw::renderer::RendererHandle;
use crate::draw::terminal_buffer::screen_buffer::shaders::{
    FlipDiagonal, FlipHorizontal, Grayscale,
};
use crate::draw::terminal_buffer::standard_buffers::crossterm_buffer::DefaultScreenBuffer;
use crate::draw::terminal_buffer::standard_drawables::PolygonDrawable;
use crate::draw::terminal_buffer::standard_drawables::sprite_drawable::{
    AnimationInfo, FrameIdent, VideoLoopType, VideoSpeed,
};
use crate::draw::{DrawObject, DrawObjectBuilder, DrawObjectKey};

pub mod draw;
struct PolyAnim {
    id: DrawObjectKey,
    center: Point<f64>,
    radius: f64,
    sides: usize,
    angle: f64,
    speed: f64,
}

fn generate_regular_polygon(cx: f64, cy: f64, r: f64, sides: usize) -> Vec<Point<i32>> {
    let mut rng = rand::thread_rng();
    let mut pts = Vec::with_capacity(sides);

    for i in 0..sides {
        let theta = 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);

        let mut x = (cx + r * theta.cos()) as i32;
        let mut y = (cy + r * theta.sin()) as i32;

        x += rng.gen_range(-10..=10);
        y += rng.gen_range(-10..=10);

        let x = x.clamp(0, i32::MAX);
        let y = y.clamp(0, i32::MAX);

        pts.push(Point { x, y });
    }
    pts
}

fn main() -> Result<(), AppError> {
    crate::draw::init_terminal()?;
    setup_logger("./log.txt".to_string())?;
    let mut rng = rand::thread_rng();

    let term_size = size()?;

    let mut r = RendererHandle::new::<DefaultScreenBuffer>(term_size, 500);
    r.set_update_interval(20);

    // create screen
    let screen_id = r.create_screen(
        Rect::from_coords(0, 0, term_size.0 as i32, term_size.1 as i32),
        7,
    );

    let sprite_id =
        r.register_sprite_from_source("./assets/debugging/test_video.ascv".to_string())?;

    let circle_id = DrawObjectBuilder::default()
        .layer(50)
        .screen(screen_id)
        .circle_drawable(|c| c.radius(4).center((20, 20)).radius(13).border_style('G'))?
        .shader(FlipDiagonal)
        .build(&mut r)?;

    let video_drawable = SpriteDrawableBuilder::default()
        .sprite_id(sprite_id)
        .position((0, 0))
        .animation_type(AnimationInfo::Video {
            loop_type: VideoLoopType::Loop,
            speed: VideoSpeed::Fps(3),
            start_frame: FrameIdent::FirstFrame,
            end_frame: FrameIdent::LastFrame,
        })
        .build()?;

    let sprite_video_id = DrawObjectBuilder::default()
        .layer(10)
        .screen(screen_id)
        .drawable(video_drawable)
        .shader(FlipHorizontal)
        .shader(Grayscale)
        .build(&mut r)?;

    let mut poly_anims: Vec<PolyAnim> = Vec::new();
    for i in 0..3 {
        let cx = rng.gen_range(2..term_size.0 - 2) as f64;
        let cy = rng.gen_range(2..term_size.1 - 2) as f64;
        let radius = rng.gen_range(3..20) as f64;
        let sides = rng.gen_range(3..5);
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
        let fill_style = TerminalChar::with_fg('#', colors[(i + 2) % colors.len()]);

        let obj = DrawObject {
            layer: i + 5,
            shaders: Vec::new(),
            drawable: Box::new(PolygonDrawable {
                points: vec![],
                fill_style: Some(fill_style),
                border_style,
            }),
        };
        let poly_id = r.register_drawable(screen_id, obj)?;

        let base_points = generate_regular_polygon(cx, cy, radius, sides);
        r.replace_drawable_points(poly_id, base_points);

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
    let mut max = 0;
    while running && max < 1000 {
        max += 1;
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
                    if k.code == KeyCode::Char('d') {
                        r.move_drawable_by(sprite_video_id, 1, 0);
                    }
                }
                Event::Resize(new_cols, new_rows) => {
                    r.handle_resize((new_cols, new_rows));
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
                    x: x as i32,
                    y: y as i32,
                });
            }
            r.replace_drawable_points(anim.id, new_pts);
            r.render_drawable(anim.id);
        }

        let frame_duration = start.elapsed();
        frame_time.push(frame_duration);

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

        //thread::sleep(Duration::from_millis(80));

        r.render_drawable(sprite_video_id);
        r.render_drawable(circle_id);
        r.render_frame();
        r.clear_terminal();
    }

    r.stop();
    crate::draw::restore_terminal()?;
    Ok(())
}

fn setup_logger(logfilepath: String) -> Result<(), AppError> {
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
