use std::fmt::Debug;

use crate::{
    UpdateIntervalHandler, terminal_buffer::CharacterInfoList,
    update_interval_handler::UpdateIntervalCreator,
};

/// Trait that contains all internal state required by a screen buffer.
/// contains some utility functions
pub trait ScreenBufferCore: Sized + Debug {
    /// Return a mutable reference to the per‑cell info list
    fn cell_info_mut(&mut self) -> &mut Vec<CharacterInfoList>;

    /// Return reference to the per‑cell info list
    fn cell_info(&self) -> &Vec<CharacterInfoList>;

    /// Return a mutable reference to the interval handler
    fn intervals_mut(&mut self) -> &mut UpdateIntervalHandler;

    /// Current terminal size (width, height)
    fn size(&self) -> (u16, u16);

    // Helpers
    fn merge_creator(&mut self, creator: UpdateIntervalCreator) {
        self.intervals_mut().merge_creator(creator);
    }

    /// Mark the whole screen as dirty
    fn invalidate_entire_screen(&mut self) {
        self.intervals_mut().invalidate_entire_screen();
    }
}
