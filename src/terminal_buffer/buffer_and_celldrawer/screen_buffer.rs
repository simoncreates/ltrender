use crate::{
    BasicDraw, DrawError, DrawObject, ObjectId, SpriteRegistry, UpdateInterval,
    terminal_buffer::{CellDrawer, CharacterInfo, CharacterInfoList, ScreenBufferCore},
    update_interval_handler::UpdateIntervalCreator,
};
use common_stdx::Rect;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

use ascii_assets::Color;
use ascii_assets::TerminalChar;
use common_stdx::Point;

#[derive(Debug, Clone)]
pub enum CellDrawerCommand {
    SetString(BatchDrawInfo, (u16, u16)),
    Flush,
    Stop,
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

pub trait ScreenBuffer: ScreenBufferCore {
    type Drawer: CellDrawer + Send + 'static;
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

        let mut draws = drawable.draw(sprites)?.dump_draws();

        // TODO: make this only necessary, if a shader is applied, that requires the top left corner
        let top_left = if let Some(corner) = drawable.get_top_left() {
            corner
        } else {
            Point { x: 0, y: 0 }
        };
        let opt_c: Option<UpdateIntervalCreator> = drawable.bounding_iv(sprites);
        // TODO: is this right??
        let update_intervals: HashMap<u16, Vec<UpdateInterval>> =
            self.handle_none_interval_creator(opt_c, bounds.p1);

        let size = drawable.size(sprites)?;

        let mut touched: HashSet<usize> = HashSet::new();

        for unshifted_bd in draws.iter_mut() {
            // using the top left corner of the screen, to shift the drawables position on screen
            let mut rd = BasicDraw {
                pos: unshifted_bd.pos + bounds.p1,
                chr: unshifted_bd.chr,
            };

            for shader in &obj.shaders {
                shader.apply(
                    &mut rd,
                    (size.0 as usize, size.1 as usize),
                    top_left + bounds.p1,
                );
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
            let idx = self.idx_of_unchecked(rd.pos);
            self.cell_info_mut()[idx].info.insert(obj_id, ci);

            touched.insert(idx);
        }

        let (buf_cols, buf_rows) = self.size();
        for (&row_u16, ivs) in &update_intervals {
            if row_u16 >= buf_rows {
                continue;
            }
            let row = row_u16 as i32;
            for iv in ivs {
                let (start_x, end_x) = iv.interval;

                if end_x <= start_x {
                    continue;
                }

                let end_x = end_x.min(buf_cols as usize);

                for x in start_x..end_x {
                    let x_i32 = x as i32;
                    if x_i32 < 0 {
                        continue;
                    }
                    let pos = Point { x: x_i32, y: row };

                    // skip if out of total bounds
                    if pos.x as u16 >= buf_cols || pos.y as u16 >= buf_rows {
                        continue;
                    }

                    let idx = self.idx_of_unchecked(pos);

                    if !touched.contains(&idx) {
                        self.cell_info_mut()[idx].info.remove(&obj_id);
                    }
                }
            }
        }

        Ok(())
    }
    /// Remove an object from the buffer.
    fn remove_from_buffer(
        &mut self,
        obj: &DrawObject,
        obj_id: ObjectId,
        sprites: &SpriteRegistry,
        bounds: &Rect<i32>,
    ) {
        let drawable = &*obj.drawable;
        let opt_c = drawable.bounding_iv(sprites);

        let ivs: HashMap<u16, Vec<UpdateInterval>> =
            self.handle_none_interval_creator(opt_c, bounds.p1);
        let (buf_cols, buf_rows) = self.size();

        for (row, iv_list) in ivs.into_iter() {
            if row >= buf_rows {
                continue;
            }

            for iv in iv_list {
                let (start, end) = iv.interval;
                let clamped_end = end.min(buf_cols as usize);
                if start >= clamped_end {
                    continue;
                }

                for x in start..clamped_end {
                    let pos = Point {
                        x: x as i32,
                        y: row as i32,
                    };
                    let idx = self.idx_of_unchecked(pos);
                    self.cell_info_mut()[idx].info.remove(&obj_id);
                }
            }
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

            let slice = &cells[start..end];
            for (i, cell) in slice.iter().enumerate() {
                let idx = start + i;
                let y = (idx / cols as usize) as u16;
                let x = (idx % cols as usize) as u16;

                let chr_to_write = if let Some((_, char)) = Self::get_char_to_write(cell) {
                    char
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
                .send(CellDrawerCommand::SetString(batch, self.size()))
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

    fn idx_of_unchecked(&self, pos: Point<i32>) -> usize {
        (pos.y as usize) * self.size().0 as usize + pos.x as usize
    }

    // only returns some if the index is in the bounds of the buffer
    fn idx_of(&self, pos: Point<i32>) -> Option<usize> {
        let idx = self.idx_of_unchecked(pos);
        if idx < self.cell_info().len() {
            Some(idx)
        } else {
            None
        }
    }

    fn get_char_to_write(cell: &CharacterInfoList) -> Option<(ObjectId, TerminalChar)> {
        cell.info
            .iter()
            .max_by_key(|(_, c)| (c.screen_layer, c.layer))
            .map(|(obj_id, c)| (*obj_id, c.chr))
    }

    fn drop(&mut self);

    fn handle_none_interval_creator(
        &mut self,
        opt_c: Option<UpdateIntervalCreator>,
        shift_amount: Point<i32>,
    ) -> HashMap<u16, Vec<UpdateInterval>> {
        if let Some(mut c) = opt_c {
            c.shift_all_intervals_by(shift_amount);
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
