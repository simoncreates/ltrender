use std::sync::mpsc;
use std::thread;

use common_stdx::Point;

use crate::display_screen::AreaRect;
use crate::error::AppError;
use crate::renderer::RenderModeBehavior;
use crate::terminal_buffer::{CellDrawer, Drawable};
use crate::{DrawObject, DrawObjectKey, Renderer, ScreenBuffer, ScreenKey, SpriteId};

/// Commands that the renderer thread understands.
/// Each variant carries a responder (oneshot Sender) to send the result back.
pub enum RenderCommand {
    CreateScreen {
        rect: AreaRect,
        layer: usize,
        resp: mpsc::Sender<Result<ScreenKey, AppError>>,
    },
    ChangeScreenArea {
        screen_id: ScreenKey,
        new_area: AreaRect,
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    ChangeScreenLayer {
        screen_id: ScreenKey,
        new_layer: usize,
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    RegisterDrawable {
        screen_id: ScreenKey,
        obj: DrawObject,
        resp: mpsc::Sender<Result<DrawObjectKey, AppError>>,
    },
    RemoveDrawable {
        id: DrawObjectKey,
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    ExplicitRemoveDrawable {
        id: DrawObjectKey,
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    ReplaceDrawable {
        id: DrawObjectKey,
        drawable: Box<dyn Drawable + Send + 'static>,
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    RegisterSpriteFromSource {
        path: String,
        resp: mpsc::Sender<Result<SpriteId, AppError>>,
    },
    RenderDrawable {
        key: DrawObjectKey,
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    HandleResize {
        new_size: (u16, u16),
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    GetTerminalSize {
        resp: mpsc::Sender<Result<(u16, u16), AppError>>,
    },
    MoveDrawableTo {
        handle: DrawObjectKey,
        new_pos: Point<i32>,
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    MoveDrawableBy {
        handle: DrawObjectKey,
        dx: i32,
        dy: i32,
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    MoveDrawablePoint {
        handle: DrawObjectKey,
        point_index: usize,
        new_pos: Point<i32>,
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    GetAmountOfPoints {
        handle: DrawObjectKey,
        resp: mpsc::Sender<Result<Option<usize>, AppError>>,
    },
    RenderFrame {
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    ReplaceDrawablePoints {
        handle: DrawObjectKey,
        new_points: Vec<Point<i32>>,
        resp: mpsc::Sender<Result<(), AppError>>,
    },
    SetUpdateInterval {
        amount: usize,
    },
    Shutdown,
}

/// A clonable handle for producers to send commands to renderer thread.
#[derive(Clone)]
pub struct RenderHandle {
    tx: mpsc::Sender<RenderCommand>,
}

impl RenderHandle {
    /// helper to send a command and wait for the response
    fn send_and_wait<T>(
        &self,
        cmd_builder: impl FnOnce(mpsc::Sender<Result<T, AppError>>) -> RenderCommand,
    ) -> Result<T, AppError> {
        let (resp_tx, resp_rx) = mpsc::channel();
        let cmd = cmd_builder(resp_tx);
        // send the command to the renderer thread
        self.tx.send(cmd).map_err(|_| AppError::SendError)?;

        // wait for reply
        resp_rx.recv().map_err(|_| AppError::RecvError)?
    }

    fn send(&self, cmd: RenderCommand) -> Result<(), AppError> {
        self.tx.send(cmd).map_err(|_| AppError::SendError)
    }

    pub fn create_screen(&self, rect: AreaRect, layer: usize) -> Result<ScreenKey, AppError> {
        self.send_and_wait(|resp| RenderCommand::CreateScreen { rect, layer, resp })
    }

    pub fn change_screen_area(
        &self,
        screen_id: ScreenKey,
        new_area: AreaRect,
    ) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::ChangeScreenArea {
            screen_id,
            new_area,
            resp,
        })
    }

    pub fn change_screen_layer(
        &self,
        screen_id: ScreenKey,
        new_layer: usize,
    ) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::ChangeScreenLayer {
            screen_id,
            new_layer,
            resp,
        })
    }

    pub fn register_drawable(
        &self,
        screen_id: ScreenKey,
        obj: DrawObject,
    ) -> Result<DrawObjectKey, AppError> {
        self.send_and_wait(|resp| RenderCommand::RegisterDrawable {
            screen_id,
            obj,
            resp,
        })
    }

    pub fn remove_drawable(&self, id: DrawObjectKey) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::RemoveDrawable { id, resp })
    }

    pub fn replace_drawable(
        &self,
        id: DrawObjectKey,
        drawable: Box<dyn Drawable + Send + 'static>,
    ) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::ReplaceDrawable { id, drawable, resp })
    }

    pub fn register_sprite_from_source(&self, path: &str) -> Result<SpriteId, AppError> {
        let path = path.to_string();
        self.send_and_wait(|resp| RenderCommand::RegisterSpriteFromSource { path, resp })
    }

    pub fn render_drawable(&self, key: DrawObjectKey) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::RenderDrawable { key, resp })
    }

    pub fn handle_resize(&self, new_size: (u16, u16)) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::HandleResize { new_size, resp })
    }

    pub fn get_terminal_size(&self) -> Result<(u16, u16), AppError> {
        self.send_and_wait(|resp| RenderCommand::GetTerminalSize { resp })
    }

    pub fn move_drawable_to(
        &self,
        handle: DrawObjectKey,
        new_pos: Point<i32>,
    ) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::MoveDrawableTo {
            handle,
            new_pos,
            resp,
        })
    }

    pub fn move_drawable_by(
        &self,
        handle: DrawObjectKey,
        dx: i32,
        dy: i32,
    ) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::MoveDrawableBy {
            handle,
            dx,
            dy,
            resp,
        })
    }

    pub fn move_drawable_point(
        &self,
        handle: DrawObjectKey,
        point_index: usize,
        new_pos: Point<i32>,
    ) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::MoveDrawablePoint {
            handle,
            point_index,
            new_pos,
            resp,
        })
    }

    pub fn get_amount_of_points(&self, handle: DrawObjectKey) -> Result<Option<usize>, AppError> {
        self.send_and_wait(|resp| RenderCommand::GetAmountOfPoints { handle, resp })
    }

    pub fn render_frame(&self) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::RenderFrame { resp })
    }

    pub fn set_update_interval(&self, amount: usize) -> Result<(), AppError> {
        self.send(RenderCommand::SetUpdateInterval { amount })?;
        Ok(())
    }

    pub fn replace_drawable_points(
        &mut self,
        handle: DrawObjectKey,
        new_points: Vec<Point<i32>>,
    ) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::ReplaceDrawablePoints {
            handle,
            new_points,
            resp,
        })?;
        Ok(())
    }

    pub fn explicit_remove_drawable(&self, id: &DrawObjectKey) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::ExplicitRemoveDrawable { id: *id, resp })
    }

    /// ask thread to shutdown (no response)
    pub fn shutdown(&self) {
        // ignore error: means receiver already dropped
        let _ = self.tx.send(RenderCommand::Shutdown);
    }
}

/// spawn a dedicated renderer thread and return a clonable handle
///
/// NOTE: Renderer and all its contained types MUST be `Send + 'static`.
pub fn start_renderer_thread<B, M>(mut renderer: Renderer<B, M>) -> RenderHandle
where
    B: ScreenBuffer + Send + 'static,
    B::Drawer: CellDrawer + Send + 'static,
    M: RenderModeBehavior + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<RenderCommand>();

    // spawn the renderer loop
    thread::spawn(move || {
        for cmd in rx {
            match cmd {
                RenderCommand::CreateScreen { rect, layer, resp } => {
                    // create_screen returns ScreenKey (not a Result)
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
                RenderCommand::Shutdown => break,
            }
        }

        // thread exits when channel closed or Shutdown received
    });

    RenderHandle { tx }
}
