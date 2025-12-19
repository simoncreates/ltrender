use std::sync::mpsc::{self, Receiver};

use common_stdx::Point;

use crate::{
    error::DrawObjectBuilderError,
    terminal_buffer::{
        Drawable,
        standard_drawables::{VideoStreamDrawable, videostream_drawable::StreamFrame},
    },
};

use crate::handle_pointed_field;

#[derive(Default)]
pub struct VideoStreamDrawableBuilder {
    position: Option<Point<i32>>,
    receiver: Option<Receiver<StreamFrame>>,
    size: Option<(u16, u16)>,
}

impl VideoStreamDrawableBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    handle_pointed_field!(position, position);
    pub fn size(mut self, size: (u16, u16)) -> Self {
        self.size = Some(size);
        self
    }
    pub fn recv(mut self, receiver: Receiver<StreamFrame>) -> Self {
        self.receiver = Some(receiver);
        self
    }

    pub fn build(self) -> Result<Box<dyn Drawable>, DrawObjectBuilderError> {
        Ok(Box::new(VideoStreamDrawable {
            position: self.position.unwrap_or(Point { x: 0, y: 0 }),
            receiver: self
                .receiver
                .ok_or(DrawObjectBuilderError::FailedToBuildVideoStream())?,
            size: self.size.unwrap_or((0, 0)),
            last_frame: StreamFrame {
                size: (0, 0),
                data: Vec::new(),
            },
        }))
    }
}

pub fn make_videostream_drawable(
    position: impl Into<Point<i32>>,
    max_bounds: usize,
) -> Result<
    (
        mpsc::SyncSender<StreamFrame>,
        Box<dyn crate::terminal_buffer::Drawable>,
    ),
    crate::error::DrawObjectBuilderError,
> {
    let (sender, receiver) = mpsc::sync_channel::<StreamFrame>(max_bounds);

    let drawable = VideoStreamDrawableBuilder::new()
        .recv(receiver)
        .position(position)
        .build()?;

    Ok((sender, drawable))
}
