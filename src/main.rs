use ascii_assets::{Color, TerminalChar};
use common_stdx::{Point, Rect};
use crossterm::event::{Event, KeyCode, MouseEventKind, poll, read};
use crossterm::terminal::size;
use env_logger::Builder;
use log::{debug, info, warn};
use rand::Rng;
use std::cmp::Ordering;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::SyncSender;
use std::time::{Duration, Instant};

use crate::draw::draw_object_builder::SpriteDrawableBuilder;
use crate::draw::draw_object_builder::videostream_drawable_builder::make_videostream_drawable;
use crate::draw::drawable_register::ObjectLifetime;
use crate::draw::error::AppError;
use crate::draw::renderer::RendererHandle;
use crate::draw::renderer::render_handle::RendererManager;
use crate::draw::terminal_buffer::screen_buffer::shaders::Grayscale;
use crate::draw::terminal_buffer::standard_buffers::crossterm_buffer::DefaultScreenBuffer;
use crate::draw::terminal_buffer::standard_drawables::sprite_drawable::{
    AnimationInfo, FrameIdent, VideoLoopType, VideoSpeed,
};
use crate::draw::terminal_buffer::standard_drawables::videostream_drawable::StreamFrame;
use crate::draw::terminal_buffer::standard_drawables::{PolygonDrawable, text_drawable};
use crate::draw::{DrawObject, DrawObjectBuilder, DrawObjectKey};

pub mod draw;

// ------------------------------------------------------------
// Organic swarm demo (drop-in replacement for main.rs)
// - mouse left: attract swarm
// - mouse right: repel swarm
// - mouse drag: paint a soft force field trail
// - scroll (if available in your terminal): adjust force strength
// - key 'n': reseed swarm
// - key 'h': toggle HUD text
// - key 'q' or ESC: quit
// ------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Agent {
    id: DrawObjectKey,
    pos: (f64, f64),
    dir: f64,
    speed: f64,
    wobble: f64,
    size: f64,
}

#[derive(Default, Debug, Clone, Copy)]
struct MouseState {
    x: i32,
    y: i32,
    left: bool,
    right: bool,
    strength: f64,
}

fn init_logger(path: &str) -> Result<(), AppError> {
    let f = File::create(path)?;
    Builder::new()
        .format_timestamp_secs()
        .format(|buf, record| writeln!(buf, "[{}] {}", record.level(), record.args()))
        .target(env_logger::Target::Pipe(Box::new(f)))
        .filter_level(log::LevelFilter::Info)
        .init();
    Ok(())
}

