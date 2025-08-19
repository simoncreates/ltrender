use crate::draw::{
    DrawObject, DrawableKey, ScreenKey,
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

macro_rules! handle_field {
    ($fn_name:ident, $field_name:ident) => {
        pub fn $fn_name(&mut self, $field_name: usize) -> &mut Self {
            self.$field_name = Some($field_name);
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
}

impl DrawObjectBuilder {
    pub fn new() -> Self {
        Self {
            layer: None,
            shaders: Vec::new(),
            drawable: None,
            screen_id: None,
        }
    }
    handle_field!(layer, layer);
    handle_field!(screen, screen_id);
    pub fn shader<T>(&mut self, shader: T) -> &mut Self
    where
        T: Shader + 'static,
    {
        self.shaders.push(Box::new(shader));
        self
    }
    pub fn drawable<T>(&mut self, drawable: T) -> &mut Self
    where
        T: Drawable + 'static,
    {
        self.drawable = Some(Box::new(drawable));
        self
    }

    allow_drawable_building!(SpriteDrawableBuilder, sprite_drawable);
    allow_drawable_building!(CircleDrawableBuilder, circle_drawable);
    allow_drawable_building!(LineDrawableBuilder, line_drawable);
    allow_drawable_building!(PolygonDrawableBuilder, polygon_drawable);
    allow_drawable_building!(RectDrawableBuilder, rect_drawable);

    pub fn build(&self, render_handle: &mut RendererHandle) -> Result<DrawableKey, AppError> {
        let layer = self.layer.ok_or(DrawObjectBuilderError::NoLayerAdded())?;

        let screen_id = self
            .screen_id
            .ok_or(DrawObjectBuilderError::NoScreenAdded())?;

        let drawable = self
            .drawable
            .as_ref()
            .ok_or(DrawObjectBuilderError::NoDrawableAdded())?;

        render_handle
            .register_drawable(
                screen_id,
                DrawObject {
                    layer,
                    drawable: drawable.clone_box(),
                    shaders: self.shaders.clone(),
                },
            )
            .map_err(AppError::from)
    }
}
