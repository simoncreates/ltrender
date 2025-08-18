use crate::draw::{
    DrawError, DrawObject, DrawableId, SpriteRegistry, UpdateInterval, UpdateIntervalHandler,
    terminal_buffer::{CharacterInfo, CharacterInfoList},
};
use common_stdx::Rect;

use std::{collections::HashMap, fmt::Debug, sync::mpsc::Sender};

use ascii_assets::Color;
use ascii_assets::TerminalChar;
use common_stdx::Point;

/// Trait that describes how to write a string of chars
pub trait CellDrawer: Debug {
    /// Write a string at an absolute position.
    /// also used for drawing single character
    fn set_string(&mut self, batch: BatchDrawInfo);

    /// Flush any buffered output to the terminal, or any other output that you might prefer
    fn flush(&mut self) -> Result<(), DrawError>;
}

/// Trait that contains all internal state required by a screen buffer.
pub trait ScreenBufferCore: Sized + Debug {
    /// Return a mutable reference to the per‑cell info list.
    fn cell_info_mut(&mut self) -> &mut Vec<CharacterInfoList>;

    /// Return a mutable reference to the interval handler.
    fn intervals_mut(&mut self) -> &mut UpdateIntervalHandler;

    /// Current terminal size (width, height).
    fn size(&self) -> (u16, u16);

    // helpers
    /// Merge a map of  intervals into the existing interval set.
    fn merge_redraw_regions(&mut self, map: HashMap<u16, Vec<UpdateInterval>>) {
        self.intervals_mut().merge_redraw_regions(map);
    }

    /// Mark the whole screen as dirty
    fn invalidate_entire_screen(&mut self) {
        self.intervals_mut().invalidate_entire_screen();
    }
}

#[derive(Debug, Clone)]
pub struct BatchSegment {
    pub text: String,
    pub fg_color: Option<Color>,
    pub bg_color: Option<Color>,
}

#[derive(Debug, Clone)]
pub struct BatchDrawInfo {
    pub start_x: u16,
    pub y: u16,
    pub segments: Vec<BatchSegment>,
}

/// public API that combines the core bookkeeping with the drawing layer
pub trait ScreenBuffer: ScreenBufferCore {
    fn new(size: (u16, u16)) -> Self
    where
        Self: Sized;

