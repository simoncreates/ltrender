use std::{
    sync::mpsc::{self, Receiver},
    thread,
};

use common_stdx::{Point, Rect};

use crate::draw::{
    DrawError, DrawObject, DrawableKey, Renderer, ScreenBuffer, ScreenKey, SpriteId,
    error::AppError,
    renderer::RenderMode,
    terminal_buffer::standard_drawables::sprite_drawable::{AnimationInfo, FrameIdent},
};

pub struct RendererHandle {
    tx: mpsc::SyncSender<RendererCommand>,
    handle: std::thread::JoinHandle<()>,
}

impl RendererHandle {
    pub fn new<B: ScreenBuffer + 'static>(size: (u16, u16), buffer_size: usize) -> Self {
        // bounded channel
        let (tx, rx) = mpsc::sync_channel(buffer_size);
        let handle = run_renderer::<B>(size, rx);

        RendererHandle { tx, handle }
    }

    pub fn stop(self) {
        let _ = self.tx.send(RendererCommand::Stop);
        let _ = self.handle.join();
    }

    pub fn set_update_interval(&self, amount: usize) {
        let _ = self.tx.send(RendererCommand::SetUpdateInterval(amount));
    }

    pub fn create_screen(&self, rect: Rect<u16>, layer: usize) -> ScreenKey {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self
            .tx
            .send(RendererCommand::CreateScreen(rect, layer, reply_tx));
        reply_rx.recv().unwrap()
    }

    pub fn render_drawable(&self, id: DrawableKey) {
        let _ = self.tx.send(RendererCommand::RenderDrawable(id));
    }

    pub fn register_drawable(
        &self,
        screen_id: ScreenKey,
        obj: DrawObject,
    ) -> Result<DrawableKey, DrawError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self
            .tx
            .send(RendererCommand::RegisterDrawable(screen_id, obj, reply_tx));
        reply_rx.recv().unwrap()
    }

    pub fn register_sprite_drawable(
        &mut self,
        screen_id: ScreenKey,
        layer: usize,
        position: Point<u16>,
        sprite_id: SpriteId,
        frame: FrameIdent,
    ) -> Result<DrawableKey, AppError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self.tx.send(RendererCommand::RegisterSpriteDrawable(
            screen_id, layer, position, sprite_id, frame, reply_tx,
        ));
        reply_rx.recv().unwrap()
    }

    pub fn register_video_drawable(
        &mut self,
        screen_id: ScreenKey,
        layer: usize,
        position: Point<u16>,
        sprite_id: SpriteId,
        animation_info: AnimationInfo,
        frame: FrameIdent,
    ) -> Result<DrawableKey, AppError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self.tx.send(RendererCommand::RegisterVideoDrawable(
            screen_id,
            layer,
            position,
            sprite_id,
            animation_info,
            reply_tx,
        ));
        reply_rx.recv().unwrap()
    }

    pub fn register_sprite_from_source(&self, path: String) -> Result<SpriteId, AppError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self
            .tx
            .send(RendererCommand::RegisterSpriteFromSource(path, reply_tx));
        reply_rx.recv().unwrap()
    }

    pub fn replace_drawable_points(&self, id: DrawableKey, pts: Vec<Point<u16>>) {
        let _ = self
            .tx
            .send(RendererCommand::ReplaceDrawablePoints(id, pts));
    }

    pub fn move_drawable_by(&self, id: DrawableKey, dx: i32, dy: i32) {
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
                RendererCommand::RegisterDrawable(screen, obj, reply) => {
                    let result = r.register_drawable(screen, obj);
                    let _ = reply.send(result);
                }
                RendererCommand::RemoveDrawable(id) => {
                    let _ = r.remove_drawable(id);
                }
                RendererCommand::RegisterSpriteDrawable(s, l, pos, id, frame, reply) => {
                    let result = r.register_sprite_drawable(s, l, pos, id, frame);
                    let _ = reply.send(result.map_err(Into::into));
                }
                RendererCommand::RegisterVideoDrawable(
                    screen_id,
                    layer,
                    position,
                    sprite_id,
                    animation_info,
                    reply,
                ) => {
                    let result = r.register_video_drawable(
                        screen_id,
                        layer,
                        position,
                        sprite_id,
                        animation_info,
                    );
                    let _ = reply.send(result.map_err(Into::into));
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
    CreateScreen(Rect<u16>, usize, std::sync::mpsc::Sender<ScreenKey>),
    RegisterDrawable(
        ScreenKey,
        DrawObject,
        std::sync::mpsc::Sender<Result<DrawableKey, DrawError>>,
    ),
    RemoveDrawable(DrawableKey),
    RegisterSpriteDrawable(
        ScreenKey,
        usize,
        Point<u16>,
        SpriteId,
        FrameIdent,
        std::sync::mpsc::Sender<Result<DrawableKey, AppError>>,
    ),
    RegisterVideoDrawable(
        ScreenKey,
        usize,
        Point<u16>,
        SpriteId,
        AnimationInfo,
        std::sync::mpsc::Sender<Result<DrawableKey, AppError>>,
    ),
    RegisterSpriteFromSource(String, std::sync::mpsc::Sender<Result<SpriteId, AppError>>),
    RenderDrawable(DrawableKey),
    ReplaceDrawable(DrawableKey, DrawObject),
    ReplaceDrawablePoints(DrawableKey, Vec<Point<u16>>),
    MoveDrawableTo(DrawableKey, Point<u16>),
    MoveDrawableBy(DrawableKey, i32, i32),
    MoveDrawablePoint(DrawableKey, usize, Point<u16>),
    MoveMultiPointDrawablePoint(DrawableKey, usize, Point<u16>),
    HandleResize((u16, u16)),
    RenderFrame,
    ClearTerminal,
    Stop,
}
