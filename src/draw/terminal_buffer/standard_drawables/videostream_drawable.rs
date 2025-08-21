use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};
use flume::{Receiver, TryRecvError};
use std::collections::HashMap;

use crate::draw::{
    DrawError, SpriteRegistry, UpdateInterval,
    terminal_buffer::{
        Drawable,
        drawable::{BasicDraw, SinglePointed, convert_rect_to_update_intervals},
    },
};

#[derive(Debug, Clone)]
pub struct StreamFrame {
    pub size: (u16, u16),
    pub data: Vec<Option<TerminalChar>>,
}

#[derive(Debug, Clone)]
pub struct VideoStreamDrawable {
    pub position: Point<i32>,
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

    fn draw(&mut self, _: &SpriteRegistry) -> Result<Vec<BasicDraw>, DrawError> {
        match self.receiver.try_recv() {
            Ok(frame) => {
                self.size = frame.size;
                let width = self.size.0 as usize;
                let height = self.size.1 as usize;
                let cells = width * height;

                // avoid division/modulo by zero
                if width == 0 || height == 0 {
                    return Ok(Vec::new());
                }

                let mut draws: Vec<BasicDraw> = Vec::with_capacity(frame.data.len().min(cells));

                for (i, opt_ch) in frame.data.iter().enumerate() {
                    let idx = i % cells;
                    let x_off = (idx % width) as i32;
                    let y_off = (idx / width) as i32;

                    if let Some(char) = opt_ch {
                        draws.push(BasicDraw {
                            pos: Point {
                                x: self.position.x + x_off,
                                y: self.position.y + y_off,
                            },
                            chr: *char,
                        });
                    }
                }

                Ok(draws)
            }
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => Ok(Vec::new()),
        }
    }
    fn bounding_iv(&self, _: &SpriteRegistry) -> HashMap<i32, Vec<UpdateInterval>> {
        let rect = Rect {
            p1: self.position,
            p2: Point {
                x: self.position.x + self.size.0 as i32,
                y: self.position.y + self.size.1 as i32,
            },
        };

        convert_rect_to_update_intervals(rect)
    }

    fn shifted(&self, offset: Point<i32>) -> Box<dyn Drawable> {
        Box::new(VideoStreamDrawable {
            position: self.position + offset,
            receiver: self.receiver.clone(),
            size: self.size,
        })
    }
}
