use ascii_assets::{AsciiSprite, AsciiVideo, Color, TerminalChar};
use common_stdx::{Point, Rect};
use crossterm::terminal::size;
use env_logger::Builder;
use log::info;
use ltrender::draw_object_builder::SpriteDrawableBuilder;
use ltrender::drawable_register::ObjectLifetime;
use ltrender::error::{AppError, DrawError};
use ltrender::renderer::render_handle::RendererManager;
use ltrender::terminal_buffer::buffer_and_celldrawer::standard_buffers::TestScreenBuffer;
use ltrender::terminal_buffer::buffer_and_celldrawer::standard_buffers::test_buffer::TerminalContentInformation;
use ltrender::terminal_buffer::standard_drawables::sprite_drawable::{
    AnimationInfo, FrameIdent, VideoLoopType, VideoSpeed,
};
use ltrender::{DrawObjectBuilder, get_test_data};

use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};
use std::{panic, thread, u16};

#[test]
fn video_render_test() -> Result<(), AppError> {
    //this hook is made by chatty
    panic::set_hook(Box::new(|info| {
        let thread = thread::current();
        let name = thread.name().unwrap_or("<unnamed>");
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("PANIC in thread '{}'", name));
        if let Some(location) = info.location() {
            lines.push(format!("panic at {}:{}", location.file(), location.line()));
        }
        match info.payload().downcast_ref::<&str>() {
            Some(s) => lines.push(format!("payload: {:?}", s)),
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => lines.push(format!("payload: {:?}", s)),
                None => lines.push("payload: <non-text>".to_string()),
            },
        }
        eprintln!("{}", lines.join(" "));
    }));

    ltrender::init_terminal()?;
    init_logger("./video_test.log")?;
    generate_sprites()?;

    let (cols, rows) = size()?;
    let (manager, mut r) = RendererManager::new::<TestScreenBuffer>((cols, rows), 600);
    r.set_render_mode(ltrender::renderer::RenderMode::Buffered);
    r.set_update_interval(16);

    let screen = r.create_screen(Rect::from_coords(0, 0, cols as i32, rows as i32), 5);
    expect_no_data();

    let obj_id = if let Ok(sprite) =
        r.register_sprite_from_source("./assets/debugging/test_video.ascv".to_string())
    {
        let spr_draw = SpriteDrawableBuilder::default()
            .sprite_id(sprite)
            .position((0, 0))
            .animation_type(AnimationInfo::Video {
                loop_type: VideoLoopType::KillOnFinish,
                speed: VideoSpeed::MillisecondsPerFrame(u16::MAX),
                start_frame: FrameIdent::FirstFrame,
                end_frame: FrameIdent::LastFrame,
            })
            .build()?;

        DrawObjectBuilder::default()
            .layer(2)
            .screen(screen)
            .drawable(spr_draw)
            .add_lifetime(ObjectLifetime::ExplicitRemove)
            .build(&mut r)?
    } else {
        panic!("failed to register sprite from source")
    };

    expect_no_data();

    // making sure flush hasnt been called
    test_if_screen_doesnt_exit();

    expect_no_data();

    // rendering and expecting to see the video
    r.render_frame();
    test_if_video_exist(Point { x: 0, y: 0 });

    // video should be removed, only after render frame has been called
    r.explicit_remove_drawable(obj_id);
    test_if_video_exist(Point { x: 0, y: 0 });
    r.render_frame();

    // expecting video to be removed
    test_if_eq_at_pos(Point::from((0, 0)), None);

    // rendering again
    r.render_drawable(obj_id);
    r.render_frame();
    test_if_video_exist(Point { x: 0, y: 0 });

    // changing the screen area, should only create change on the screen, after render frame has been called
    r.change_screen_area(screen, Rect::from_coords(1, 1, 20, 20));
    test_if_video_exist(Point { x: 0, y: 0 });
    // video should now be at 1,1
    r.render_frame();
    test_if_video_exist(Point { x: 1, y: 1 });

    //now adding a rect on a screen below the video, then moving it above
    // using 1,1 as position, since the video is also there
    let border_style = TerminalChar::from_char('#');

    let rect_screen = r.create_screen(Rect::from_coords(1, 1, cols as i32, rows as i32), 0);
    let rect_obj = DrawObjectBuilder::default()
        .add_lifetime(ObjectLifetime::ExplicitRemove)
        .layer(0)
        .rect_drawable(|r| {
            r.border_style(border_style)
                .rect(Rect::from_coords(0, 0, 1, 1))
                .border_thickness(1)
                .fill_style(TerminalChar::from_char(' '))
        })?
        .screen(rect_screen)
        .build(&mut r)?;

    // now the video should still be visible at 1,1
    test_if_video_exist(Point { x: 1, y: 1 });
    r.render_frame();
    // and now still
    test_if_video_exist(Point { x: 1, y: 1 });

    // changing the layer, now the rect should be above
    r.change_screen_layer(rect_screen, 10);
    r.render_frame();
    test_if_eq_at_pos(Point { x: 1, y: 1 }, Some(border_style));

    ltrender::restore_terminal()?;
    manager.stop();
    Ok(())
}

