use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
};
use std::{
    collections::HashMap,
    io::{Write, stdout},
};

use crate::draw::{
    DrawError, DrawObject, DrawableHandle, ScreenBuffer, SpriteRegistry, UpdateIntervalHandler,
    terminal_buffer::{CharacterInfo, CharacterInfoList},
    update_interval_handler::UpdateIntervalType,
};

#[derive(Debug)]
struct SaveDrawInfo {
    draw_pos: Point<u16>,
    bounding_box: Rect<u16>,
    info: CharacterInfo,
    obj_id: DrawableHandle,
}

#[derive(Debug)]
pub struct CrosstermScreenBuffer {
    info_screen: Vec<CharacterInfoList>,
    screen: Vec<TerminalChar>,
    update_handler: UpdateIntervalHandler,
    size: (u16, u16),
}

impl CrosstermScreenBuffer {
    fn save_draw_to_buffer(&mut self, sd: SaveDrawInfo) {
        let SaveDrawInfo {
            draw_pos,
            bounding_box,
            info,
            obj_id,
        } = sd;
        let (cols, rows) = self.size;

        if draw_pos.x >= cols || draw_pos.y >= rows {
            return;
        }
        let p1 = bounding_box.p1;
        let p2 = bounding_box.p2;

        if draw_pos.x < p1.x || draw_pos.x >= p2.x || draw_pos.y < p1.y || draw_pos.y >= p2.y {
            return;
        }

        let idx = (draw_pos.y as usize) * cols as usize + draw_pos.x as usize;
        self.info_screen[idx].info.insert(obj_id, info);
    }
}

impl ScreenBuffer for CrosstermScreenBuffer {
    fn mark_all_dirty(&mut self, new_size: (u16, u16)) {
        self.update_handler.invalidate_entire_screen(new_size);
    }

    fn new(size: (u16, u16)) -> Self {
        let capacity = size.0 as usize * size.1 as usize;
        CrosstermScreenBuffer {
            info_screen: vec![
                CharacterInfoList {
                    info: HashMap::new()
                };
                capacity
            ],
            screen: vec![
                TerminalChar {
                    chr: ' ',
                    fg_color: None,
                    bg_color: None
                };
                capacity
            ],
            update_handler: UpdateIntervalHandler::new(),
            size,
        }
    }

    fn add_to_buffer(
        &mut self,
        obj: &DrawObject,
        obj_id: DrawableHandle,
        screen_layer: usize,
        screen_bounds: &Rect<u16>,
        sprites: &SpriteRegistry,
    ) -> Result<(), DrawError> {
        let update_map = obj.drawable.bounding_iv(sprites);
        self.update_handler.merge_redraw_regions(update_map);
        for rd in obj.drawable.draw(sprites)? {
            if !screen_bounds.contains(rd.pos) {
                continue;
            }

            let info = CharacterInfo {
                chr: rd.chr,
                layer: obj.layer,
                screen_layer,
                display_id: obj_id,
            };

            self.save_draw_to_buffer(SaveDrawInfo {
                draw_pos: rd.pos,
                bounding_box: *screen_bounds,
                info,
                obj_id,
            });
        }
        Ok(())
    }

    fn remove_from_buffer(
        &mut self,
        obj: &DrawObject,
        obj_id: DrawableHandle,
        screen_bounds: &Rect<u16>,
        sprites: &SpriteRegistry,
    ) {
        let update_map = obj.drawable.bounding_iv(sprites);
        self.update_handler.merge_redraw_regions(update_map.clone());

        let (cols, rows) = self.size;

        for (&y, ivs) in &update_map {
            if y < screen_bounds.p1.y || y >= screen_bounds.p2.y || y >= rows {
                continue;
            }
            let y_usize = y as usize;

            for iv in ivs {
                let (x_start, x_end) = iv.interval;

                let xs = x_start.max(screen_bounds.p1.x);
                let xe = x_end.min(screen_bounds.p2.x);

                if xs >= xe {
                    continue;
                }

                for x in xs..xe {
                    if x >= cols {
                        continue;
                    }
                    let idx = y_usize * (cols as usize) + (x as usize);
                    self.info_screen[idx].info.remove(&obj_id);
                }
            }
        }
    }

