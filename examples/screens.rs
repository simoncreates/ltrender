use std::{sync::mpsc, time::Duration};

use ascii_assets::TerminalChar;
use common_stdx::Rect;
use crossterm::{event::KeyCode, terminal::size};
use log::info;
use ltrender::{
    DrawObjectBuilder, Renderer, ScreenKey,
    display_screen::AreaRect,
    draw_object_builder::VideoStreamDrawableBuilder,
    drawable_register::ObjectLifetime,
    error::AppError,
    init_logger, init_terminal,
    input_handler::{
        hook::InputButton,
        manager::{MouseButtons, TargetScreen},
    },
    rendering::{
        render_handle::RenderHandle,
        render_thread::start_renderer_with_input,
        renderer::{Instant, RenderModeBehavior},
    },
    terminal_buffer::{
        buffer_and_celldrawer::{
            CrosstermCellDrawer, DefaultScreenBuffer,
            standard_celldrawer::test_celldrawer::TerminalContentInformation,
        },
        standard_drawables::{rect_drawable::ScreenFitType, videostream_drawable::StreamFrame},
    },
};

pub struct AdjustableScreens {
    screens: Vec<Vec<ScreenKey>>,
    current: usize,
}

impl AdjustableScreens {
    pub fn new_uniform_grid<M>(
        amount_screens_x: usize,
        amount_screens_y: usize,
        r: &mut RenderHandle<M>,
    ) -> Result<Self, AppError>
    where
        M: RenderModeBehavior,
    {
        let mut screens = Vec::new();
        for _ in 0..amount_screens_y {
            let mut screen_row = Vec::new();
            for _ in 0..amount_screens_x {
                let screen =
                    r.create_screen(AreaRect::FromPoints((0, 0).into(), (0, 0).into()), 0)?;
                screen_row.push(screen);
            }
            screens.push(screen_row);
        }
        let ad_screens = AdjustableScreens {
            screens,
            current: 0,
        };
        ad_screens.place_screens_uniformly(r)?;
        Ok(ad_screens)
    }
    pub fn place_screens_uniformly<M>(&self, r: &mut RenderHandle<M>) -> Result<(), AppError>
    where
        M: RenderModeBehavior,
    {
        let current_size = r.get_terminal_size()?;
        let screens_each_height = current_size.1 as usize / self.screens.len();
        for (i, screen_row) in &mut self.screens.iter().enumerate() {
            let screens_each_width = current_size.0 as usize / screen_row.len();
            for (j, screen) in screen_row.iter().enumerate() {
                r.change_screen_area(
                    *screen,
                    AreaRect::FromPoints(
                        (j * screens_each_width, i * screens_each_height).into(),
                        (
                            (j + 1) * screens_each_width - 1,
                            (i + 1) * screens_each_height - 1,
                        )
                            .into(),
                    ),
                )?;
            }
        }
        Ok(())
    }
}

impl Iterator for AdjustableScreens {
    type Item = ScreenKey;

    fn next(&mut self) -> Option<Self::Item> {
        let mut count = 0;

        for row in &self.screens {
            for &screen_id in row {
                if count == self.current {
                    self.current += 1;
                    return Some(screen_id);
                }
                count += 1;
            }
        }

        None
    }
}

const WIDTH: usize = 7;
const HEIGHT: usize = 7;

pub fn main() -> Result<(), AppError> {
    init_terminal!();
    init_logger("./screens_showcase.log")?;

    let (cols, rows) = size()?;
    let renderer = Renderer::<DefaultScreenBuffer<CrosstermCellDrawer>, Instant>::create_renderer(
        (cols, rows),
    );
    let (mut r, ev_handler, _ev_manager) = start_renderer_with_input(renderer);
    r.set_update_interval(1)?;

    let mut hook = ev_handler.create_hook();

    let mut ad_screens = AdjustableScreens::new_uniform_grid(WIDTH, HEIGHT, &mut r)?;

    for screen in &mut ad_screens {
        let builder = DrawObjectBuilder::default();
        let d_key = builder
            .layer(100)
            .rect_drawable(|r| {
                r.border_style(
                ltrender::terminal_buffer::standard_drawables::rect_drawable::BorderStyle::Custom {
                    top: TerminalChar::from_char('T'),
                    bottom: TerminalChar::from_char('B'),
                    left: TerminalChar::from_char('L'),
                    right: TerminalChar::from_char('R'),
                },
            )
            .border_thickness(1)
            .fill_style(' ')
            .rect(Rect::from_coords(0, 0, 2, 2))
            .screen_fit(ScreenFitType::Full)
            })?
            .add_lifetime(ObjectLifetime::ExplicitRemove)
            .screen(screen)
            .build(&mut r)?;
        info!("manually rendering drawable: {:?}", d_key);
        r.render_drawable(d_key)?;
    }

    let (s, recv) = mpsc::channel();
    let builder = DrawObjectBuilder::default();
    let text_stream = builder
        .layer(200)
        .videostream_drawable(|b| b.position((0, 0)).recv(recv))?
        .add_lifetime(ObjectLifetime::ExplicitRemove)
        .screen(0)
        .build(&mut r)?;
    let callback_hook = ev_handler.create_hook();
    let callback_renderh: RenderHandle<Instant> = r.clone();
    hook.on_mouse_button_press(MouseButtons::Left, move |_| {
        if let TargetScreen::Screen(id) = callback_hook.current_selected_screen() {
            let msg = format!("selected_screen: {}", id);
            info!("{}", msg);
            let mut data = Vec::new();
            for chr in msg.chars() {
                data.push(Some(TerminalChar::from_char(chr)));
            }
            let _ = s.send(StreamFrame {
                size: (data.len() as u16, 1),
                data,
            });
            let _ = callback_renderh.render_drawable(text_stream);
        }
    })?;
    loop {
        if hook.is_pressed(InputButton::Key(KeyCode::Char('c'))) {
            info!("breaking out of the loop");
            break;
        }

        let cur_size = size()?;
        let rendered_size = r.get_terminal_size()?;
        if cur_size != rendered_size {
            info!(
                "redrawing with cur_size: {:?}, rendered: {:?}",
                cur_size, rendered_size
            );
            r.handle_resize(cur_size)?;
            ad_screens.place_screens_uniformly(&mut r)?;
        }
    }
    restore_terminal()?;
    info!("returning with ok");
    Ok(())
}
