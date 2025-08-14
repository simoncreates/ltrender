use std::{
    collections::HashMap,
    io::{Write, stdout},
};

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

#[derive(Debug)]
pub struct CrosstermScreenBuffer {
    cells: Vec<CharacterInfoList>,
    intervals: UpdateIntervalHandler,
    size: (u16, u16),
}
impl ScreenBufferCore for CrosstermScreenBuffer {
    fn cell_info_mut(&mut self) -> &mut Vec<CharacterInfoList> {
        &mut self.cells
    }

    fn intervals_mut(&mut self) -> &mut UpdateIntervalHandler {
        &mut self.intervals
    }

    fn size(&self) -> (u16, u16) {
        self.size
    }
}

impl CellDrawer for CrosstermScreenBuffer {
    fn set_string(
        &mut self,
        pos: Point<u16>,
        s: &str,
        fg_color: Option<Color>,
        bg_color: Option<Color>,
    ) {
        let mut out = stdout();
        queue!(
            out,
            MoveTo(pos.x, pos.y),
            SetForegroundColor(fg_color.unwrap_or(Color::Reset)),
            SetBackgroundColor(bg_color.unwrap_or(Color::Reset)),
            Print(s)
        )
        .expect("Failed to write to terminal");
    }

    fn flush(&mut self) -> Result<(), DrawError> {
        stdout().flush()?;
        Ok(())
    }
}

impl ScreenBuffer for CrosstermScreenBuffer {
    fn new(size: (u16, u16)) -> Self {
        let capacity = size.0 as usize * size.1 as usize;
        CrosstermScreenBuffer {
            cells: vec![
                CharacterInfoList {
                    info: HashMap::new()
                };
                capacity
            ],
            intervals: UpdateIntervalHandler::new(),
            size,
        }
    }
}
