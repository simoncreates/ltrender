use crate::ScreenBuffer;
use crate::display_screen::AreaRect;
use crate::drawable_register::ObjectLifetime;
use crate::terminal_buffer::CellDrawer;
use crate::terminal_buffer::drawable::Drawable;
use crate::{
    DrawError, DrawObject, DrawObjectKey, DrawObjectLibrary, Screen, ScreenKey, SpriteEntry,
    SpriteRegistry, error::AppError,
};
use ascii_assets::AsciiVideo;
use common_stdx::{Point, Rect};
use log::info;
use std::collections::HashMap;

pub type SpriteId = usize;
pub type ObjectId = usize;

#[derive(Debug, Clone, Copy)]
pub struct Instant;
#[derive(Debug, Clone, Copy)]
pub struct Buffered;

pub trait RenderModeBehavior {
    fn after_update<B: ScreenBuffer>(
        renderer: &mut Renderer<B, Self>,
        object_key: DrawObjectKey,
    ) -> Result<(), DrawError>
    where
        Self: Sized;

    fn refresh<B: ScreenBuffer>(renderer: &mut Renderer<B, Self>) -> Result<(), DrawError>
    where
        Self: Sized;

    fn render_all<B: ScreenBuffer>(renderer: &mut Renderer<B, Self>) -> Result<(), DrawError>
    where
        Self: Sized;
}

impl RenderModeBehavior for Instant {
    fn after_update<B: ScreenBuffer>(
        renderer: &mut Renderer<B, Self>,
        object_key: DrawObjectKey,
    ) -> Result<(), DrawError> {
        renderer.render_drawable(object_key)
    }
    fn refresh<B: ScreenBuffer>(renderer: &mut Renderer<B, Self>) -> Result<(), DrawError> {
        renderer.forced_refresh()?;
        Ok(())
    }

    /// Render all objects on all screens.
    fn render_all<B: ScreenBuffer>(renderer: &mut Renderer<B, Self>) -> Result<(), DrawError> {
        for screen in renderer.screens.values_mut() {
            screen.render_all(
                &mut renderer.screen_buffer,
                &mut renderer.obj_library,
                &renderer.sprites,
            )?;
        }
        Self::refresh(renderer)?;
        Ok(())
    }
}

impl RenderModeBehavior for Buffered {
    fn after_update<B: ScreenBuffer>(
        _renderer: &mut Renderer<B, Self>,
        _object_key: DrawObjectKey,
    ) -> Result<(), DrawError> {
        Ok(())
    }
    // noop, since buffered rendering does not need to refresh
    fn refresh<B: ScreenBuffer>(_renderer: &mut Renderer<B, Self>) -> Result<(), DrawError> {
        Ok(())
    }
    // might change up the impl here
    fn render_all<B: ScreenBuffer>(renderer: &mut Renderer<B, Self>) -> Result<(), DrawError> {
        for screen in renderer.screens.values_mut() {
            screen.render_all(
                &mut renderer.screen_buffer,
                &mut renderer.obj_library,
                &renderer.sprites,
            )?;
        }
        renderer.forced_refresh()?;
        Ok(())
    }
}

pub struct Renderer<B, M>
where
    B: ScreenBuffer,
    B::Drawer: CellDrawer,
{
    screens: HashMap<ScreenKey, Screen>,
    obj_library: DrawObjectLibrary,
    screen_buffer: B,
    sprites: SpriteRegistry,
    update_interval_expand_amount: usize,
    terminal_size: (u16, u16),
    _mode: std::marker::PhantomData<M>,
}

