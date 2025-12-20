use crate::{
    DrawError, SpriteRegistry, drawable_traits::basic_draw_creator::BasicDrawCreator,
    update_interval_handler::UpdateIntervalCreator,
};

use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};
use log::warn;

pub mod basic_draw_creator;

#[derive(Debug, Clone, Copy)]
pub struct BasicDraw {
    pub pos: Point<i32>,
    pub chr: TerminalChar,
}

pub trait Drawable: std::fmt::Debug + Send {
    /// ### Info
    ///
    /// - Usually you will not need to use the SpriteRegistry
    /// - Also it is generally not recommended to mutate self during the draw function,
    ///   but has been added for flexibility reasons.
    ///
    fn draw(&mut self, sprites: &SpriteRegistry) -> Result<BasicDrawCreator, DrawError>;

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
    fn as_screen_fitting_mut(&mut self) -> Option<&mut dyn ScreenFitting> {
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

/// If implemented, this function will be ran, if a screens size changes.
///
/// ## Requirements:
/// Requires as_screen_fitting to be implemented in the Drawable trait
///
/// ## Example:
/// When implemententing a RectDrawable you can make this function just replace the internal rect field.
/// On the next Draw the RectangleDrawable will be displayed as having the same size as the screen
///
pub trait ScreenFitting {
    fn fit_to_screen(&mut self, rect: Rect<i32>);
}
