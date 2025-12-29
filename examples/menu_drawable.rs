use ascii_assets::{Color, TerminalChar};
use common_stdx::Rect;
use crossterm::{event::MouseButton, terminal::size};
use ltrender::{
    DrawObject, Renderer,
    display_screen::AreaRect,
    draw_object_builder::{CircleDrawableBuilder, RectDrawableBuilder},
    drawable_register::ObjectLifetime,
    error::AppError,
    init_logger, init_terminal,
    input_handler::hook::InputButton,
    rendering::{render_thread::start_renderer_with_input, renderer::Instant},
    terminal_buffer::{
        RectDrawable,
        buffer_and_celldrawer::{CrosstermCellDrawer, DefaultScreenBuffer},
        standard_drawables::{
            rect_drawable::BorderStyle,
            select_menu_drawable::{SelectMenuDrawable, SelectMenuFitting},
        },
    },
};

fn main() -> Result<(), AppError> {
    init_terminal!();
    init_logger("./menu_drawable_example.log")?;
    let (cols, rows) = size()?;
    let renderer = Renderer::<DefaultScreenBuffer<CrosstermCellDrawer>, Instant>::create_renderer(
        (cols, rows),
    );
    let (r, ev_handler, _ev_manager) = start_renderer_with_input(renderer);
    r.set_update_interval(1)?;

    let hook = ev_handler.create_hook();

    let inside_rect1 = RectDrawableBuilder::new()
        .border_style(BorderStyle::Basic)
        .fill_style(' ')
        .rect(Rect::from_coords(0, 0, 3, 3))
        .build()?;
    let inside_rect2 = RectDrawableBuilder::new()
        .border_style(BorderStyle::AllRound(TerminalChar::from_char('~')))
        .fill_style(' ')
        .rect(Rect::from_coords(0, 0, 3, 3))
        .build()?;
    let inside_circle = CircleDrawableBuilder::new()
        .border_style(TerminalChar::from_char('O').set_bg(Color::Maroon))
        .radius(4)
        .build()?;
    let border_rect = RectDrawable {
        rect: Rect::from_coords(0, 0, 0, 0),
        border_thickness: 1,
        border_style: BorderStyle::AllRound(TerminalChar::from_char('+').set_fg(Color::Fuchsia)),
        fill_style: Some(TerminalChar::from_char(' ')),
        screen_fit: None,
    };

    let menu_screen = r.create_screen(AreaRect::FullScreen, 0)?;

    let menu_drawable = SelectMenuDrawable::new(
        vec![inside_rect1, inside_rect2, inside_circle],
        border_rect,
        SelectMenuFitting::FitToContents,
        menu_screen,
        hook,
        r.clone(),
    )?;

    let drawable_id = r.register_drawable(
        menu_screen,
        DrawObject {
            creation_time: std::time::Instant::now(),
            drawable: Box::new(menu_drawable),
            layer: 0,
            lifetime: ObjectLifetime::ExplicitRemove,
            shaders: Vec::new(),
        },
    )?;
    r.render_drawable(drawable_id)?;
    let hook = ev_handler.create_hook();
    loop {
        if hook.is_pressed(InputButton::Mouse(MouseButton::Right)) {
            break;
        }
    }
    let _ = restore_terminal();
    Ok(())
}
