use ascii_assets::{AsciiSprite, AsciiVideo, Color, TerminalChar};
use common_stdx::{Point, Rect};
use crossterm::event::{Event, KeyCode, MouseEventKind, poll, read};
use crossterm::terminal::size;
use env_logger::Builder;
use log::info;
use ltrender::display_screen::ScreenAreaRect;
use ltrender::terminal_buffer::buffer_and_celldrawer::CrosstermCellDrawer;
use ltrender::terminal_buffer::buffer_and_celldrawer::DefaultScreenBuffer;
use rand::Rng;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::SyncSender;
use std::time::{Duration, Instant};

use ltrender::draw_object_builder::SpriteDrawableBuilder;
use ltrender::draw_object_builder::videostream_drawable_builder::make_videostream_drawable;
use ltrender::drawable_register::ObjectLifetime;
use ltrender::error::AppError;
use ltrender::renderer::RendererHandle;
use ltrender::renderer::render_handle::RendererManager;
use ltrender::terminal_buffer::buffer_and_celldrawer::shaders::Grayscale;
use ltrender::terminal_buffer::standard_drawables::sprite_drawable::{
    AnimationInfo, FrameIdent, VideoLoopType, VideoSpeed,
};
use ltrender::terminal_buffer::standard_drawables::videostream_drawable::StreamFrame;
use ltrender::terminal_buffer::standard_drawables::{PolygonDrawable, text_drawable};
use ltrender::{DrawObject, DrawObjectBuilder, DrawObjectKey};

use tick_manager_rs::{TickManager, TickMember};

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

fn spawn_soft_wave(
    sender: SyncSender<StreamFrame>,
    ren: RendererHandle,
    key: DrawObjectKey,
    size: (u16, u16),
    bg_member: TickMember,
) {
    std::thread::spawn(move || {
        let mut t = 0.0f32;
        loop {
            let mut data: Vec<Option<TerminalChar>> =
                Vec::with_capacity((size.0 as usize) * (size.1 as usize));
            for y in 0..size.1 {
                for x in 0..size.0 {
                    let xf = x as f32 / size.1 as f32;
                    let yf = y as f32 / size.1 as f32;
                    let w = ((xf * 10.0 + t).sin() + (yf * 7.0 - t * 0.7).cos()) * 0.5 + 0.5;
                    let ch = match w {
                        w if w > 0.70 => Some(TerminalChar::from_char(' ')),
                        w if w > 0.50 => Some(TerminalChar::from_char('‧')),
                        w if w > 0.30 => Some(TerminalChar::from_char(' ')),
                        _ => Some(TerminalChar::with_fg('.', Color::Grey)),
                    };

                    data.push(ch);
                }
            }
            let _ = sender.send(StreamFrame { data, size });
            ren.render_drawable(key);
            t += 0.04;
            bg_member.wait_for_tick();
        }
    });
}

