use crate::{DrawError, SpriteRegistry, update_interval_handler::UpdateIntervalCreator};

use ascii_assets::TerminalChar;
use common_stdx::Point;
use log::warn;

#[derive(Debug, Clone, Copy)]
pub struct BasicDraw {
    pub pos: Point<i32>,
    pub chr: TerminalChar,
}

pub trait Drawable: std::fmt::Debug + Send {
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
