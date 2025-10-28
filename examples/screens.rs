use ascii_assets::TerminalChar;
use common_stdx::Rect;
use crossterm::{event::KeyCode, terminal::size};
use log::info;
use ltrender::{
    DrawObjectBuilder, ScreenKey,
    display_screen::AreaRect,
    drawable_register::ObjectLifetime,
    error::AppError,
    init_logger, init_terminal,
    input_handler::hook::InputButton,
    renderer::{RenderMode, RendererHandle, render_handle::RendererManager},
    restore_terminal,
    terminal_buffer::buffer_and_celldrawer::{CrosstermCellDrawer, DefaultScreenBuffer},
};

pub struct AdjustableScreens {
    screens: Vec<Vec<ScreenKey>>,
    current: usize,
}

impl AdjustableScreens {
    pub fn new_uniform_grid(
        amount_screens_x: usize,
        amount_screens_y: usize,
        r: &mut RendererHandle,
    ) -> Self {
        let mut screens = Vec::new();
        for _ in 0..amount_screens_y {
            let mut screen_row = Vec::new();
            for _ in 0..amount_screens_x {
                screen_row
                    .push(r.create_screen(AreaRect::FromPoints((0, 0).into(), (0, 0).into()), 0));
            }
            screens.push(screen_row);
        }
        let ad_screens = AdjustableScreens {
            screens,
            current: 0,
        };
        ad_screens.place_screens_uniformly(r);
        ad_screens
    }
    pub fn place_screens_uniformly(&self, r: &mut RendererHandle) {
        let current_size = r.get_terminal_size();
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
                );
            }
        }
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
    let (_manager, mut r, ev_handler) = RendererManager::new_with_ev_handler::<
        DefaultScreenBuffer<CrosstermCellDrawer>,
    >((cols, rows), 100);
    r.set_update_interval(1);
    r.set_render_mode(RenderMode::Instant);

    let hook = ev_handler.create_hook();

    let mut ad_screens = AdjustableScreens::new_uniform_grid(WIDTH, HEIGHT, &mut r);
    let mut builder = DrawObjectBuilder::default();

    for screen in &mut ad_screens {
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
            })?
            .add_lifetime(ObjectLifetime::ExplicitRemove)
            .screen(screen)
            .build(&mut r)?;
        info!("manually rendering drawable: {:?}", d_key);
        r.render_drawable(d_key);
    }
    loop {
        if hook.is_pressed(InputButton::Key(KeyCode::Char('c'))) {
            info!("breaking out of the loop");
            break;
        }

        let cur_size = size()?;
        let rendered_size = r.get_terminal_size();
        if cur_size != rendered_size {
            info!(
                "redrawing with cur_size: {:?}, rendered: {:?}",
                cur_size, rendered_size
            );
            r.handle_resize(cur_size);
            ad_screens.place_screens_uniformly(&mut r);
        }
    }
    restore_terminal()?;
    info!("returning with ok");
    Ok(())
}