pub fn main() -> Result<(), AppError> {
    ltrender::init_terminal()?;
    init_logger("./organic_swarm.log")?;

    // creating the tick manager for syncing the waves with the main swarm
    let (_manager, manager_handle) = TickManager::new(tick_manager_rs::Speed::Fps(160));
    let main_member = tick_manager_rs::TickMember::new(manager_handle.clone(), 1);
    // running at half the speed
    let bg_member = tick_manager_rs::TickMember::new(manager_handle, 6);

    let (cols, rows) = size()?;
    let (manager, mut r) =
        RendererManager::new::<DefaultScreenBuffer<CrosstermCellDrawer>>((cols, rows), 100);
    r.set_update_interval(16);

    // One screen spanning the whole terminal.
    let screen = r.create_screen(ScreenAreaRect::FullScreen, 5);

    // Gentle animated background (kept subtle by Grayscale shader on the sprite video).
    let (tx, video_drawable) = make_videostream_drawable((0, 0), 1)?;
    let video_id = DrawObjectBuilder::default()
        .layer(1)
        .screen(screen)
        .drawable(video_drawable)
        .shader(Grayscale)
        .add_lifetime(ObjectLifetime::ExplicitRemove)
        .build(&mut r)?;
    spawn_soft_wave(tx, r.clone(), video_id, (cols, rows), bg_member);

    // Optional: tiny sprite clock in the corner (if you have a .ascv around, otherwise harmless).
    if let Ok(sprite) =
        r.register_sprite_from_source("./assets/debugging/test_video.ascv".to_string())
    {
        let spr_draw = SpriteDrawableBuilder::default()
            .sprite_id(sprite)
            .position((0, 50))
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
    let rect_id: DrawObjectKey = DrawObjectBuilder::default()
        .layer(100)
        .screen(screen)
        .rect_drawable(|r| {
            r.border_style(ltrender::terminal_buffer::standard_drawables::rect_drawable::BorderStyle::Custom { top: TerminalChar::from_char('T'), bottom: TerminalChar::from_char('B'), left: TerminalChar::from_char('L'), right: TerminalChar::from_char('R') })
                .border_thickness(1)
                .fill_style(' ')
                .rect(Rect::from_coords(20, 20, 40, 40))
        })?
        .add_lifetime(ObjectLifetime::ExplicitRemove)
        .build(&mut r)?;

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
    let mut mouse = MouseState {
        x: (cols / 2) as i32,
        y: (rows / 2) as i32,
        left: false,
        right: false,
        strength: 18.0,
    };
    let mut running = true;

    let mut render_start;
    let mut fps_vec: Vec<f64> = Vec::new();
    let mut time_since_last_fps_log = Instant::now();

    while running {
        render_start = Instant::now();
        // Input ----------------------------------------------------
        if poll(Duration::from_millis(0))? {
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

        r.render_drawable(rect_id);

        let frame_preperation_duration = render_start.elapsed();

        if show_hud {
            let fps = 1.0 / frame_preperation_duration.as_secs_f64();
            fps_vec.push(fps);

            if time_since_last_fps_log.elapsed() > Duration::from_secs(5) {
                let count = fps_vec.len();

                if count > 0 {
                    let sum: f64 = fps_vec.iter().sum();
                    let avg_fps = sum / count as f64;
                    info!("Avg FPS over the last 5s: {:.2} ", avg_fps);
                }
                fps_vec.clear();
                time_since_last_fps_log = Instant::now();
            }

            let txt = format!(
                "time spent rendering frame: ~{fps:.1} | agents: {} | force: {:.1} | mouse: ({}, {}) L:{} R:{}",
                agents.len(),
                mouse.strength,
                mouse.x,
                mouse.y,
                mouse.left as u8,
                mouse.right as u8
            );

            let hud = text_drawable::TextDrawable {
                area: Rect::from_coords(1, 1, 100, 6),
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
        main_member.wait_for_tick();
        r.render_frame();
    }

    // cleanup
    for a in &agents {
        r.explicit_remove_drawable(a.id);
    }
    r.explicit_remove_drawable(video_id);

    manager.stop();
    ltrender::restore_terminal()?;
    Ok(())
}

use ltrender::error::DrawError;
pub fn generate_sprites() -> Result<(), DrawError> {
    let width = 2;
    let height = 2;
    let frame1 = vec![
        TerminalChar {
            chr: 'A',
            fg_color: Some(Color {
                rgb: (255, 0, 0),
                reset: false,
            }),
            bg_color: None,
        },
        TerminalChar {
            chr: 'B',
            fg_color: None,
            bg_color: Some(Color {
                rgb: (0, 255, 0),
                reset: false,
            }),
        },
        TerminalChar {
            chr: 'C',
            fg_color: None,
            bg_color: None,
        },
        TerminalChar {
            chr: 'D',
            fg_color: None,
            bg_color: None,
        },
    ];

    let frame2 = vec![
        TerminalChar {
            chr: 'E',
            fg_color: None,
            bg_color: None,
        },
        TerminalChar {
            chr: 'F',
            fg_color: None,
            bg_color: None,
        },
        TerminalChar {
            chr: 'G',
            fg_color: None,
            bg_color: None,
        },
        TerminalChar {
            chr: 'H',
            fg_color: None,
            bg_color: None,
        },
    ];

    let video = AsciiVideo {
        width,
        height,
        frames: vec![
            AsciiSprite {
                pixels: frame1,
                width,
                height,
            },
            AsciiSprite {
                pixels: frame2,
                width,
                height,
            },
        ],
    };

    let path = "assets/debugging/test_video.ascv";

    video.write_to_file(path)?;
    Ok(())
}
