use std::{
    collections::HashMap,
    io::{Write, stdout},
};

use ascii_assets;
use common_stdx::Point;
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
};

use crate::draw::{
    DrawError, ScreenBuffer, UpdateIntervalHandler,
    terminal_buffer::{CellDrawer, CharacterInfoList, ScreenBufferCore},
};

use crate::default_screen_buffer;

default_screen_buffer!(CrosstermScreenBuffer);

pub fn to_crossterm_color(colour: Option<ascii_assets::Color>) -> crossterm::style::Color {
    if let Some(colour) = colour {
        if colour.reset {
            Color::Reset
        } else {
            let (r, g, b) = colour.rgb;
            crossterm::style::Color::Rgb { r, g, b }
        }
    } else {
        Color::Reset
    }
}

impl CellDrawer for CrosstermScreenBuffer {
    fn set_string(
        &mut self,
        pos: Point<u16>,
        s: &str,
        fg_color: Option<ascii_assets::Color>,
        bg_color: Option<ascii_assets::Color>,
    ) {
        let ct_fg_color = to_crossterm_color(fg_color);
        let ct_bg_color = to_crossterm_color(bg_color);

        queue!(
            self.out,
            MoveTo(pos.x, pos.y),
            SetForegroundColor(ct_fg_color),
            SetBackgroundColor(ct_bg_color),
            Print(s)
        )
        .expect("Failed to write to terminal");
    }

    fn flush(&mut self) -> Result<(), DrawError> {
        self.out.flush()?;
        Ok(())
    }
}