fn wrap(v: f64, max: f64) -> f64 {
    if v < 0.0 {
        v + max
    } else if v >= max {
        v - max
    } else {
        v
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn triangle_points(cx: f64, cy: f64, angle: f64, size: f64) -> Vec<Point<i32>> {
    let a = angle;
    let s = size.max(1.0);
    let p1 = (cx + a.cos() * s, cy + a.sin() * s);
    let p2 = (
        cx + (a + 2.4).cos() * (s * 0.6),
        cy + (a + 2.4).sin() * (s * 0.6),
    );
    let p3 = (
        cx + (a - 2.4).cos() * (s * 0.6),
        cy + (a - 2.4).sin() * (s * 0.6),
    );
    vec![
        Point {
            x: p1.0 as i32,
            y: p1.1 as i32,
        },
        Point {
            x: p2.0 as i32,
            y: p2.1 as i32,
        },
        Point {
            x: p3.0 as i32,
            y: p3.1 as i32,
        },
    ]
}

fn make_agent_obj(layer: usize, col: Color) -> DrawObject {
    let border = TerminalChar::with_fg('#', col);
    let fill = TerminalChar::with_fg('#', col);
    DrawObject {
        layer,
        shaders: Vec::new(),
        drawable: Box::new(PolygonDrawable {
            points: vec![],
            fill_style: Some(fill),
            border_style: border,
        }),
        lifetime: ObjectLifetime::ExplicitRemove,
        creation_time: Instant::now(),
    }
}

fn seeded_swarm(
    r: &mut RendererHandle,
    screen_id: usize,
    n: usize,
    cols: u16,
    rows: u16,
) -> Vec<Agent> {
    let mut rng = rand::thread_rng();
    let palette = [
        Color::Teal,
        Color::Blue,
        Color::Green,
        Color::Yellow,
        Color::White,
        Color::Grey,
    ];
    let mut out = Vec::with_capacity(n);

    for i in 0..n {
        let col = palette[i % palette.len()];
        let obj = make_agent_obj(100 + i, col);
        let key = r
            .register_drawable(screen_id, obj)
            .expect("register_drawable");
        let x = rng.gen_range(1..cols.saturating_sub(2)) as f64;
        let y = rng.gen_range(1..rows.saturating_sub(2)) as f64;
        let dir = rng.gen_range(0.0..std::f64::consts::TAU);
        let spd = rng.gen_range(0.6..1.8);
        let wob = rng.gen_range(0.2..1.0);
        let sz = rng.gen_range(1.5..3.5);
        out.push(Agent {
            id: key,
            pos: (x, y),
            dir,
            speed: spd,
            wobble: wob,
            size: sz,
        });
    }
    out
}

fn spawn_soft_wave(sender: SyncSender<StreamFrame>, ren: RendererHandle, key: DrawObjectKey) {
    // A very lightweight background that breathes under the swarm.
    const W: u16 = 64;
    const H: u16 = 32;
    std::thread::spawn(move || {
        let mut t = 0.0f32;
        loop {
            let mut data: Vec<Option<TerminalChar>> =
                Vec::with_capacity((W as usize) * (H as usize));
            for y in 0..H {
                for x in 0..W {
                    let xf = x as f32 / W as f32;
                    let yf = y as f32 / H as f32;
                    let w = ((xf * 10.0 + t).sin() + (yf * 7.0 - t * 0.7).cos()) * 0.5 + 0.5;
                    let ch = match (w * 8.0) as i32 {
                        0..=1 => Some(TerminalChar::from_char(' ')),
                        2..=3 => Some(TerminalChar::from_char('.')),
                        4..=5 => Some(TerminalChar::from_char('*')),
                        _ => Some(TerminalChar::with_fg('@', Color::Grey)),
                    };
                    data.push(ch);
                }
            }
            let _ = sender.send(StreamFrame { data, size: (W, H) });
            ren.render_drawable(key);
            std::thread::sleep(Duration::from_millis(40));
            t += 0.04;
        }
    });
}

pub fn main() -> Result<(), AppError> {
    crate::draw::init_terminal()?;
    init_logger("./organic_swarm.log")?;

    let (cols, rows) = size()?;
    let (manager, mut r) = RendererManager::new::<DefaultScreenBuffer>((cols, rows), 600);
    r.set_update_interval(16);

    // One screen spanning the whole terminal.
    let screen = r.create_screen(Rect::from_coords(0, 0, cols as i32, rows as i32), 5);

    // Gentle animated background (kept subtle by Grayscale shader on the sprite video).
    let (tx, video_drawable) = make_videostream_drawable((8, 6), 1)?;
    let video_id = DrawObjectBuilder::default()
        .layer(1)
        .screen(screen)
        .drawable(video_drawable)
        .shader(Grayscale)
        .add_lifetime(ObjectLifetime::ExplicitRemove)
        .build(&mut r)?;
    spawn_soft_wave(tx, r.clone(), video_id);

    // Optional: tiny sprite clock in the corner (if you have a .ascv around, otherwise harmless).
    if let Ok(sprite) =
        r.register_sprite_from_source("./assets/debugging/test_video.ascv".to_string())
    {
        let spr_draw = SpriteDrawableBuilder::default()
            .sprite_id(sprite)
            .position((0, 0))
            .animation_type(AnimationInfo::Video {
                loop_type: VideoLoopType::Loop,
                speed: VideoSpeed::Fps(4),
                start_frame: FrameIdent::FirstFrame,
                end_frame: FrameIdent::LastFrame,
            })
            .build()?;
        let _spr_id = DrawObjectBuilder::default()
            .layer(2)
            .screen(screen)
            .drawable(spr_draw)
            .add_lifetime(ObjectLifetime::ExplicitRemove)
            .build(&mut r)?;
    }

    // HUD text (toggle with 'h').
    let mut show_hud = true;
    let hud_drawable = Box::new(text_drawable::TextDrawable {
        area: Rect::from_coords(1, 1, 50, 10),
        lines: vec![text_drawable::LineInfo {
            text: String::from(
                "organic swarm — drag mouse, left: attract, right: repel, scroll: strength, n: reseed, h: HUD, q: quit",
            ),
            spans: vec![],
            alignment: text_drawable::TextAlignment::Left,
            default_style: text_drawable::TextStyle {
                foreground: Some(Color::White),
                bold: Some(true),
                ..Default::default()
            },
        }],
        wrapping: true,
        scroll_y: 0,
    });
    let hud_id = DrawObjectBuilder::default()
        .layer(9)
        .screen(screen)
        .drawable(hud_drawable)
        .add_lifetime(ObjectLifetime::ExplicitRemove)
        .build(&mut r)?;

    // Seed agents.
    let mut agents = seeded_swarm(&mut r, screen, 140, cols, rows);

    // Input and timing state.
    let start = Instant::now();
    let mut last_tick = Instant::now();
    let mut mouse = MouseState {
        x: (cols / 2) as i32,
        y: (rows / 2) as i32,
        left: false,
        right: false,
        strength: 18.0,
    };
    let mut running = true;

    // Simple FPS reporter.
    let mut frame_times: Vec<Duration> = Vec::with_capacity(300);
    let mut last_report = Instant::now();
    let mut frames = 0u32;

    while running {
        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f64().min(0.05);
        last_tick = now;

        // Input ----------------------------------------------------
        if poll(Duration::from_millis(5))? {
            match read()? {
                Event::Key(k) => {
                    match k.code {
                        KeyCode::Esc | KeyCode::Char('q') => running = false,
                        KeyCode::Char('n') => {
                            // Reseed: drop old agents visually and create new ones.
                            for a in &agents {
                                r.explicit_remove_drawable(a.id);
                            }
                            agents = seeded_swarm(&mut r, screen, 140, cols, rows);
                        }
                        KeyCode::Char('h') => {
                            show_hud = !show_hud;
                            if show_hud {
                                r.render_drawable(hud_id);
                            } else {
                                r.explicit_remove_drawable(hud_id);
                            }
                        }
                        _ => {}
                    }
                }
                Event::Mouse(me) => match me.kind {
                    MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                        mouse.x = me.column as i32;
                        mouse.y = me.row as i32;
                    }
                    MouseEventKind::Down(btn) => {
                        use crossterm::event::MouseButton;
                        match btn {
                            MouseButton::Left => mouse.left = true,
                            MouseButton::Right => mouse.right = true,
                            MouseButton::Middle => {
                                mouse.strength = 12.0;
                            }
                        }
                    }
                    MouseEventKind::Up(btn) => {
                        use crossterm::event::MouseButton;
                        match btn {
                            MouseButton::Left => mouse.left = false,
                            MouseButton::Right => mouse.right = false,
                            MouseButton::Middle => {}
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        mouse.strength = (mouse.strength - 2.0).max(4.0);
                    }
                    MouseEventKind::ScrollUp => {
                        mouse.strength = (mouse.strength + 2.0).min(40.0);
                    }
                    _ => {}
                },
                Event::Resize(c, rws) => {
                    r.handle_resize((c, rws));
                }
                _ => {}
            }
        }

        // Simulation ----------------------------------------------
        let (cw, ch) = (cols as f64, rows as f64);
        let t = start.elapsed().as_secs_f64();

        for a in &mut agents {
            // Flow field (soft curl-like):
            let fx = ((a.pos.1 * 0.05 + t * 0.3).sin() * 0.8) + (t * 0.1 + a.wobble).cos() * 0.2;
            let fy = ((a.pos.0 * 0.05 - t * 0.25).cos() * 0.8) + (t * 0.13).sin() * 0.2;

            let mut steer_x = fx;
            let mut steer_y = fy;

            // Mouse field:
            let dx = mouse.x as f64 - a.pos.0;
            let dy = mouse.y as f64 - a.pos.1;
            let d2 = (dx * dx + dy * dy).max(0.0001);
            let inv = 1.0 / d2.sqrt();
            let toward = (dx * inv, dy * inv);

            if mouse.left {
                // attract
                steer_x += toward.0 * (mouse.strength * inv);
                steer_y += toward.1 * (mouse.strength * inv);
            }
            if mouse.right {
                // repel
                steer_x -= toward.0 * (mouse.strength * inv);
                steer_y -= toward.1 * (mouse.strength * inv);
            }

            // Convert steer into angle change.
            let desired = steer_y.atan2(steer_x);
            let mut diff = desired - a.dir;
            while diff > std::f64::consts::PI {
                diff -= std::f64::consts::TAU;
            }
            while diff < -std::f64::consts::PI {
                diff += std::f64::consts::TAU;
            }

            a.dir += diff * 0.15 + (t * a.wobble).sin() * 0.02;
            a.speed = lerp(a.speed, 1.2 + (t * 0.7 + a.wobble).sin().abs(), 0.05);

            // Integrate position with gentle easing near mouse to make it feel alive.
            let step = a.speed;
            a.pos.0 = wrap(a.pos.0 + a.dir.cos() * step, cw);
            a.pos.1 = wrap(a.pos.1 + a.dir.sin() * step, ch);

            // Push to renderer
            let pts = triangle_points(a.pos.0, a.pos.1, a.dir, a.size);
            r.replace_drawable_points(a.id, pts);
            r.render_drawable(a.id);
        }

        // HUD live update (only redraw occasionally to reduce cost).
        if show_hud && frames % 6 == 0 {
            let fps = if frame_times.is_empty() {
                0.0
            } else {
                1000.0
                    / (frame_times
                        .iter()
                        .map(|d| d.as_millis() as f64)
                        .sum::<f64>()
                        / frame_times.len() as f64)
                        .max(1.0)
            };
            let txt = format!(
                "fps ~{fps:.1} | agents: {} | force: {:.1} | mouse: ({}, {}) L:{} R:{}",
                agents.len(),
                mouse.strength,
                mouse.x,
                mouse.y,
                mouse.left as u8,
                mouse.right as u8
            );
            let mut hud = text_drawable::TextDrawable {
                area: Rect::from_coords(1, 1, 60, 6),
                lines: vec![text_drawable::LineInfo {
                    text: txt,
                    spans: vec![],
                    alignment: text_drawable::TextAlignment::Left,
                    default_style: text_drawable::TextStyle {
                        foreground: Some(Color::White),
                        bold: Some(true),
                        ..Default::default()
                    },
                }],
                wrapping: true,
                scroll_y: 0,
            };
            r.replace_drawable(hud_id, Box::new(hud));
            r.render_drawable(hud_id);
        }

        // Maintain ~60 FPS if possible.
        let frame = last_tick.elapsed();
        frame_times.push(frame);
        if frame_times.len() > 120 {
            frame_times.remove(0);
        }
        frames = frames.wrapping_add(1);
        if last_report.elapsed() > Duration::from_secs(1) {
            let avg_ms = frame_times
                .iter()
                .map(|d| d.as_millis() as f64)
                .sum::<f64>()
                / frame_times.len().max(1) as f64;
            info!("avg frame: {:.2}ms ({} samples)", avg_ms, frame_times.len());
            last_report = Instant::now();
        }

        std::thread::sleep(Duration::from_millis(12));
        r.render_frame();
    }

    // cleanup
    for a in &agents {
        r.explicit_remove_drawable(a.id);
    }
    r.explicit_remove_drawable(video_id);

    manager.stop();
    crate::draw::restore_terminal()?;
    Ok(())
}
