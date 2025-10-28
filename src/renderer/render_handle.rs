use core::panic;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, SyncSender},
    },
    thread,
    time::Duration,
};

use common_stdx::Point;
use log::info;

#[cfg(feature = "screen_select_subscription")]
use crate::input_handler::screen_select_handler::ScreenSelectHandler;
use crate::{
    CrosstermEventManager, DrawError, DrawObject, DrawObjectKey, Renderer, ScreenBuffer, ScreenKey,
    SpriteId,
    display_screen::AreaRect,
    error::AppError,
    input_handler::manager::{EventHandler, MouseMessage, SubscriptionMessage, TargetScreen},
    renderer::RenderMode,
    terminal_buffer::{CellDrawer, Drawable},
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

    pub fn create_screen(&self, area: AreaRect, layer: usize) -> ScreenKey {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self
            .tx
            .send(RendererCommand::CreateScreen(area, layer, reply_tx));
        reply_rx.recv().unwrap()
    }

    pub fn change_screen_area(&self, screen_id: ScreenKey, new_area: AreaRect) {
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

    pub fn explicit_remove_drawable(&self, id: DrawObjectKey) {
        let _ = self.tx.send(RendererCommand::ExplicitRemoveDrawable(id));
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

    pub fn replace_drawable(&self, id: DrawObjectKey, drawable: Box<dyn Drawable>) {
        let _ = self.tx.send(RendererCommand::ReplaceDrawable(id, drawable));
    }

    pub fn move_drawable_by(&self, id: DrawObjectKey, dx: i32, dy: i32) {
        let _ = self.tx.send(RendererCommand::MoveDrawableBy(id, dx, dy));
    }

    pub fn render_frame(&self) {
        let _ = self.tx.send(RendererCommand::RenderFrame);
    }

    pub fn check_if_object_lifetime_ended(&self) {
        let _ = self.tx.send(RendererCommand::CheckIfObjectLifetimeEnded());
    }

    pub fn handle_resize(&self, new_size: (u16, u16)) {
        let _ = self.tx.send(RendererCommand::HandleResize(new_size));
    }

    pub fn get_terminal_size(&self) -> (u16, u16) {
        let (reply_tx, reply_rx) = mpsc::channel();
        let _ = self.tx.send(RendererCommand::GetCurrentSize(reply_tx));
        reply_rx.recv().unwrap()
    }
}

pub struct RendererManager {
    tx: mpsc::SyncSender<RendererCommand>,
    handle: Option<std::thread::JoinHandle<()>>,
    checker_handle: Option<std::thread::JoinHandle<()>>,
    checker_shutdown: Arc<AtomicBool>,
    event_mngr: Option<CrosstermEventManager>,
}

impl RendererManager {
    pub fn new<B>(size: (u16, u16), buffer_size: usize) -> (Self, RendererHandle)
    where
        B: ScreenBuffer + 'static,
        B::Drawer: CellDrawer + Send + 'static,
    {
        let (a, b, _) = RendererManager::standard_new::<B>(size, buffer_size, false);
        (a, b)
    }

    #[cfg(feature = "screen_select_subscription")]
    pub fn new_with_ev_handler<B>(
        size: (u16, u16),
        buffer_size: usize,
    ) -> (Self, RendererHandle, EventHandler)
    where
        B: ScreenBuffer + 'static,
        B::Drawer: CellDrawer + Send + 'static,
    {
        let (r_mngr, r_handle, opt_ev_handler) =
            RendererManager::standard_new::<B>(size, buffer_size, true);
        if let Some(ev_h) = opt_ev_handler {
            (r_mngr, r_handle, ev_h)
        } else {
            panic!("impossible")
        }
    }

    fn standard_new<B>(
        size: (u16, u16),
        buffer_size: usize,
        with_screen_sh: bool,
    ) -> (Self, RendererHandle, Option<EventHandler>)
    where
        B: ScreenBuffer + 'static,
        B::Drawer: CellDrawer + Send + 'static,
    {
        let (ev_h, screen_sh, event_mngr) = if with_screen_sh {
            let triple = CrosstermEventManager::new_with_select_sub(TargetScreen::None);
            (Some(triple.1), Some(triple.2), Some(triple.0))
        } else {
            (None, None, None)
        };

        let (tx, rx) = mpsc::sync_channel(buffer_size);
        let join_handle = run_renderer::<B>(size, rx, screen_sh);
        let shutdown = Arc::new(AtomicBool::new(false));
        let checker_join_handle = run_object_lifetime_checker(tx.clone(), shutdown.clone());

        let manager = RendererManager {
            tx: tx.clone(),
            handle: Some(join_handle),
            checker_handle: Some(checker_join_handle),
            event_mngr,
            checker_shutdown: shutdown,
        };
        let handle = RendererHandle { tx };

        (manager, handle, ev_h)
    }
}

impl Drop for RendererManager {
    fn drop(&mut self) {
        info!("dropping Event Manager");
        if let Some(mut m) = self.event_mngr.take() {
            m.stop();
        }

        info!("stopping lifetime checker");
        self.checker_shutdown.store(true, Ordering::Relaxed);
        if let Some(h) = self.checker_handle.take() {
            let _ = h.join();
        }

        info!("stopping renderer");
        if let Err(err) = self.tx.try_send(RendererCommand::Stop) {
            info!("gotten error while stopping renderer: {:?}", err);
        }

        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// used to check every 5 ms, if an object needs to be removed from screen
/// extits if the receiver is gone
/// TODO: make this code smarter
fn run_object_lifetime_checker(
    tx: SyncSender<RendererCommand>,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(5));
            if tx
                .send(RendererCommand::CheckIfObjectLifetimeEnded())
                .is_err()
            {
                break;
            }
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
        }
    })
}

