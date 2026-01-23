use crate::{
    DrawError, SpriteRegistry,
    drawable_traits::basic_draw_creator::BasicDrawCreator,
    input_handler::manager::{KeyMessage, MouseMessage, TargetScreen},
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
    /// The Output of this function will be used to render to the terminal
    fn draw(&mut self, sprites: &SpriteRegistry) -> Result<BasicDrawCreator, DrawError>;

    /// Return an UpdateIntervalCreator
    ///
    /// UpdateIntervalCreator`s utility functions shall be used, to create the areas to be updated.
    ///
    /// if None is returned, a Standard imeplementation will be used, which may be less efficient.
    ///
    fn bounding_iv(&self, _sprites: &SpriteRegistry) -> Option<UpdateIntervalCreator> {
        None
    }

    fn size(&self, sprites: &SpriteRegistry) -> Result<(u16, u16), DrawError>;

    // if the screen that the drawable is on has been selected
    fn on_screen_select(&mut self, selected_screen: TargetScreen) -> Result<(), DrawError> {
        let _ = selected_screen;
        Ok(())
    }

    /// gets triggered, if any key with any selected screen has been pressed.
    /// To know, if the drawable is on this screen:
    ///     1. use the on_key_press function
    ///     2. pass the screen_id of the screen that the drawable is on to the drawable and check manually
    fn on_any_key_press(&mut self, msg: KeyMessage, screen: TargetScreen) -> Result<(), DrawError> {
        let _ = msg;
        let _ = screen;
        Ok(())
    }

    /// gets triggered, if a key has been pressed and the screen, which the drawable is on is selected.
    /// If you want react to global keypresses use the trait function 'on_any_keypress'
    fn on_key_press(&mut self, msg: KeyMessage) -> Result<(), DrawError> {
        let _ = msg;
        Ok(())
    }

    /// gets triggered, if a mouse_key has been pressed and the screen, which the drawable is on is selected.
    /// If you want react to global keypresses use the trait function 'on_any_keypress'
    fn on_mousekey_press(&mut self, msg: MouseMessage) -> Result<(), DrawError> {
        let _ = msg;
        Ok(())
    }

    /// gets triggered, if any mouse_key with any selected screen has been pressed.
    /// To know, if the drawable is on this screen:
    ///     1. use the on_mousekey_press function
    ///     2. pass the screen_id of the screen that the drawable is on to the drawable and check manually
    fn on_any_mousekey_press(
        &mut self,
        msg: MouseMessage,
        screen: TargetScreen,
    ) -> Result<(), DrawError> {
        let _ = msg;
        let _ = screen;
        Ok(())
    }

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
/// ## Example:
/// When implemententing a RectDrawable you can make this function just replace the internal rect field.
/// On the next Draw the RectangleDrawable will be displayed as having the same size as the screen
///
pub trait ScreenFitting {
    fn fit_to_screen(&mut self, rect: Rect<i32>);
}
