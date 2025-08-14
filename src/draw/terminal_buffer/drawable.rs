use crate::draw::{
    DrawError, SpriteRegistry, UpdateInterval, update_interval_handler::UpdateIntervalType,
};

use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};
use std::collections::HashMap;

pub fn convert_rect_to_update_intervals(rect: Rect<u16>) -> HashMap<u16, Vec<UpdateInterval>> {
    let mut intervals: HashMap<u16, Vec<UpdateInterval>> = HashMap::new();
    let iv = (rect.p1.x, rect.p2.x);

    for y in rect.p1.y..rect.p2.y {
        let intv = UpdateInterval {
            interval: iv,
            iv_type: UpdateIntervalType::Optimized,
        };
        intervals.entry(y).or_default().push(intv);
    }
    intervals
}

#[derive(Debug)]
pub struct BasicDraw {
    pub pos: Point<u16>,
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
    fn draw(&self, sprites: &SpriteRegistry) -> Result<Vec<BasicDraw>, DrawError>;

    /// Return a map of update intervals keyed by row.
    ///
    /// Each entry tells the renderer which horizontal segments of a given
    /// row need to be refreshed because this object occupies that area.
    /// Implementations typically call `convert_rect_to_update_intervals` with
    /// the drawable's bounding rectangle.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let size = sprites
    ///     .get(&self.sprite_id)
    ///     .map(|s| s.size())
    ///     .unwrap_or((0, 0, 0));
    /// convert_rect_to_update_intervals(Rect {
    ///     p1: self.position,
    ///     p2: Point {
    ///         x: self.position.x + size.1 as u16,
    ///         y: self.position.y + size.2 as u16,
    ///     },
    /// })
    /// ```
    fn bounding_iv(&self, sprites: &SpriteRegistry) -> HashMap<u16, Vec<UpdateInterval>>;

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
    /// ```rust
    /// // Assume we have a concrete implementation called `SpriteDrawable`
    /// let original = SpriteDrawable {
    ///     position: Point::new(0, 0),
    ///     sprite_id: 42,
    /// };
    /// let shifted = original.shifted(Point::new(10, 5));
    ///
    /// // The new drawable has the updated position.
    /// assert_eq!(shifted.position(), Point::new(10, 5));
    /// ```
    fn shifted(&self, offset: Point<u16>) -> Box<dyn Drawable>;

    fn as_double_pointed_mut(&mut self) -> Option<&mut dyn DoublePointed> {
        None
    }
    fn as_single_pointed_mut(&mut self) -> Option<&mut dyn SinglePointed> {
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
    /// The *current* position of the object.
    fn position(&self) -> Point<u16>;
    /// Change the stored position.
    fn set_position(&mut self, p: Point<u16>);
}

/// Two points (start & end).  The default implementation could simply
/// forward to `SinglePointed` if you want a single‑point shape to be usable
/// as a “first point” of a double‑point shape.
pub trait DoublePointed {
    /// Return the two defining points.
    fn start(&self) -> Point<u16>;
    fn end(&self) -> Point<u16>;

    /// Mutably change one of them.
    fn set_start(&mut self, p: Point<u16>);
    fn set_end(&mut self, p: Point<u16>);
}
