use std::{
    collections::HashMap,
    io::{BufWriter, Stdout, Write, stdout},
    sync::mpsc::{Receiver, SyncSender},
    thread::JoinHandle,
};

use crate::{
    DrawError, ScreenBuffer, UpdateIntervalHandler,
    terminal_buffer::{
        CellDrawer, CharacterInfoList, ScreenBufferCore,
        screen_buffer::{BatchDrawInfo, CellDrawerCommand},
    },
};
use ascii_assets;
use crossterm::style::Color;
use std::fmt::Write as FmtWrite;

pub fn create_celldrawer(
    size: usize,
    out: BufWriter<Stdout>,
) -> (SyncSender<CellDrawerCommand>, JoinHandle<()>) {
    use std::sync::mpsc;
    use std::thread;

    // create the channel
    let (tx, rx) = mpsc::sync_channel::<CellDrawerCommand>(size);

    // spawn a thread that owns the receiver and the writer
    let handle = thread::spawn(move || {
        let mut drawer = CrosstermCellDrawer { rx, out };

        // process commands until the sender(s) are dropped
        while let Ok(cmd) = drawer.rx.recv() {
            match cmd {
                CellDrawerCommand::SetString(batch) => drawer.set_string(batch),
                CellDrawerCommand::Flush => {
                    if let Err(e) = drawer.flush() {
                        log::error!("Flush error in drawer thread: {}", e);
                    }
                }
            }
        }

        // best-effort final flush
        let _ = drawer.flush();
    });

    (tx, handle)
}
#[derive(Debug)]
pub struct DefaultScreenBuffer {
    cells: Vec<CharacterInfoList>,
    intervals: UpdateIntervalHandler,
    size: (u16, u16),

    drawer_tx: std::sync::mpsc::SyncSender<CellDrawerCommand>,

    // todo: implement drop handler
    drawer_handle: std::thread::JoinHandle<()>,
}

impl ScreenBufferCore for DefaultScreenBuffer {
    fn cell_info_mut(&mut self) -> &mut Vec<CharacterInfoList> {
        &mut self.cells
    }

    fn intervals_mut(&mut self) -> &mut UpdateIntervalHandler {
        &mut self.intervals
    }

    fn size(&self) -> (u16, u16) {
        self.size
    }
}
impl ScreenBuffer for DefaultScreenBuffer {
    fn new(size: (u16, u16)) -> Self {
        let capacity = size.0 as usize * size.1 as usize;
        let out: BufWriter<Stdout> = BufWriter::new(stdout());
        let (drawer_tx, drawer_handle) = create_celldrawer(100, out);

        DefaultScreenBuffer {
            cells: vec![
                CharacterInfoList {
                    info: HashMap::new()
                };
                capacity
            ],
            intervals: UpdateIntervalHandler::new(size.0, size.1),
            size,
            drawer_tx,
            drawer_handle,
        }
    }
    fn drawer_sender(&self) -> std::sync::mpsc::SyncSender<CellDrawerCommand> {
        self.drawer_tx.clone()
    }
}

#[derive(Debug)]
pub struct CrosstermCellDrawer {
    rx: Receiver<CellDrawerCommand>,
    out: BufWriter<Stdout>,
}

impl CellDrawer for CrosstermCellDrawer {
    fn set_string(&mut self, batch: BatchDrawInfo) {
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
