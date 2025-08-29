use std::{
    collections::HashMap,
    io::{BufWriter, Stdout, Write, stdout},
    sync::mpsc::Receiver,
};

use crate::{
    DrawError, ScreenBuffer, ScreenBufferNew, UpdateIntervalHandler, define_screen_buffer,
    terminal_buffer::{
        CellDrawer, CharacterInfoList, ScreenBufferCore,
        buffer_and_celldrawer::{BatchDrawInfo, screen_buffer::CellDrawerCommand},
    },
};
use ascii_assets;
use crossterm::style::Color;
use std::fmt::Write as FmtWrite;

define_screen_buffer!(CrosstermScreenBuffer, CrosstermCellDrawer);
#[derive(Debug)]
pub struct CrosstermCellDrawer {
    rx: Receiver<CellDrawerCommand>,
    out: BufWriter<Stdout>,
}

impl CellDrawer for CrosstermCellDrawer {
    fn init(rx: Receiver<CellDrawerCommand>) -> Self {
        CrosstermCellDrawer {
            out: BufWriter::new(stdout()),
            rx,
        }
    }

    fn set_string(&mut self, batch: BatchDrawInfo, _size: (u16, u16)) {
        let text_len: usize = batch.segments.iter().map(|s| s.text.len()).sum();
        let mut output = String::with_capacity(text_len + 256);

        let _ = write!(output, "\x1b[{};{}H", batch.y + 1, batch.start_x + 1);

        let mut current_fg: Option<Color> = None;
        let mut current_bg: Option<Color> = None;

        for seg in &batch.segments {
            let desired_fg = to_crossterm_color(seg.fg_color);
            let desired_bg = to_crossterm_color(seg.bg_color);

            if current_fg.as_ref() != Some(&desired_fg) {
                match desired_fg {
                    Color::Reset => output.push_str("\x1b[39m"),
                    Color::Rgb { r, g, b } => {
                        let _ = write!(output, "\x1b[38;2;{};{};{}m", r, g, b);
                    }
                    _ => {}
                }
                current_fg = Some(desired_fg);
            }

            if current_bg.as_ref() != Some(&desired_bg) {
                match desired_bg {
                    Color::Reset => output.push_str("\x1b[49m"),
                    Color::Rgb { r, g, b } => {
                        let _ = write!(output, "\x1b[48;2;{};{};{}m", r, g, b);
                    }
                    _ => {}
                }
                current_bg = Some(desired_bg);
            }

            output.push_str(&seg.text);
        }

        output.push_str("\x1b[0m");

        if let Err(e) = self.out.write_all(output.as_bytes()) {
            log::error!("Failed to write to terminal BufWriter: {}", e);
        }
    }

    fn flush(&mut self) -> Result<(), DrawError> {
        self.out.flush()?;
        Ok(())
    }
}

pub fn to_crossterm_color(colour: Option<ascii_assets::Color>) -> Color {
    if let Some(colour) = colour {
        if colour.reset {
            Color::Reset
        } else {
            let (r, g, b) = colour.rgb;
            Color::Rgb { r, g, b }
        }
    } else {
        Color::Reset
    }
}
