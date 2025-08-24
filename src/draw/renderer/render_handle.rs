use std::{
    sync::mpsc::{self, Receiver},
    thread,
};

use common_stdx::{Point, Rect};

use crate::draw::{
    DrawError, DrawObject, DrawObjectKey, Renderer, ScreenBuffer, ScreenKey, SpriteId,
    error::AppError, renderer::RenderMode,
};

#[derive(Clone, Debug)]
pub struct RendererHandle {
    tx: mpsc::SyncSender<RendererCommand>,
}

impl RendererHandle {
    pub fn set_update_interval(&self, amount: usize) {
        let _ = self.tx.send(RendererCommand::SetUpdateInterval(amount));
    }

    pub fn set_render_mode(&self, mode: RenderMode) {
        let _ = self.tx.send(RendererCommand::SetRenderMode(mode));
    }

    pub fn create_screen(&self, rect: Rect<i32>, layer: usize) -> ScreenKey {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self
            .tx
            .send(RendererCommand::CreateScreen(rect, layer, reply_tx));
        reply_rx.recv().unwrap()
    }

    pub fn change_screen_area(&self, screen_id: ScreenKey, new_area: Rect<i32>) {
        let _ = self
            .tx
            .send(RendererCommand::ChangeScreenArea(screen_id, new_area));
    }
    pub fn change_screen_layer(&self, screen_id: ScreenKey, new_layer: usize) {
        let _ = self
            .tx
            .send(RendererCommand::ChangeScreenLayer(screen_id, new_layer));
    }

    pub fn render_drawable(&self, id: DrawObjectKey) {
        let _ = self.tx.send(RendererCommand::RenderDrawable(id));
    }

    pub fn register_drawable(
        &self,
        screen_id: ScreenKey,
        obj: DrawObject,
    ) -> Result<DrawObjectKey, DrawError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self
            .tx
            .send(RendererCommand::RegisterDrawable(screen_id, obj, reply_tx));
        reply_rx.recv().unwrap()
    }

    pub fn register_sprite_from_source(&self, path: String) -> Result<SpriteId, AppError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self
            .tx
            .send(RendererCommand::RegisterSpriteFromSource(path, reply_tx));
        reply_rx.recv().unwrap()
    }

    pub fn replace_drawable_points(&self, id: DrawObjectKey, pts: Vec<Point<i32>>) {
        let _ = self
            .tx
            .send(RendererCommand::ReplaceDrawablePoints(id, pts));
    }

    pub fn move_drawable_by(&self, id: DrawObjectKey, dx: i32, dy: i32) {
        let _ = self.tx.send(RendererCommand::MoveDrawableBy(id, dx, dy));
    }

    pub fn render_frame(&self) {
        let _ = self.tx.send(RendererCommand::RenderFrame);
    }

    pub fn clear_terminal(&self) {
        let _ = self.tx.send(RendererCommand::ClearTerminal);
    }

    pub fn handle_resize(&self, new_size: (u16, u16)) {
        let _ = self.tx.send(RendererCommand::HandleResize(new_size));
    }
}

pub struct RendererManager {
    tx: mpsc::SyncSender<RendererCommand>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl RendererManager {
    pub fn new<B: ScreenBuffer + 'static>(
        size: (u16, u16),
        buffer_size: usize,
    ) -> (Self, RendererHandle) {
        let (tx, rx) = mpsc::sync_channel(buffer_size);
        let join_handle = run_renderer::<B>(size, rx);

        let manager = RendererManager {
            tx: tx.clone(),
            handle: Some(join_handle),
        };
        let handle = RendererHandle { tx };
        (manager, handle)
    }