    /// Add an object to the buffer.
    fn add_to_buffer(
        &mut self,
        obj: &mut DrawObject,
        obj_id: DrawableId,
        screen_layer: usize,
        bounds: &Rect<u16>,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError> {
        let drawable = &mut obj.drawable;
        let map = drawable.bounding_iv(sprites);
        self.merge_redraw_regions(map);

        for rd in drawable.draw(sprites)? {
            if !bounds.contains(rd.pos) {
                continue;
            }

            let (buf_cols, buf_rows) = self.size();
            if rd.pos.x >= buf_cols || rd.pos.y >= buf_rows {
                continue;
            }

            let ci = CharacterInfo {
                chr: rd.chr,
                layer: obj.layer,
                screen_layer,
                display_id: obj_id,
            };
            let idx = self.idx_of(rd.pos);
            self.cell_info_mut()[idx].info.insert(obj_id, ci);
        }

        Ok(())
    }

    /// Remove an object from the buffer.
    fn remove_from_buffer(
        &mut self,
        obj: &DrawObject,
        obj_id: DrawableId,
        sprites: &SpriteRegistry,
    ) {
        let drawable = &*obj.drawable;
        let map = drawable.bounding_iv(sprites);
        self.merge_redraw_regions(map.clone());

        for ci in self.cell_info_mut() {
            ci.info.remove(&obj_id);
        }
    }
    fn update_terminal(&mut self, expand: usize) -> Result<(), DrawError> {
        self.intervals_mut().expand_regions(expand);
        self.intervals_mut().merge_intervals();
        let intervals = self.intervals_mut().dump_intervals();
        let (cols, rows) = self.size();

        for iv in intervals {
            let (start, end) = iv.interval;
            let max_idx = cols as usize * rows as usize;
            let start = start.min(max_idx);
            let end = end.min(max_idx);

            let cells = self.cell_info_mut();
            let mut batch = BatchDrawInfo {
                start_x: 0,
                y: 0,
                segments: Vec::new(),
            };
            let mut current_segment: Option<BatchSegment> = None;

            for idx in start..end {
                let y = (idx / cols as usize) as u16;
                let x = (idx % cols as usize) as u16;

                let chr_to_write = if let Some(ci_opt) = cells[idx]
                    .info
                    .values()
                    .max_by_key(|c| (c.screen_layer, c.layer))
                {
                    ci_opt.chr
                } else {
                    TerminalChar {
                        chr: ' ',
                        fg_color: None,
                        bg_color: None,
                    }
                };

                let colors = (chr_to_write.fg_color, chr_to_write.bg_color);

                if current_segment.is_none() {
                    current_segment = Some(BatchSegment {
                        text: chr_to_write.chr.to_string(),
                        fg_color: colors.0,
                        bg_color: colors.1,
                    });
                    batch.start_x = x;
                    batch.y = y;
                } else if let Some(seg) = &mut current_segment {
                    if seg.fg_color == colors.0 && seg.bg_color == colors.1 {
                        seg.text.push(chr_to_write.chr);
                    } else {
                        batch.segments.push(seg.clone());
                        *seg = BatchSegment {
                            text: chr_to_write.chr.to_string(),
                            fg_color: colors.0,
                            bg_color: colors.1,
                        };
                    }
                }
            }

            if let Some(seg) = current_segment {
                batch.segments.push(seg);
            }

            if let Err(e) = self
                .drawer_sender()
                .send(CellDrawerCommand::SetString(batch))
            {
                log::error!("Failed to send SetString to drawer thread: {}", e);
            }
        }

        if let Err(e) = self.drawer_sender().send(CellDrawerCommand::Flush) {
            log::error!("Failed to send Flush to drawer thread: {}", e);
        }
        Ok(())
    }

    fn mark_all_dirty(&mut self, new_size: (u16, u16)) {
        self.invalidate_entire_screen();
        let capacity = new_size.0 as usize * new_size.1 as usize;
        if self.cell_info_mut().len() != capacity {
            self.cell_info_mut()
                .resize_with(capacity, || CharacterInfoList {
                    info: HashMap::new(),
                });
        }
    }
    fn drawer_sender(&self) -> std::sync::mpsc::SyncSender<CellDrawerCommand>;

    fn idx_of(&self, pos: Point<u16>) -> usize {
        (pos.y as usize) * self.size().0 as usize + pos.x as usize
    }
}

#[derive(Debug)]
pub enum CellDrawerCommand {
    SetString(BatchDrawInfo),
    Flush,
}

#[macro_export]
macro_rules! spawn_cell_drawer {
    ($name:ident, $out:expr, $buffersize:expr) => {{
        use std::sync::mpsc;
        use std::sync::mpsc::{Receiver, Sender};
        use std::thread;

        // create the channel
        let (tx, rx) = mpsc::sync_channel::<CellDrawerCommand>($buffersize);

        // evaluate the expression here so it's moved into the thread below
        let out = $out;

        // spawn a thread that owns the receiver and the writer
        let handle = thread::spawn(move || {
            let mut drawer = $name { rx, out };

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
    }};
}
/// ## $name:
/// The name of the default drawbuffer
/// ## $drawer:
/// The name of the cell drawer
/// ## $buffersize:
/// the amount of unhandled draw commands, that can be buffered before blocking
///
///
///
/// ### info:
/// When using this macro, you are required to create a struct
/// that implements the `CellDrawer` trait and at minimum looks like this:
/// ```
///#[derive(Debug)]
/// pub struct $drawer {
///     rx: Receiver<CellDrawerCommand>,
///     out: BufWriter<Stdout>,
/// }
/// ```
#[macro_export]
macro_rules! create_drawbuffer {
    ($name:ident, $drawer:ident, $buffersize:expr) => {
        use crate::spawn_cell_drawer;
        #[derive(Debug)]
        pub struct $name {
            cells: Vec<CharacterInfoList>,
            intervals: UpdateIntervalHandler,
            size: (u16, u16),

            drawer_tx: std::sync::mpsc::SyncSender<CellDrawerCommand>,

            // todo: implement drop handler
            drawer_handle: std::thread::JoinHandle<()>,
        }

        impl ScreenBufferCore for $name {
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
        impl ScreenBuffer for $name {
            fn new(size: (u16, u16)) -> Self {
                let capacity = size.0 as usize * size.1 as usize;
                let out = BufWriter::new(stdout());
                let (drawer_tx, drawer_handle) = spawn_cell_drawer!($drawer, out, $buffersize);

                $name {
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
    };
}