impl<B, M> Renderer<B, M>
where
    B: ScreenBuffer,
    B::Drawer: CellDrawer,
    M: RenderModeBehavior,
{
    /// set how aggrisive draw batches should try to merge
    pub fn set_update_interval(&mut self, amount: usize) {
        self.update_interval_expand_amount = amount;
    }

    pub fn get_terminal_size(&self) -> (u16, u16) {
        self.terminal_size
    }

    /// Create a new renderer with an initial terminal size.
    pub fn create_renderer(size: (u16, u16)) -> Self {
        Renderer::<B, M> {
            obj_library: DrawObjectLibrary::new(),
            screens: HashMap::new(),
            screen_buffer: B::new(size),
            sprites: SpriteRegistry::new(),
            update_interval_expand_amount: 50000,
            terminal_size: size,
            _mode: std::marker::PhantomData,
        }
    }

    /// Create a new screen and return its key.
    pub fn create_screen(&mut self, rect: AreaRect, layer: usize) -> ScreenKey {
        let new_id = self.generate_screen_key();
        self.screens
            .insert(new_id, Screen::new(rect, layer, new_id, self.terminal_size));
        new_id
    }

    pub fn change_screen_area(
        &mut self,
        screen_id: ScreenKey,
        new_area: AreaRect,
    ) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&screen_id) {
            s.remove_all(
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;
            s.change_screen_area(new_area);
            let ids = s.draw_objects.to_vec();
            let screen_rect = s.rect();

            for object_id in ids {
                self.run_drawable_screen_fitting(
                    DrawObjectKey {
                        screen_id,
                        object_id,
                    },
                    screen_rect,
                )?;
            }

            if let Some(s) = self.screens.get_mut(&screen_id) {
                s.render_all(
                    &mut self.screen_buffer,
                    &mut self.obj_library,
                    &self.sprites,
                )?;
            }
            M::refresh(self)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(screen_id))
        }
    }

    pub fn change_screen_layer(
        &mut self,
        screen_id: ScreenKey,
        new_layer: usize,
    ) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&screen_id) {
            s.remove_all(
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;
            s.change_screen_layer(new_layer);
            s.render_all(
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;
            M::refresh(self)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(screen_id))
        }
    }

    /// Register a drawable object on a screen.
    pub fn register_drawable(
        &mut self,
        screen_id: ScreenKey,
        obj: DrawObject,
    ) -> Result<DrawObjectKey, DrawError> {
        if let Some(s) = self.screens.get_mut(&screen_id) {
            let new_obj_id = self.obj_library.add_obj(screen_id, obj);
            s.register_drawable(new_obj_id, &self.obj_library);
            let area = s.rect();
            self.run_drawable_screen_fitting(
                DrawObjectKey {
                    object_id: new_obj_id,
                    screen_id,
                },
                area,
            )?;
            M::refresh(self)?;
            Ok(DrawObjectKey {
                screen_id,
                object_id: new_obj_id,
            })
        } else {
            Err(DrawError::DisplayKeyNotFound(screen_id))
        }
    }

    pub fn remove_drawable(&mut self, id: DrawObjectKey) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&id.screen_id) {
            s.remove_drawable(
                id.object_id,
                &mut self.screen_buffer,
                &mut self.obj_library,
                &self.sprites,
            )?;
            M::refresh(self)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(id.screen_id))
        }
    }

    pub fn explicit_remove_drawable(&mut self, id: &DrawObjectKey) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&id.screen_id) {
            s.deregister_drawable(id.object_id);
            self.remove_drawable(*id)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(id.screen_id))
        }
    }

    pub fn replace_drawable(
        &mut self,
        id: DrawObjectKey,
        drawable: Box<dyn Drawable>,
    ) -> Result<(), DrawError> {
        {
            if let Some(s) = self.screens.get_mut(&id.screen_id) {
                s.deregister_drawable(id.object_id);
            } else {
                return Err(DrawError::DisplayKeyNotFound(id.screen_id));
            }
        }

        self.remove_drawable(id)?;

        if let Some(obj) = self.obj_library.get_mut(&id) {
            obj.drawable = drawable;
        }

        {
            if let Some(s) = self.screens.get_mut(&id.screen_id) {
                s.register_drawable(id.object_id, &self.obj_library);
                s.render_drawable(
                    id.object_id,
                    &mut self.screen_buffer,
                    &mut self.obj_library,
                    &self.sprites,
                )?;
            }
        }

        Ok(())
    }

    /// checks if any of the currently existing drawobjects should be removed,
    /// because its duration on screen has ended
    pub fn check_if_object_lifetime_ended(&mut self) -> Result<(), DrawError> {
        let now = std::time::Instant::now();
        let mut expired_keys = Vec::new();

        for (screen_id, screen) in &self.screens {
            for obj_id in &screen.draw_objects {
                let key = &DrawObjectKey {
                    screen_id: *screen_id,
                    object_id: *obj_id,
                };

                if let Some(obj) = self.obj_library.all_objects.get(key)
                    && let ObjectLifetime::ForTime(dur) = obj.lifetime
                    && now >= obj.creation_time + dur
                {
                    expired_keys.push(*key);
                }
            }
        }

        for key in expired_keys {
            self.explicit_remove_drawable(&key)?;
        }

        Ok(())
    }

    /// Load a sprite from an ASCII video file.
    pub fn register_sprite_from_source(&mut self, path: &str) -> Result<SpriteId, AppError> {
        let video = AsciiVideo::read_from_file(path)?;

        let sprite_id = self.sprites.add(SpriteEntry { info: video });

        Ok(sprite_id)
    }

    /// Render a single drawable object.
    pub fn render_drawable(&mut self, object_key: DrawObjectKey) -> Result<(), DrawError> {
        if let Some(s) = self.screens.get_mut(&object_key.screen_id) {
            if let Some(obj) = self.obj_library.get_mut(&object_key) {
                obj.creation_time = std::time::Instant::now()
            }

            s.register_drawable(object_key.object_id, &self.obj_library);

            M::refresh(self)?;
            Ok(())
        } else {
            Err(DrawError::DisplayKeyNotFound(object_key.screen_id))
        }
    }

    fn forced_refresh(&mut self) -> Result<(), DrawError> {
        B::update_terminal(&mut self.screen_buffer, self.update_interval_expand_amount)?;
        Ok(())
    }

    pub fn handle_resize(&mut self, new_size: (u16, u16)) -> Result<(), DrawError> {
        self.screen_buffer = B::new(new_size);
        self.terminal_size = new_size;
        for screen in self.screens.values_mut() {
            screen.terminal_size = self.terminal_size
        }
        B::mark_all_dirty(&mut self.screen_buffer, new_size);
        M::render_all(self)?;
        Ok(())
    }

    /// Generate a unique screen key.
    pub fn generate_screen_key(&self) -> ScreenKey {
        let mut id = self.screens.len();
        while self.screens.contains_key(&id) {
            id = id.saturating_add(1);
        }
        id
    }

    pub fn move_drawable_to(
        &mut self,
        handle: DrawObjectKey,
        new_pos: Point<i32>,
    ) -> Result<(), DrawError> {
        self.get_drawable_mut(handle, |drawable| {
            if let Some(dp) = drawable.as_double_pointed_mut() {
                let old_start = dp.start();
                let old_end = dp.end();
                let dx = new_pos.x - old_start.x;
                let dy = new_pos.y - old_start.y;
                dp.set_start(new_pos);
                dp.set_end(Point {
                    x: (old_end.x + dx).clamp(0, u16::MAX as i32),
                    y: (old_end.y + dy).clamp(0, u16::MAX as i32),
                });
            } else if let Some(sp) = drawable.as_single_pointed_mut() {
                sp.set_position(new_pos);
            }
        })
    }

    pub fn move_drawable_by(
        &mut self,
        handle: DrawObjectKey,
        dx: i32,
        dy: i32,
    ) -> Result<(), DrawError> {
        self.get_drawable_mut(handle, |drawable| {
            if let Some(dp) = drawable.as_double_pointed_mut() {
                let start = dp.start();
                let end = dp.end();
                dp.set_start(Point {
                    x: (start.x + dx).clamp(0, i32::MAX),
                    y: (start.y + dy).clamp(0, i32::MAX),
                });
                dp.set_end(Point {
                    x: (end.x + dx).clamp(0, i32::MAX),
                    y: (end.y + dy).clamp(0, i32::MAX),
                });
            } else if let Some(sp) = drawable.as_single_pointed_mut() {
                let pos = sp.position();
                sp.set_position(Point {
                    x: (pos.x + dx).clamp(0, i32::MAX),
                    y: (pos.y + dy).clamp(0, i32::MAX),
                });
            }
        })
    }

    pub fn move_drawable_point(
        &mut self,
        handle: DrawObjectKey,
        point_index: usize,
        new_pos: Point<i32>,
    ) -> Result<(), DrawError> {
        self.get_drawable_mut(handle, |drawable| {
            if let Some(dp) = drawable.as_double_pointed_mut() {
                let clamped = Point {
                    x: (new_pos.x).clamp(0, i32::MAX),
                    y: (new_pos.y).clamp(0, i32::MAX),
                };
                match point_index {
                    0 => dp.set_start(clamped),
                    1 => dp.set_end(clamped),
                    _ => {}
                }
            }
        })
    }
    pub fn move_multipoint_drawable_point(
        &mut self,
        handle: DrawObjectKey,
        point_index: usize,
        new_pos: Point<i32>,
    ) -> Result<(), DrawError> {
        self.get_drawable_mut(handle, |drawable| {
            if let Some(mp) = drawable.as_multi_pointed_mut() {
                let clamped = Point {
                    x: (new_pos.x).clamp(0, i32::MAX),
                    y: (new_pos.y).clamp(0, i32::MAX),
                };
                mp.set_point(point_index, clamped);
            }
        })
    }
    /// replace a drawables points
    pub fn replace_drawable_points(
        &mut self,
        handle: DrawObjectKey,
        new_points: Vec<Point<i32>>,
    ) -> Result<(), DrawError> {
        self.get_drawable_mut(handle, |drawable| {
            if let Some(mp) = drawable.as_multi_pointed_mut() {
                mp.set_points(new_points.clone());
            } else if let Some(dp) = drawable.as_double_pointed_mut() {
                if new_points.len() == 2 {
                    dp.set_start(new_points[0]);
                    dp.set_end(new_points[1]);
                } else {
                    info!(
                        "Expected exactly two points for double-pointed drawable, got {}",
                        new_points.len()
                    );
                }
            } else if new_points.len() == 1 {
                if let Some(sp) = drawable.as_single_pointed_mut() {
                    sp.set_position(new_points[0]);
                } else {
                    info!("Drawable is not single-pointed");
                }
            } else {
                info!(
                    "Expected exactly one point for single-pointed drawable, got {}",
                    new_points.len()
                );
            }
        })
    }

    pub fn run_drawable_screen_fitting(
        &mut self,
        handle: DrawObjectKey,
        // not yet normalized to (0,0) as the top left corner
        unadjusted_screen_area: Rect<i32>,
    ) -> Result<(), DrawError> {
        let adjusted_area = Rect {
            p1: Point::from((0, 0)),
            p2: unadjusted_screen_area.p2 - unadjusted_screen_area.p1,
        };
        self.get_drawable_mut(handle, |drawable| {
            if let Some(sf) = drawable.as_screen_fitting_mut() {
                sf.fit_to_screen(adjusted_area);
            }
        })
    }

    pub fn get_amount_of_points(
        &mut self,
        handle: DrawObjectKey,
    ) -> Result<Option<usize>, DrawError> {
        self.get_drawable_mut(handle, |drawable| {
            if let Some(mp) = drawable.as_multi_pointed_mut() {
                Some(mp.points().len())
            } else if drawable.as_double_pointed_mut().is_some() {
                Some(2)
            } else if drawable.as_single_pointed_mut().is_some() {
                Some(1)
            } else {
                info!("Drawable is not multipointed");
                None
            }
        })
    }

    fn get_drawable_mut<F, R>(
        &mut self,
        object_key: DrawObjectKey,
        mut update_fn: F,
    ) -> Result<R, DrawError>
    where
        F: FnMut(&mut dyn Drawable) -> R,
        M: RenderModeBehavior,
    {
        self.remove_drawable(object_key)?;

        let result = {
            let obj =
                self.obj_library
                    .get_mut(&object_key)
                    .ok_or(DrawError::DrawableHandleNotFound {
                        screen_id: object_key.screen_id,
                        obj_id: object_key.object_id,
                    })?;

            update_fn(&mut *obj.drawable)
        };

        M::after_update(self, object_key)?;

        Ok(result)
    }

    pub fn into_buffered(self) -> Renderer<B, M> {
        Renderer {
            screens: self.screens,
            obj_library: self.obj_library,
            screen_buffer: self.screen_buffer,
            sprites: self.sprites,
            update_interval_expand_amount: self.update_interval_expand_amount,
            terminal_size: self.terminal_size,
            _mode: std::marker::PhantomData,
        }
    }

    pub fn into_instant(self) -> Renderer<B, M> {
        Renderer {
            screens: self.screens,
            obj_library: self.obj_library,
            screen_buffer: self.screen_buffer,
            sprites: self.sprites,
            update_interval_expand_amount: self.update_interval_expand_amount,
            terminal_size: self.terminal_size,
            _mode: std::marker::PhantomData,
        }
    }
}

impl<B, Buffered> Renderer<B, Buffered>
where
    B: ScreenBuffer,
    B::Drawer: CellDrawer,
    Buffered: RenderModeBehavior,
{
    pub fn render_frame(&mut self) -> Result<(), DrawError> {
        Buffered::render_all(self)?;

        self.remove_all_framebased_objects()?;
        Ok(())
    }

    /// removes all the objects from screen, that have frame-based removal
    pub fn remove_all_framebased_objects(&mut self) -> Result<(), DrawError> {
        let mut expired_keys = Vec::new();

        for (screen_id, screen) in &self.screens {
            for obj_id in &screen.draw_objects {
                let key = &DrawObjectKey {
                    screen_id: *screen_id,
                    object_id: *obj_id,
                };
                if let Some(obj) = self.obj_library.all_objects.get(key)
                    && let ObjectLifetime::RemoveNextFrame = obj.lifetime
                {
                    expired_keys.push(*key);
                }
            }
        }

        for key in expired_keys {
            self.explicit_remove_drawable(&key)?;
        }

        Ok(())
    }
}