pub fn run_renderer<B>(
    size: (u16, u16),
    rx: Receiver<RendererCommand>,
    screen_sh: Option<ScreenSelectHandler>,
) -> thread::JoinHandle<()>
where
    B: ScreenBuffer + 'static,
    B::Drawer: CellDrawer + 'static + Send,
{
    let screen_select_handler = Arc::new(Mutex::new(screen_sh));
    thread::spawn(move || {
        let mut r = Renderer::<B>::create_renderer(size);
        'main: loop {
            if let Ok(mut guard) = screen_select_handler.lock()
                && let Some(ssh) = &*guard
                && let Ok(sub_msg) = ssh.try_recv()
            {
                if ssh.send(None).is_err() {
                    *guard = None
                }
                if let SubscriptionMessage::Mouse { msg, .. } = sub_msg
                    && let MouseMessage::Pressed(b) = msg
                    && b.is_left()
                {
                    info!("pressed_lmb");
                }
            }
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    RendererCommand::CheckIfObjectLifetimeEnded() => {
                        let _ = r.check_if_object_lifetime_ended();
                    }
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
                    RendererCommand::ExplicitRemoveDrawable(id) => {
                        let _ = r.explicit_remove_drawable(&id);
                    }
                    RendererCommand::RemoveDrawable(id) => {
                        let _ = r.remove_drawable(id);
                    }
                    RendererCommand::ReplaceDrawable(id, drawable) => {
                        let _ = r.replace_drawable(id, drawable);
                    }
                    RendererCommand::RegisterSpriteFromSource(path, reply) => {
                        let result = r.register_sprite_from_source(&path);
                        let _ = reply.send(result);
                    }
                    RendererCommand::RenderDrawable(id) => {
                        let _ = r.render_drawable(id);
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
                    RendererCommand::GetCurrentSize(sender) => {
                        let res = r.terminal_size;
                        let _ = sender.send(res);
                    }
                    RendererCommand::RenderFrame => {
                        let _ = r.render_frame();
                    }

                    RendererCommand::Stop => break 'main,
                }
            }
        }
    })
}

#[derive(Debug)]
pub enum RendererCommand {
    SetRenderMode(RenderMode),
    SetUpdateInterval(usize),
    CreateScreen(AreaRect, usize, Sender<ScreenKey>),
    ChangeScreenArea(ScreenKey, AreaRect),
    ChangeScreenLayer(ScreenKey, usize),
    CheckIfObjectLifetimeEnded(),
    RegisterDrawable(
        ScreenKey,
        DrawObject,
        Sender<Result<DrawObjectKey, DrawError>>,
    ),
    ReplaceDrawable(DrawObjectKey, Box<dyn Drawable>),
    RemoveDrawable(DrawObjectKey),
    RegisterSpriteFromSource(String, Sender<Result<SpriteId, AppError>>),
    RenderDrawable(DrawObjectKey),
    ExplicitRemoveDrawable(DrawObjectKey),
    ReplaceDrawablePoints(DrawObjectKey, Vec<Point<i32>>),
    MoveDrawableTo(DrawObjectKey, Point<i32>),
    MoveDrawableBy(DrawObjectKey, i32, i32),
    MoveDrawablePoint(DrawObjectKey, usize, Point<i32>),
    MoveMultiPointDrawablePoint(DrawObjectKey, usize, Point<i32>),
    HandleResize((u16, u16)),
    GetCurrentSize(Sender<(u16, u16)>),
    RenderFrame,
    Stop,
}
