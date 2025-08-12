use thiserror::Error;

use crate::draw::SpriteId;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    File(#[from] FileError),

    #[error(transparent)]
    Draw(#[from] DrawError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
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
    #[error("Screen with id {0} not found")]
    DisplayKeyNotFound(usize),

    #[error("Object {obj_id} not found on screen {screen_id}")]
    DrawableHandleNotFound { screen_id: usize, obj_id: usize },

    #[error("Sprite {0} not found")]
    SpriteNotFound(SpriteId),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
