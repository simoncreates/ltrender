use thiserror::Error;

use crate::{
    SpriteId,
    input_handler::manager::{EventManagerCommand, SubscriptionMessage},
};
use std::fmt::Debug;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    File(#[from] FileError),

    #[error(transparent)]
    DrawObjectBuilder(#[from] DrawObjectBuilderError),

    #[error(transparent)]
    Draw(#[from] DrawError),

    #[error(transparent)]
    InputCommunication(#[from] EventCommunicationError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum EventCommunicationError {
    #[error("did not receive a subscription id from the EventManager after subscribing")]
    DidNotReceiveIDResponse,
    #[error("failed to send a command message to the Event Manager. Message: {message}")]
    FailedToSendEventManagerCommandMessage { message: EventManagerCommand },
    #[error("expected a SubscriptionMessage of type: {expected_type}, got: {received_type}")]
    ReceiveUnexpectedResponse {
        expected_type: SubscriptionMessage,
        received_type: SubscriptionMessage,
    },
}

#[derive(Debug, Error)]
pub enum FileError {
    #[error("Failed reading path {path}")]
    FailedReadingPath {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Video {video_path} frame {frame_id} not found")]
    VideoFrameNotFound { video_path: String, frame_id: usize },
}

#[derive(Debug, Error)]
pub enum DrawError {
    #[error("Wrong drawable type: expected `{expected}`, found `{found}`")]
    WrongDrawableType {
        expected: &'static str,
        found: &'static str,
    },

    #[error("Screen with id {0} not found")]
    DisplayKeyNotFound(usize),

    #[error("Object {obj_id} not found on screen {screen_id}")]
    DrawableHandleNotFound { screen_id: usize, obj_id: usize },

    #[error("Sprite {0} not found")]
    SpriteNotFound(SpriteId),

    #[error("frame {1} of Sprite {0} not found")]
    SpriteFrameNotFound(SpriteId, u16),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    // Channel errors
    #[error("Failed to send reply on channel: {0}")]
    ChannelSendError(String),
}

#[derive(Debug, Error)]

pub enum DrawObjectBuilderError {
    #[error("No Drawable added to the object")]
    NoDrawableAdded(),

    #[error("No layer added to the object")]
    NoLayerAdded(),

    #[error("No screen ID added to the object")]
    NoScreenAdded(),

    #[error("No no lifetime added to the object")]
    NoLifetimeAdded(),

    #[error("Failed to build the sprite object")]
    FailedToBuildSpriteObject(),

    #[error("Failed to build the circle object")]
    FailedToBuildCircleObject(),

    #[error("Failed to build the line object")]
    FailedToBuildLineObject(),

    #[error("Failed to build the polygon object")]
    FailedToBuildPolygonObject(),

    #[error("Failed to build the rect object")]
    FailedToBuildRectObject(),

    #[error("Failed to build VideoStreamDrawable due to missing receiver")]
    FailedToBuildVideoStream(),
}

// convert to DrawError
impl<T> From<std::sync::mpsc::SendError<T>> for DrawError
where
    T: Debug,
{
    fn from(e: std::sync::mpsc::SendError<T>) -> Self {
        DrawError::ChannelSendError(format!("{:?}", e.0))
    }
}