#[track_caller]
fn wait_for_test_data(timeout_ms: u64) -> Option<TerminalContentInformation> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);

    let baseline = get_test_data();
    while start.elapsed() < timeout {
        if let Some(current) = get_test_data() {
            if baseline.is_none() {
                return Some(current);
            }

            let base = baseline.as_ref().unwrap();
            let changed_by_cmds = current.amount_of_draw_commands != base.amount_of_draw_commands;
            let changed_by_content = current.content != base.content;

            if changed_by_cmds || changed_by_content {
                return Some(current);
            }
        }

        thread::sleep(Duration::from_millis(5));
    }

    get_test_data().or(baseline)
}

fn dump_test_data_brief(opt: Option<TerminalContentInformation>) -> String {
    match opt {
        None => "TerminalContentInformation: None".to_string(),
        Some(d) => {
            let mut lines = Vec::new();
            lines.push(format!(
                "size={:?}, content.len={}, changed={}, draw_cmds={}",
                d.size,
                d.content.len(),
                d.amount_of_changed_chars,
                d.amount_of_draw_commands
            ));

            let width = d.size.0 as usize;
            let height = d.size.1 as usize;
            if width == 0 || height == 0 {
                lines.push("preview: (empty)".to_string());
                return lines.join(" ").to_string();
            }

            let max_rows = std::cmp::min(4, height);
            let max_cols = std::cmp::min(8, width);
            lines.push(format!("preview (top-left {}x{}):", max_cols, max_rows));

            for row in 0..max_rows {
                let mut cols: Vec<String> = Vec::with_capacity(max_cols);
                for col in 0..max_cols {
                    let idx = row * width + col;
                    if idx < d.content.len() {
                        cols.push(format!("{:?}", &d.content[idx]));
                    } else {
                        cols.push(" ".to_string());
                    }
                }
                lines.push(format!("r{:02}: {}", row, cols.join(" | ")));
            }

            if width * height > max_cols * max_rows {
                lines.push(format!(
                    "  ({} more cells not shown)",
                    d.content.len() - (max_cols * max_rows)
                ));
            }

            lines.join(" ")
        }
    }
}
#[track_caller]
fn test_if_eq_at_pos(pos: Point<u16>, term_char: Option<TerminalChar>) {
    info!("testing if {:?} exists at position {:?}", term_char, pos);
    let opt_data = wait_for_test_data(10).or_else(get_test_data);

    if let Some(data) = opt_data {
        if data.content.is_empty() {
            panic!(
                "nothing drawn after last clearing. debug:{}",
                dump_test_data_brief(Some(data))
            );
        }

        let idx = data.size.0 as usize * pos.y as usize + pos.x as usize;
        if idx >= data.content.len() {
            panic!(
                "index out of bounds: idx={} len={} pos={:?} size={:?}{}",
                idx,
                data.content.len(),
                pos,
                data.size,
                dump_test_data_brief(Some(data.clone()))
            );
        }

        let actual = &data.content[idx];
        let info = format!(
            "mismatch at pos {:?}: expected={:?} found={:?}",
            pos, &term_char, &actual
        );
        if actual != &term_char {
            panic!("{}{}{}", info, "  ", dump_test_data_brief(Some(data)))
        }
    } else {
        panic!(
            "timed out waiting for TerminalContentInformation. snapshot:{}",
            dump_test_data_brief(get_test_data())
        );
    }
}
#[track_caller]
fn expect_no_data() {
    if let Some(data) = wait_for_test_data(10) {
        panic!(
            "there should be no TerminalContentInformation here, but got: {}",
            dump_test_data_brief(Some(data))
        );
    }
}

#[track_caller]
fn test_if_video_exist(point: Point<u16>) {
    let opt_data = wait_for_test_data(10);
    if opt_data.is_some() {
        let expected = Some(TerminalChar {
            chr: 'A',
            fg_color: Some(Color {
                rgb: (255, 0, 0),
                reset: false,
            }),
            bg_color: None,
        });
        test_if_eq_at_pos(point, expected);
    } else {
        panic!(
            "timed out waiting for TerminalContentInformation while expecting video. Current snapshot: {}",
            dump_test_data_brief(get_test_data())
        );
    }
}

#[track_caller]
fn test_if_screen_doesnt_exit() {
    if let Some(data) = wait_for_test_data(10) {
        if !data.content.is_empty() {
            panic!(
                "data should be empty but isn't. debug: {}",
                dump_test_data_brief(Some(data))
            )
        }
    }
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
