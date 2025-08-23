use crate::draw::{DrawError, SpriteRegistry, update_interval_handler::UpdateIntervalCreator};

use ascii_assets::TerminalChar;
use common_stdx::Point;
use log::warn;

#[derive(Debug, Clone, Copy)]
pub struct BasicDraw {
    pub pos: Point<i32>,
    pub chr: TerminalChar,
}

pub trait Cloneable {
    fn clone_box(&self) -> Box<dyn Drawable>;
}

impl<T> Cloneable for T
where
    T: 'static + Drawable + Clone,
{
    fn clone_box(&self) -> Box<dyn Drawable> {
        Box::new(self.clone())
    }
}

pub trait Drawable: Cloneable + std::fmt::Debug + Send {
    /// Render this object into a list of terminal cells.
    ///
    /// The method should not perform any clipping, layering or buffer
    /// management; it only needs to return the raw `(pos, chr)` pairs that
    /// represent what would be drawn on screen.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let sprite = SpriteDrawable { position: Point::new(0, 0), sprite_id: 1 };
    /// let draws = sprite.draw(&all_sprites).unwrap();
    /// ```
    ///
    /// ### Info
    ///
    /// - Usually you will not need to use the SpriteRegistry
    /// - Also it is generally not recommended to mutate self during the draw function,
    ///   but has been added for flexibility reasons.
    ///
    fn draw(&mut self, sprites: &SpriteRegistry) -> Result<Vec<BasicDraw>, DrawError>;

    /// Return an UpdateIntervalCreator
    ///
    /// UpdateIntervalCreator`s utility functions shall be used, to create the areas to be updated.
    ///
    /// if None is returned, a Standard imeplementation will be used, which is very inefficient.
    ///
    fn bounding_iv(&self, _sprites: &SpriteRegistry) -> Option<UpdateIntervalCreator> {
        None
    }

    /// Return **a new** drawable that has been shifted by the given offset.
    ///
    /// The returned value is boxed as a trait object
    ///
    /// ## Parameters
    ///
    /// * `offset` – A point indicating how far to shift the current position.
    ///
    /// ## Returns
    ///
    /// A boxed trait object that implements `Drawable`.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use crate::draw::{
    /// #    terminal_buffer::{
    /// #        Drawable,
    /// #        drawable::SpriteDrawable,
    /// #    },
    /// # };
    /// # use common_stdx::Point;
    ///
    /// let original = SpriteDrawable {
    ///     position: Point { x: 0, y: 0 },
    ///     sprite_id: 42,
    /// };
    ///
    /// let shifted_drawable = original.shifted(Point { x: 10, y: 5 });
    ///
    /// let shifted = shifted_drawable
    ///     .downcast::<SpriteDrawable>()
    ///     .unwrap();
    ///
    /// assert_eq!(shifted.position, Point { x: 10, y: 5 });
    /// ```
    fn shifted(&self, offset: Point<i32>) -> Box<dyn Drawable>;

    fn size(&self, sprites: &SpriteRegistry) -> Result<(u16, u16), DrawError>;

    fn get_top_left(&mut self) -> Option<Point<i32>> {
        if self.as_double_pointed_mut().is_some() {
            Some(self.as_double_pointed_mut().unwrap().start())
        } else if self.as_single_pointed_mut().is_some() {
            Some(self.as_single_pointed_mut().unwrap().position())
        } else {
            warn!("drawable has not been given an explicit top left corner");
            warn!("to fix, implement get_top_left");
            None
        }
    }

    fn as_double_pointed_mut(&mut self) -> Option<&mut dyn DoublePointed> {
        None
    }
    fn as_single_pointed_mut(&mut self) -> Option<&mut dyn SinglePointed> {
        None
    }
    fn as_multi_pointed_mut(&mut self) -> Option<&mut dyn MultiPointed> {
        None
    }
    fn as_multi_pointed(&self) -> Option<&dyn MultiPointed> {
        None
    }
}

impl Clone for Box<dyn Drawable> {
    fn clone(&self) -> Box<dyn Drawable> {
        self.clone_box()
    }
}

// extensions:

/// One point that can be read / written.
pub trait SinglePointed {
    /// The current position of the object.
    fn position(&self) -> Point<i32>;
    /// Change the stored position.
    fn set_position(&mut self, p: Point<i32>);
}

/// Two points (start & end)
pub trait DoublePointed {
    /// Return the two defining points.
    fn start(&self) -> Point<i32>;
    fn end(&self) -> Point<i32>;

    /// Mutably change one of them.
    fn set_start(&mut self, p: Point<i32>);
    fn set_end(&mut self, p: Point<i32>);
}

/// A collection of points
/// intended for general shapes, usually is implemented
/// when a variable amount of points and or more than two Points are needed for a drawable
pub trait MultiPointed {
    /// get a point at the given index
    fn get_point(&self, idx: usize) -> Option<Point<i32>>;
    /// set a point at the given index
    fn set_point(&mut self, idx: usize, p: Point<i32>);
    /// get all current points
    fn points(&self) -> &[Point<i32>];
    /// replace all current points
    fn set_points(&mut self, points: Vec<Point<i32>>);
}
