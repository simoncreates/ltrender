use crate::draw::{
    DrawError, DrawObject, ObjectId, SpriteRegistry, UpdateInterval, UpdateIntervalHandler,
    terminal_buffer::{CharacterInfo, CharacterInfoList},
    update_interval_handler::UpdateIntervalCreator,
};
use common_stdx::Rect;

use std::{collections::HashMap, fmt::Debug};

use ascii_assets::Color;
use ascii_assets::TerminalChar;
use common_stdx::Point;

pub mod default;
pub mod shaders;
pub use shaders::Shader;

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

    fn merge_creator(&mut self, creator: UpdateIntervalCreator) {
        self.intervals_mut().merge_creator(creator);
    }

    /// Mark the whole screen as dirty
    fn invalidate_entire_screen(&mut self) {
        self.intervals_mut().invalidate_entire_screen();
    }
}

#[derive(Debug)]
pub enum CellDrawerCommand {
    SetString(BatchDrawInfo),
    Flush,
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
        obj_id: ObjectId,
        screen_layer: usize,
        bounds: &Rect<i32>,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError> {
        let drawable = &mut obj.drawable;

        let mut draws = drawable.draw(sprites)?;

        let top_left = if let Some(corner) = drawable.get_top_left() {
            corner
        } else {
            Point { x: 0, y: 0 }
        };

        let opt_c = drawable.bounding_iv(sprites);
        _ = self.handle_none_interval_creator(opt_c);
        let size = drawable.size(sprites)?;
        for rd in draws.iter_mut() {
            for shader in &obj.shaders {
                shader.apply(rd, (size.0 as usize, size.1 as usize), top_left);
            }
            if !bounds.contains(rd.pos) {
                continue;
            }

            if rd.pos.x < 0 || rd.pos.y < 0 {
                continue;
            }
            let (buf_cols, buf_rows) = self.size();
            if rd.pos.x as u16 >= buf_cols || rd.pos.y as u16 >= buf_rows {
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
    fn remove_from_buffer(&mut self, obj: &DrawObject, obj_id: ObjectId, sprites: &SpriteRegistry) {
        let drawable = &*obj.drawable;
        let opt_c = drawable.bounding_iv(sprites);

        let ivs: HashMap<u16, UpdateInterval> = self.handle_none_interval_creator(opt_c);
        let (buf_cols, buf_rows) = self.size();

        for (row, interval) in ivs.into_iter() {
            if row >= buf_rows {
                continue;
            }

            let (start, end) = interval.interval;
            let clamped_end = end.min(buf_cols as usize);
            if start >= clamped_end {
                continue;
            }

            for x in start..clamped_end {
                let pos = Point {
                    x: x as i32,
                    y: row as i32,
                };
                let idx = self.idx_of(pos);
                self.cell_info_mut()[idx].info.remove(&obj_id);
            }
        }
    }

    fn update_terminal(&mut self, expand: usize) -> Result<(), DrawError> {
        self.intervals_mut().invalidate_entire_screen();
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

    fn idx_of(&self, pos: Point<i32>) -> usize {
        (pos.y as usize) * self.size().0 as usize + pos.x as usize
    }

    fn handle_none_interval_creator(
        &mut self,
        opt_c: Option<UpdateIntervalCreator>,
    ) -> HashMap<u16, UpdateInterval> {
        if let Some(mut c) = opt_c {
            self.merge_creator(c.clone());
            c.dump_intervals()
        } else {
            let mut c = UpdateIntervalCreator::new();
            c.register_redraw_region(Rect {
                p1: Point { x: 0, y: 0 },
                p2: Point {
                    x: self.size().0 as i32,
                    y: self.size().1 as i32,
                },
            });
            self.merge_creator(c.clone());
            c.dump_intervals()
        }
    }
}
