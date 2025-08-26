use std::time::Instant;

use crate::{
    DrawObject, DrawObjectKey, ScreenKey,
    drawable_register::ObjectLifetime,
    error::{AppError, DrawObjectBuilderError},
    renderer::RendererHandle,
    terminal_buffer::{Drawable, screen_buffer::Shader},
};
pub mod sprite_drawable_builder;
pub use sprite_drawable_builder::SpriteDrawableBuilder;
pub mod circle_drawable_builder;
pub use circle_drawable_builder::CircleDrawableBuilder;
pub mod line_drawable_builder;
pub use line_drawable_builder::LineDrawableBuilder;
pub mod polygon_drawable_builder;
pub use polygon_drawable_builder::PolygonDrawableBuilder;
pub mod rect_drawable_builder;
pub use rect_drawable_builder::RectDrawableBuilder;
pub mod videostream_drawable_builder;
pub use videostream_drawable_builder::VideoStreamDrawableBuilder;

macro_rules! allow_drawable_building {
    ($builder_name:ident, $fn_name:ident) => {
        pub fn $fn_name<F>(&mut self, build_fn: F) -> Result<&mut Self, DrawObjectBuilderError>
        where
            F: FnOnce($builder_name) -> $builder_name,
        {
            let builder = build_fn($builder_name::new());
            let drawable = builder.build()?;
            self.drawable = Some(drawable);
            Ok(self)
        }
    };
}

#[macro_export]
macro_rules! handle_field {
    ($fn_name:ident, $field_name:ident, $type:ty) => {
        pub fn $fn_name(&mut self, $field_name: $type) -> &mut Self {
            self.$field_name = Some($field_name);
            self
        }
    };
}

#[macro_export]
macro_rules! handle_pointed_field {
    ($fn_name:ident, $field_name:ident) => {
        pub fn $fn_name(mut self, $field_name: impl Into<Point<i32>>) -> Self {
            let point = $field_name.into();
            self.$field_name = Some(point);
            self
        }
    };
}

#[macro_export]
macro_rules! handle_char_field {
    ($fn_name:ident, $field_name:ident) => {
        pub fn $fn_name(mut self, $field_name: impl Into<TerminalChar>) -> Self {
            let point = $field_name.into();
            self.$field_name = Some(point);
            self
        }
    };
}

#[derive(Default)]
pub struct DrawObjectBuilder {
    layer: Option<usize>,
    shaders: Vec<Box<dyn Shader>>,
    drawable: Option<Box<dyn Drawable>>,
    screen_id: Option<ScreenKey>,
    lt: Option<ObjectLifetime>,
}

impl DrawObjectBuilder {
    handle_field!(layer, layer, usize);
    handle_field!(screen, screen_id, usize);
    handle_field!(add_lifetime, lt, ObjectLifetime);
    pub fn shader<T>(&mut self, shader: T) -> &mut Self
    where
        T: Shader + 'static,
    {
        self.shaders.push(Box::new(shader));
        self
    }

    pub fn drawable(&mut self, drawable: Box<dyn Drawable>) -> &mut Self {
        self.drawable = Some(drawable);
        self
    }

    allow_drawable_building!(SpriteDrawableBuilder, sprite_drawable);
    allow_drawable_building!(CircleDrawableBuilder, circle_drawable);
    allow_drawable_building!(LineDrawableBuilder, line_drawable);
    allow_drawable_building!(PolygonDrawableBuilder, polygon_drawable);
    allow_drawable_building!(RectDrawableBuilder, rect_drawable);
    allow_drawable_building!(VideoStreamDrawableBuilder, videostream_drawable);

    pub fn build(&mut self, render_handle: &mut RendererHandle) -> Result<DrawObjectKey, AppError> {
        let layer = self.layer.ok_or(DrawObjectBuilderError::NoLayerAdded())?;
        let screen_id = self
            .screen_id
            .ok_or(DrawObjectBuilderError::NoScreenAdded())?;

        let drawable = self
            .drawable
            .take()
            .ok_or(DrawObjectBuilderError::NoDrawableAdded())?;

        let lifetime = self
            .lt
            .take()
            .ok_or(DrawObjectBuilderError::NoLifetimeAdded())?;

        render_handle
            .register_drawable(
                screen_id,
                DrawObject {
                    lifetime,
                    layer,
                    drawable,
                    shaders: self.shaders.clone(),
                    creation_time: Instant::now(),
                },
            )
            .map_err(AppError::from)
    }
}
