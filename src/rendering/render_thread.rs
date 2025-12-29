use std::{sync::mpsc, thread};

#[cfg(feature = "screen_select_subscription")]
use crate::CrosstermEventManager;
use crate::{
    Renderer, ScreenBuffer,
    input_handler::{hook::EventHook, manager::EventHandler},
    rendering::{
        render_handle::{RenderCommand, RenderHandle},
        renderer::RenderModeBehavior,
    },
    terminal_buffer::CellDrawer,
};

fn start_thread<B, M>(
    mut renderer: Renderer<B, M>,
    event_hook: Option<EventHook>,
) -> RenderHandle<M>
where
    B: ScreenBuffer + Send + 'static,
    B::Drawer: CellDrawer + Send + 'static,
    M: RenderModeBehavior + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<RenderCommand>();
    // spawn the renderer loop
    thread::spawn(move || {
        loop {
            let _ = renderer.remove_all_framebased_objects();
            if let Some(hook) = &event_hook {
                let _ = renderer.handle_screen_selection(hook);
            }
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    RenderCommand::CreateScreen { rect, layer, resp } => {
                        // create_screen returns ScreenKey
                        let key = renderer.create_screen(rect, layer);
                        let _ = resp.send(Ok(key));
                    }
                    RenderCommand::ChangeScreenArea {
                        screen_id,
                        new_area,
                        resp,
                    } => {
                        let res = renderer
                            .change_screen_area(screen_id, new_area)
                            .map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::FitScreenAreaToContents { screen_id, resp } => {
                        let res = renderer
                            .fit_screen_area_to_contents(screen_id)
                            .map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::ChangeScreenLayer {
                        screen_id,
                        new_layer,
                        resp,
                    } => {
                        let res = renderer
                            .change_screen_layer(screen_id, new_layer)
                            .map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::RegisterDrawable {
                        screen_id,
                        obj,
                        resp,
                    } => {
                        let res = renderer
                            .register_drawable(screen_id, obj)
                            .map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::RemoveDrawable { id, resp } => {
                        let res = renderer.remove_drawable(id).map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::ExplicitRemoveDrawable { id, resp } => {
                        let res = renderer.explicit_remove_drawable(&id).map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::ReplaceDrawable { id, drawable, resp } => {
                        let res = renderer.replace_drawable(id, drawable).map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::RegisterSpriteFromSource { path, resp } => {
                        // note: this returns AppError
                        let res = renderer.register_sprite_from_source(&path);
                        let _ = resp.send(res);
                    }
                    RenderCommand::RenderDrawable { key, resp } => {
                        let res = renderer.render_drawable(key).map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::HandleResize { new_size, resp } => {
                        let res = renderer.handle_resize(new_size).map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::GetTerminalSize { resp } => {
                        let size = renderer.get_terminal_size();
                        let _ = resp.send(Ok(size));
                    }
                    RenderCommand::MoveDrawableTo {
                        handle,
                        new_pos,
                        resp,
                    } => {
                        let res = renderer
                            .move_drawable_to(handle, new_pos)
                            .map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::MoveDrawableBy {
                        handle,
                        dx,
                        dy,
                        resp,
                    } => {
                        let res = renderer
                            .move_drawable_by(handle, dx, dy)
                            .map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::MoveDrawablePoint {
                        handle,
                        point_index,
                        new_pos,
                        resp,
                    } => {
                        let res = renderer
                            .move_drawable_point(handle, point_index, new_pos)
                            .map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::GetAmountOfPoints { handle, resp } => {
                        let res = renderer.get_amount_of_points(handle).map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::RenderFrame { resp } => {
                        let res = renderer.render_frame().map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::ReplaceDrawablePoints {
                        handle,
                        new_points,
                        resp,
                    } => {
                        let res = renderer
                            .replace_drawable_points(handle, new_points)
                            .map_err(Into::into);
                        let _ = resp.send(res);
                    }
                    RenderCommand::SetUpdateInterval { amount } => {
                        renderer.set_update_interval(amount);
                    }
                    RenderCommand::IntoInstant => {
                        renderer = renderer.into_instant();
                    }
                    RenderCommand::IntoBuffered => {
                        renderer = renderer.into_buffered();
                    }
                    RenderCommand::Shutdown => break,
                }
            }
        }
    });

    RenderHandle {
        tx,
        _mode: std::marker::PhantomData,
    }
}

pub fn start_renderer<B, M>(renderer: Renderer<B, M>) -> RenderHandle<M>
where
    B: ScreenBuffer + Send + 'static,
    B::Drawer: CellDrawer + Send + 'static,
    M: RenderModeBehavior + Send + 'static,
{
    start_thread(renderer, None)
}
#[cfg(feature = "screen_select_subscription")]
pub fn start_renderer_with_input<B, M>(
    mut renderer: Renderer<B, M>,
) -> (RenderHandle<M>, EventHandler, CrosstermEventManager)
where
    B: ScreenBuffer + Send + 'static,
    B::Drawer: CellDrawer + Send + 'static,
    M: RenderModeBehavior + Send + 'static,
{
    use crate::{CrosstermEventManager, input_handler::manager::TargetScreen};

    let (event_mngr, event_handler, screen_sel_handler) =
        CrosstermEventManager::new_with_select_sub(TargetScreen::None);
    renderer.add_screen_select_handler(screen_sel_handler);
    (
        start_thread(renderer, Some(event_handler.create_hook())),
        event_handler,
        event_mngr,
    )
}
