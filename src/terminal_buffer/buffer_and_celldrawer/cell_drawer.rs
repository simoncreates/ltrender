use crate::{
    DrawError,
    terminal_buffer::buffer_and_celldrawer::{BatchDrawInfo, screen_buffer::CellDrawerCommand},
};

use std::{fmt::Debug, sync::mpsc::Receiver};

/// Trait that describes how to write a string of chars
pub trait CellDrawer: Debug {
    fn init(rx: Receiver<CellDrawerCommand>) -> Self;

    /// Write a string at an absolute position.
    /// also used for drawing single character
    fn set_string(&mut self, batch: BatchDrawInfo, size: (u16, u16));

    /// Flush any buffered output to the terminal, or any other output that you might prefer
    fn flush(&mut self) -> Result<(), DrawError>;

    fn recv(&self) -> Result<CellDrawerCommand, std::sync::mpsc::RecvError>;
}
