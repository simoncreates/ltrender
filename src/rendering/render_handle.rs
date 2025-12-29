use std::sync::mpsc;

use common_stdx::Point;

use crate::Drawable;
use crate::display_screen::AreaRect;
use crate::error::AppError;
use crate::rendering::renderer::{Buffered, Instant, RenderModeBehavior};
use crate::{DrawObject, DrawObjectKey, ScreenKey, SpriteId};

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
    FitScreenAreaToContents {
        screen_id: ScreenKey,
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
    IntoInstant,

    IntoBuffered,

    Shutdown,
}

#[derive(Debug, Clone)]
pub struct RenderHandle<M> {
    pub tx: mpsc::Sender<RenderCommand>,
    pub _mode: std::marker::PhantomData<M>,
}

impl<M> RenderHandle<M>
where
    M: RenderModeBehavior,
{
    fn send_and_wait<T>(
        &self,
        cmd_builder: impl FnOnce(mpsc::Sender<Result<T, AppError>>) -> RenderCommand,
    ) -> Result<T, AppError> {
        let (resp_tx, resp_rx) = mpsc::channel();
        let cmd = cmd_builder(resp_tx);
        self.tx.send(cmd).map_err(|_| AppError::SendError)?;

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

    pub fn fit_screen_area_to_contents(&self, screen_id: ScreenKey) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::FitScreenAreaToContents { screen_id, resp })
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

impl RenderHandle<Buffered> {
    pub fn render_frame(&self) -> Result<(), AppError> {
        self.send_and_wait(|resp| RenderCommand::RenderFrame { resp })
    }

    pub fn into_instant(&self) -> Result<RenderHandle<Instant>, AppError> {
        self.send(RenderCommand::IntoInstant)?;
        Ok(RenderHandle {
            tx: self.tx.clone(),
            _mode: std::marker::PhantomData,
        })
    }
}

impl RenderHandle<Instant> {
    pub fn into_buffered(&self) -> Result<RenderHandle<Buffered>, AppError> {
        self.send(RenderCommand::IntoBuffered)?;
        Ok(RenderHandle {
            tx: self.tx.clone(),
            _mode: std::marker::PhantomData,
        })
    }
}
