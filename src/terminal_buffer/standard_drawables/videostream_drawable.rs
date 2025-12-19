use std::sync::mpsc::{Receiver, TryRecvError};

use crate::{
    DrawError, SpriteRegistry,
    terminal_buffer::{BasicDrawCreator, Drawable, drawable::SinglePointed},
    update_interval_handler::UpdateIntervalCreator,
};
use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};

#[derive(Debug, Clone)]
pub struct StreamFrame {
    pub size: (u16, u16),
    pub data: Vec<Option<TerminalChar>>,
}

#[derive(Debug)]
pub struct VideoStreamDrawable {
    pub position: Point<i32>,
    pub last_frame: StreamFrame,
    pub receiver: Receiver<StreamFrame>,
    pub size: (u16, u16),
}

impl SinglePointed for VideoStreamDrawable {
    fn position(&self) -> Point<i32> {
        self.position
    }
    fn set_position(&mut self, p: Point<i32>) {
        self.position = p;
    }
}

impl Drawable for VideoStreamDrawable {
    fn size(&self, _: &SpriteRegistry) -> Result<(u16, u16), DrawError> {
        Ok(self.size)
    }

    fn as_single_pointed_mut(&mut self) -> Option<&mut dyn SinglePointed> {
        Some(self)
    }

    fn draw(&mut self, _: &SpriteRegistry) -> Result<BasicDrawCreator, DrawError> {
        loop {
            match self.receiver.try_recv() {
                Ok(frame) => self.last_frame = frame,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ok(BasicDrawCreator::new()),
            };
        }
        let frame = &self.last_frame;
        self.size = frame.size;
        let width = self.size.0 as usize;
        let height = self.size.1 as usize;
        let cells = width * height;

        // avoid division/modulo by zero
        if width == 0 || height == 0 {
            return Ok(BasicDrawCreator::new());
        }

        let mut bd_creator = BasicDrawCreator::new_with_capacity(frame.data.len().min(cells));

        for (i, opt_ch) in frame.data.iter().enumerate() {
            let idx = i % cells;
            let x_off = (idx % width) as i32;
            let y_off = (idx / width) as i32;

            if let Some(chr) = opt_ch {
                bd_creator.draw_char((self.position.x + x_off, self.position.y + y_off), *chr);
            }
        }

        Ok(bd_creator)
    }
    fn bounding_iv(&self, _: &SpriteRegistry) -> Option<UpdateIntervalCreator> {
        let mut c = UpdateIntervalCreator::new();

        let rect = Rect {
            p1: self.position,
            p2: Point {
                x: self.position.x + self.size.0 as i32,
                y: self.position.y + self.size.1 as i32,
            },
        };

        c.register_redraw_region(rect);
        Some(c)
    }
}
