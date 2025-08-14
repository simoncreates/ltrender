use std::{collections::HashMap, fmt::Debug};

use crate::draw::{
    DrawError, DrawObject, DrawableId, SpriteRegistry, UpdateInterval, UpdateIntervalHandler,
    terminal_buffer::{CharacterInfo, CharacterInfoList},
};
use common_stdx::Rect;

use ascii_assets::TerminalChar;
use common_stdx::Point;
use crossterm::style::Color;

/// Trait that describes how to write a single character and flush the
/// underlying output device.
pub trait CellDrawer: Sized + Debug {
    /// Write a string at an absolute position.
    /// also used for drawing single character
    fn set_string(
        &mut self,
        pos: Point<u16>,
        s: &str,
        fg_color: Option<Color>,
        bg_color: Option<Color>,
    );

    /// Flush any buffered output to the terminal (or other destination).
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
    fn merge_intervals(&mut self, map: HashMap<u16, Vec<UpdateInterval>>) {
        self.intervals_mut().merge_redraw_regions(map);
    }

    /// Mark the whole screen as dirty
    fn invalidate_entire_screen(&mut self) {
        let size = self.size();
        self.intervals_mut().invalidate_entire_screen(size);
    }
}

pub struct BatchDrawInfo {
    start_x: u16,
    fg_color: Option<Color>,
    bg_color: Option<Color>,
    buffer: String,
}

/// public API that combines the core bookkeeping with the drawing layer
pub trait ScreenBuffer: ScreenBufferCore + CellDrawer {
    fn new(size: (u16, u16)) -> Self
    where
        Self: Sized;

    /// Add an object to the buffer.
    fn add_to_buffer(
        &mut self,
        obj: &DrawObject,
        obj_id: DrawableId,
        screen_layer: usize,
        bounds: &Rect<u16>,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError> {
        let drawable = &*obj.drawable;
        let map = drawable.bounding_iv(sprites);
        self.merge_intervals(map);

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
        self.merge_intervals(map.clone());

        for ci in self.cell_info_mut() {
            ci.info.remove(&obj_id);
        }
    }
    fn update_terminal(&mut self) -> Result<(), DrawError> {
        self.intervals_mut().merge_intervals();
        let intervals_per_row = self.intervals_mut().dump_intervals();
        let (cols, rows) = self.size();

        for (y, ivs) in intervals_per_row {
            if y >= rows {
                continue;
            }

            for _iv in ivs {
                let mut writes: Vec<BatchDrawInfo> = Vec::new();

                {
                    let cells = self.cell_info_mut();

                    let mut current_x: Option<u16> = None;
                    let mut current_colors: Option<(Option<Color>, Option<Color>)> = None;
                    let mut current_buf = String::new();

                    for x in 0..cols {
                        let idx = (y as usize) * cols as usize + x as usize;

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

                        if current_x.is_none() {
                            // new batch
                            current_x = Some(x);
                            current_colors = Some(colors);
                            current_buf.push(chr_to_write.chr);
                        } else if Some(colors) == current_colors {
                            // same color, append
                            current_buf.push(chr_to_write.chr);
                        } else {
                            // color changed, store previous
                            writes.push(BatchDrawInfo {
                                start_x: current_x.unwrap(),
                                buffer: current_buf.clone(),
                                fg_color: current_colors.unwrap().0,
                                bg_color: current_colors.unwrap().1,
                            });

                            // start new
                            current_x = Some(x);
                            current_colors = Some(colors);
                            current_buf.clear();
                            current_buf.push(chr_to_write.chr);
                        }
                    }

                    // flush the last batch if any
                    if let (Some(start_x), Some(colors)) = (current_x, current_colors) {
                        if !current_buf.is_empty() {
                            writes.push(BatchDrawInfo {
                                start_x,
                                buffer: current_buf.clone(),
                                fg_color: colors.0,
                                bg_color: colors.1,
                            });
                        }
                    }
                }

                // send batched writes
                for BatchDrawInfo {
                    start_x,
                    buffer,
                    fg_color,
                    bg_color,
                } in writes
                {
                    self.set_string(Point { x: start_x, y }, &buffer, fg_color, bg_color);
                }
            }
        }

        self.flush()
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

    fn idx_of(&self, pos: Point<u16>) -> usize {
        (pos.y as usize) * self.size().0 as usize + pos.x as usize
    }
}