    pub fn stop(mut self) {
        let _ = self.tx.send(RendererCommand::Stop);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for RendererManager {
    fn drop(&mut self) {
        let _ = self.tx.send(RendererCommand::Stop);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

pub fn run_renderer<B: ScreenBuffer + 'static>(
    size: (u16, u16),
    rx: Receiver<RendererCommand>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut r = Renderer::<B>::create_renderer(size);

        while let Ok(cmd) = rx.recv() {
            match cmd {
                RendererCommand::SetRenderMode(mode) => {
                    r.set_render_mode(mode);
                }
                RendererCommand::SetUpdateInterval(amount) => {
                    r.set_update_interval_expand_amount(amount);
                }
                RendererCommand::CreateScreen(rect, layer, reply) => {
                    let key = r.create_screen(rect, layer);
                    let _ = reply.send(key);
                }
                RendererCommand::ChangeScreenArea(id, new_area) => {
                    let _ = r.change_screen_area(id, new_area);
                }
                RendererCommand::ChangeScreenLayer(id, new_layer) => {
                    let _ = r.change_screen_layer(id, new_layer);
                }
                RendererCommand::RegisterDrawable(screen, obj, reply) => {
                    let result = r.register_drawable(screen, obj);
                    let _ = reply.send(result);
                }
                RendererCommand::RemoveDrawable(id) => {
                    let _ = r.remove_drawable(id);
                }
                RendererCommand::RegisterSpriteFromSource(path, reply) => {
                    let result = r.register_sprite_from_source(&path);
                    let _ = reply.send(result);
                }
                RendererCommand::RenderDrawable(id) => {
                    let _ = r.render_drawable(id);
                }
                RendererCommand::ReplaceDrawable(id, obj) => {
                    let _ = r.replace_drawable(id, obj);
                }
                RendererCommand::ReplaceDrawablePoints(id, pts) => {
                    let _ = r.replace_drawable_points(id, pts);
                }
                RendererCommand::MoveDrawableTo(id, pos) => {
                    let _ = r.move_drawable_to(id, pos);
                }
                RendererCommand::MoveDrawableBy(id, dx, dy) => {
                    let _ = r.move_drawable_by(id, dx, dy);
                }
                RendererCommand::MoveDrawablePoint(id, i, pos) => {
                    let _ = r.move_drawable_point(id, i, pos);
                }
                RendererCommand::MoveMultiPointDrawablePoint(id, i, pos) => {
                    let _ = r.move_multipoint_drawable_point(id, i, pos);
                }
                RendererCommand::HandleResize(new_size) => {
                    let _ = r.handle_resize(new_size);
                }
                RendererCommand::RenderFrame => {
                    let _ = r.render_frame();
                }
                RendererCommand::ClearTerminal => {
                    r.clear_terminal();
                }
                RendererCommand::Stop => break,
            }
        }
    })
}

#[derive(Debug)]
pub enum RendererCommand {
    SetRenderMode(RenderMode),
    SetUpdateInterval(usize),
    CreateScreen(Rect<i32>, usize, std::sync::mpsc::Sender<ScreenKey>),
    ChangeScreenArea(ScreenKey, Rect<i32>),
    ChangeScreenLayer(ScreenKey, usize),
    RegisterDrawable(
        ScreenKey,
        DrawObject,
        std::sync::mpsc::Sender<Result<DrawObjectKey, DrawError>>,
    ),
    RemoveDrawable(DrawObjectKey),
    RegisterSpriteFromSource(String, std::sync::mpsc::Sender<Result<SpriteId, AppError>>),
    RenderDrawable(DrawObjectKey),
    ReplaceDrawable(DrawObjectKey, DrawObject),
    ReplaceDrawablePoints(DrawObjectKey, Vec<Point<i32>>),
    MoveDrawableTo(DrawObjectKey, Point<i32>),
    MoveDrawableBy(DrawObjectKey, i32, i32),
    MoveDrawablePoint(DrawObjectKey, usize, Point<i32>),
    MoveMultiPointDrawablePoint(DrawObjectKey, usize, Point<i32>),
    HandleResize((u16, u16)),
    RenderFrame,
    ClearTerminal,
    Stop,
}
