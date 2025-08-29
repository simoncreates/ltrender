use std::collections::HashMap;
use std::{mem, sync::mpsc::Receiver};

use crate::set_test_data;
use crate::{
    DrawError, ScreenBuffer, ScreenBufferNew, UpdateIntervalHandler, define_screen_buffer,
    terminal_buffer::{
        CellDrawer, CharacterInfoList, ScreenBufferCore,
        buffer_and_celldrawer::{BatchDrawInfo, screen_buffer::CellDrawerCommand},
    },
};
use ascii_assets::{self, Color, TerminalChar};
use log::info;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TerminalContentInformation {
    pub size: (u16, u16),
    pub content: Vec<Option<TerminalChar>>,
    pub amount_of_changed_chars: usize,
    pub amount_of_draw_commands: usize,
}
impl TerminalContentInformation {
    pub fn resize_content(&mut self, size: (u16, u16), default_char: Option<TerminalChar>) {
        self.size = size;

        let desired_len = (size.0 as usize) * (size.1 as usize);

        if self.content.len() < desired_len {
            self.content.resize(desired_len, default_char);
        } else if self.content.len() > desired_len {
            self.content.truncate(desired_len);
        }
    }
}

define_screen_buffer!(TestScreenBuffer, TestCellDrawer);
#[derive(Debug)]
pub struct TestCellDrawer {
    rx: Receiver<CellDrawerCommand>,
    temp_screen: TerminalContentInformation,
}

impl CellDrawer for TestCellDrawer {
    fn init(rx: Receiver<CellDrawerCommand>) -> Self {
        TestCellDrawer {
            rx,
            temp_screen: TerminalContentInformation {
                size: (0, 0),
                content: Vec::new(),
                amount_of_changed_chars: 0,
                amount_of_draw_commands: 0,
            },
        }
    }

    fn set_string(&mut self, batch: BatchDrawInfo, size: (u16, u16)) {
        let mut current_pos =
            (size.0 * batch.y + batch.start_x).clamp(usize::MIN as u16, usize::MAX as u16) as usize;

        let mut current_fg: Option<Color> = None;
        let mut current_bg: Option<Color> = None;

        // prefilling content of the temp_screen, to avoid writing out of bounds
        self.temp_screen.resize_content(size, None);

        for seg in &batch.segments {
            if let Some(fg) = seg.fg_color {
                current_fg = Some(fg)
            }
            if let Some(bg) = seg.bg_color {
                current_bg = Some(bg)
            }
            self.temp_screen.amount_of_draw_commands += 1;
            info!(
                "setting string: {}, with colours: {:?}, {:?} at position {}",
                seg.text, seg.fg_color, seg.bg_color, current_pos
            );

            for chr in seg.text.chars() {
                let opt_char = if chr == ' ' && current_fg.is_none() && current_bg.is_none() {
                    None
                } else {
                    Some(TerminalChar {
                        chr,
                        fg_color: current_fg,
                        bg_color: current_bg,
                    })
                };

                self.temp_screen.content[current_pos] = opt_char;

                self.temp_screen.amount_of_changed_chars += 1;

                current_pos += 1
            }
        }
    }

    fn flush(&mut self) -> Result<(), DrawError> {
        set_test_data(self.temp_screen.clone());
        Ok(())
    }
}
