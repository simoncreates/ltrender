pub mod crossterm_buffer;
pub use crossterm_buffer::{CrosstermScreenBuffer, to_crossterm_color};
pub mod test_buffer;
pub use test_buffer::TestScreenBuffer;