    fn update_terminal(&mut self) -> Result<(), DrawError> {
        self.update_handler.merge_intervals();
        let intervals_per_row = self.update_handler.dump_intervals();

        let cols = self.size.0 as usize;
        let max_x: u16 = self.size.0.saturating_sub(1);
        let max_y: u16 = self.size.1.saturating_sub(1);

        let mut stdout = stdout();
        for (y, iv_list) in intervals_per_row {
            if y > max_y {
                continue;
            }
            let y_usize = y as usize;

            for iv in iv_list {
                match iv.iv_type {
                    UpdateIntervalType::Forced => {
                        let x_start = iv.interval.0;
                        let x_end = iv.interval.1.min(max_x + 1);

                        if x_start > max_x {
                            continue;
                        }

                        for x in x_start..x_end {
                            let idx = y_usize * cols + x as usize;

                            if let Some(cell_info) = self.info_screen.get(idx) {
                                let top_ci_opt = cell_info
                                    .info
                                    .iter()
                                    .max_by_key(|(_, ci)| (ci.screen_layer, ci.layer))
                                    .map(|(_, ci)| ci);

                                let top_char: TerminalChar = match top_ci_opt {
                                    Some(ci) => ci.chr,
                                    None => TerminalChar {
                                        chr: ' ',
                                        fg_color: None,
                                        bg_color: None,
                                    },
                                };
                                self.screen[idx] = top_char;

                                queue!(
                                    stdout,
                                    MoveTo(x, y),
                                    SetForegroundColor(top_char.fg_color.unwrap_or(Color::White)),
                                    SetBackgroundColor(top_char.bg_color.unwrap_or(Color::Black)),
                                    Print(top_char.chr)
                                )?;
                            }
                        }
                    }
                    UpdateIntervalType::Optimized => {
                        let mut current_fg: Option<Color> = None;
                        let mut current_bg: Option<Color> = None;

                        let mut buffer = String::new();
                        let mut batch_start_x = 0usize;
                        let mut batching = false;

                        for x in iv.interval.0 as usize..iv.interval.1.min(max_x + 1) as usize {
                            let idx = y_usize * cols + x;

                            if let Some(cell_info) = self.info_screen.get(idx) {
                                let top_ci_opt = cell_info
                                    .info
                                    .iter()
                                    .max_by_key(|(_, ci)| (ci.screen_layer, ci.layer))
                                    .map(|(_, ci)| ci);

                                let top_char: TerminalChar = match top_ci_opt {
                                    Some(ci) => ci.chr,
                                    None => TerminalChar {
                                        chr: ' ',
                                        fg_color: None,
                                        bg_color: None,
                                    },
                                };

                                self.screen[idx] = top_char;

                                let needs_flush = !batching
                                    || top_char.fg_color != current_fg
                                    || top_char.bg_color != current_bg;

                                if !needs_flush {
                                    current_fg = top_char.fg_color;
                                    current_bg = top_char.bg_color;
                                    batch_start_x = x;
                                    batching = true;
                                } else if !buffer.is_empty() {
                                    let start_x = batch_start_x.min(cols - 1) as u16;
                                    queue!(
                                        stdout,
                                        MoveTo(start_x, y),
                                        SetForegroundColor(current_fg.unwrap_or(Color::White)),
                                        SetBackgroundColor(current_bg.unwrap_or(Color::Black)),
                                        Print(&buffer)
                                    )?;
                                    buffer.clear();
                                }

                                buffer.push(top_char.chr);
                            }
                        } // for each column in the interval

                        if !buffer.is_empty() {
                            let start_x = batch_start_x.min(cols - 1) as u16;
                            queue!(
                                stdout,
                                MoveTo(start_x, y),
                                SetForegroundColor(current_fg.unwrap_or(Color::White)),
                                SetBackgroundColor(current_bg.unwrap_or(Color::Black)),
                                Print(&buffer)
                            )?;
                        }
                    } // Optimized
                } // match iv_type
            } // for each interval in the row
        } // for each row

        stdout.flush()?;
        Ok(())
    }
}
