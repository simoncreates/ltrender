use common_stdx::Point;
use flume::{Receiver, Sender};

use crate::draw::{
    error::DrawObjectBuilderError,
    terminal_buffer::{
        Drawable,
        standard_drawables::{VideoStreamDrawable, videostream_drawable::StreamFrame},
    },
};

use crate::{handle_field, handle_pointed_field};

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
    handle_field!(size, size, (u16, u16));
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
        }))
    }
}

pub fn make_videostream_drawable(
    position: impl Into<Point<i32>>,
) -> Result<
    (
        Sender<StreamFrame>,
        Box<dyn crate::draw::terminal_buffer::Drawable>,
    ),
    crate::draw::error::DrawObjectBuilderError,
> {
    let (sender, receiver) = flume::unbounded::<StreamFrame>();

    let drawable = VideoStreamDrawableBuilder::new()
        .recv(receiver.clone())
        .position(position)
        .build()?;

    Ok((sender, drawable))
}
