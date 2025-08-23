use ascii_assets::{Color, TerminalChar};
use common_stdx::{Point, Rect};
use crossterm::event::{Event, KeyCode, poll, read};
use crossterm::terminal::size;
use env_logger::Builder;
use flume::Sender;
use log::info;
use rand::Rng as _;
use std::fs::File;
use std::io::Write;
use std::thread;
use std::time::{Duration, Instant};

use crate::draw::draw_object_builder::SpriteDrawableBuilder;
use crate::draw::draw_object_builder::videostream_drawable_builder::make_videostream_drawable;
use crate::draw::error::AppError;
use crate::draw::renderer::RendererHandle;
use crate::draw::renderer::render_handle::RendererManager;
use crate::draw::terminal_buffer::screen_buffer::shaders::{
    FlipDiagonal, FlipHorizontal, FlipVertical, Grayscale,
};
use crate::draw::terminal_buffer::standard_buffers::crossterm_buffer::DefaultScreenBuffer;
use crate::draw::terminal_buffer::standard_drawables::sprite_drawable::{
    AnimationInfo, FrameIdent, VideoLoopType, VideoSpeed,
};
use crate::draw::terminal_buffer::standard_drawables::videostream_drawable::StreamFrame;
use crate::draw::terminal_buffer::standard_drawables::{PolygonDrawable, text_drawable};
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

fn create_frame(width: u16, height: u16, tick: usize) -> Vec<Option<TerminalChar>> {
    let mut data: Vec<Option<TerminalChar>> =
        Vec::with_capacity((width as usize) * (height as usize));
    let chars = [' ', '.', '-', '~', '*', '+', 'o', 'O', '@', '#'];

    for y in 0..height {
        for x in 0..width {
            let xf = x as f32;
            let yf = y as f32;
            let w = width as f32;
            let h = height as f32;

            let wave = (xf + tick as f32) * 0.2;
            let sin_y = (wave).sin();
            let normalized = (sin_y + 1.0) / 2.0;
            let target_y = normalized * h;

            let dy = (yf - target_y).abs();

            let ch = if dy < 0.5 {
                Some(TerminalChar::with_fg('@', Color::Blue))
            } else {
                let brightness = ((yf / h) * (chars.len() as f32 - 1.0)) as usize;
                let symbol = chars[brightness.min(chars.len() - 1)];
                Some(TerminalChar::from_char(symbol))
            };

            data.push(ch);
        }
    }

    data
}

fn spawn_frame_sender(
    sender: Sender<StreamFrame>,
    render_handle: RendererHandle,
    drawobjectkey: DrawObjectKey,
) {
    // Pick a size – the same one you will set on the drawable.
    const WIDTH: u16 = 80;
    const HEIGHT: u16 = 24;

    thread::spawn(move || {
        let mut tick = 0usize;
        loop {
            let frame_data = create_frame(WIDTH, HEIGHT, tick);
            let frame = StreamFrame {
                data: frame_data,
                size: (WIDTH, HEIGHT),
            };

            // If the renderer drops the channel we stop producing.
            if sender.send(frame).is_err() {
                break;
            }
            render_handle.render_drawable(drawobjectkey);

            // ~30 FPS
            thread::sleep(Duration::from_millis(33));
            tick = tick.wrapping_add(1);
        }
    });
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

    let (manager, mut r) = RendererManager::new::<DefaultScreenBuffer>(term_size, 500);

    r.set_update_interval(2000000000000);
    // create screen
    let screen_id = r.create_screen(
        Rect::from_coords(0, 0, term_size.0 as i32, term_size.1 as i32),
        7,
    );

    let sprite_id =
        r.register_sprite_from_source("./assets/debugging/test_video.ascv".to_string())?;

    let (sender, drawable) = make_videostream_drawable((10, 10))?;

    let videostream_id = DrawObjectBuilder::default()
        .layer(8000)
        .screen(screen_id)
        .drawable(drawable)
        .shader(FlipVertical)
        .shader(FlipHorizontal)
        .build(&mut r)?;

    spawn_frame_sender(sender, r.clone(), videostream_id);

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

    let points: Vec<Point<i32>> = vec![
        Point { x: 10, y: 2 },
        Point { x: 14, y: 8 },
        Point { x: 20, y: 4 },
        Point { x: 16, y: 12 },
        Point { x: 20, y: 20 },
        Point { x: 14, y: 16 },
        Point { x: 10, y: 22 },
        Point { x: 6, y: 12 },
    ];

    let text_drawable = Box::new(text_drawable::TextDrawable {
        area: Rect::from_coords(50, 50, 60, 60),
        lines: vec![
            text_drawable::LineInfo {
                text: "Hello, gooon world!".to_string(),
                spans: vec![
                    text_drawable::StyledSpan {
                        range: 0..5,
                        style: text_drawable::TextStyle {
                            foreground: Some(Color::Red),
                            ..Default::default()
                        },
                    },
                    text_drawable::StyledSpan {
                        range: 7..13,
                        style: text_drawable::TextStyle {
                            bold: Some(true),
                            italic: Some(true),
                            ..Default::default()
                        },
                    },
                    text_drawable::StyledSpan {
                        range: 7..17,
                        style: text_drawable::TextStyle {
                            foreground: Some(Color::Blue),
                            ..Default::default()
                        },
                    },
                ],
                alignment: text_drawable::TextAlignment::Left,
                default_style: text_drawable::TextStyle {
                    foreground: Some(Color::White),
                    ..Default::default()
                },
            },
            text_drawable::LineInfo {
                text: "centered line".to_string(),
                spans: vec![],
                alignment: text_drawable::TextAlignment::Center,
                default_style: text_drawable::TextStyle {
                    foreground: Some(Color::Green),
                    bold: Some(true),
                    ..Default::default()
                },
            },
            text_drawable::LineInfo {
                text: "this is right".to_string(),
                spans: vec![],
                alignment: text_drawable::TextAlignment::Right,
                default_style: text_drawable::TextStyle {
                    background: Some(Color::Red),
                    ..Default::default()
                },
            },
        ],
        wrapping: true,
        scroll_y: 0,
    });

    let text_id = DrawObjectBuilder::default()
        .layer(34095803498)
        .screen(screen_id)
        .drawable(text_drawable)
        .build(&mut r)?;

    let polygon_id = DrawObjectBuilder::default()
        .layer(6000)
        .screen(screen_id)
        .polygon_drawable(|p| {
            p.border_style('P').points(points).fill_style(TerminalChar {
                chr: ' ',
                fg_color: Some(Color {
                    reset: false,
                    rgb: (200, 40, 255),
                }),
                bg_color: Some(Color {
                    reset: false,
                    rgb: (200, 40, 255),
                }),
            })
        })?
        .build(&mut r)?;

    let mut poly_anims: Vec<PolyAnim> = Vec::new();
    for i in 0..20 {
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

        // thread::sleep(Duration::from_millis(80));

        r.render_drawable(sprite_video_id);
        r.render_drawable(polygon_id);
        r.render_drawable(text_id);

        r.render_frame();
        r.clear_terminal();
    }

    manager.stop();
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
