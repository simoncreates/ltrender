use crate::draw::{
    AllSprites, CharacterInfo, CharacterInfoList, DrawError, DrawObject, ObjId,
    UpdateIntervalHandler,
};
pub mod drawable;
pub use drawable::Drawable;
pub mod standard_drawables;
pub use standard_drawables::SpriteDrawable;

use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};
use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use log::info;
use std::{
    collections::HashMap,
    io::{Write, stdout},
};

#[derive(Debug)]
pub struct SaveDrawInfo {
    draw_pos: Point<u16>,
    bounding_box: Rect<u16>,
    info: CharacterInfo,
    obj_id: ObjId,
}

#[derive(Debug)]
pub struct ScreenBuffer {
    info_screen: Vec<CharacterInfoList>,
    screen: Vec<TerminalChar>,
    update_handler: UpdateIntervalHandler,
    size: (u16, u16),
}

impl ScreenBuffer {
    pub fn new(size: (u16, u16)) -> Self {
        ScreenBuffer {
            info_screen: vec![
                CharacterInfoList {
                    info: HashMap::new()
                };
                size.0 as usize * size.1 as usize
            ],
            screen: vec![
                TerminalChar {
                    chr: ' ',
                    fg_color: None,
                    bg_color: None
                };
                size.0 as usize * size.1 as usize
            ],
            update_handler: UpdateIntervalHandler::new(),
            size,
        }
    }

    pub fn add_to_buffer(
        &mut self,
        obj: &DrawObject,
        obj_id: ObjId,
        screen_layer: usize,
        screen_bounds: &Rect<u16>,
        sprites: &AllSprites,
    ) -> Result<(), DrawError> {
        let update_map = obj.drawable.bounding_iv(sprites);
        self.update_handler.add_update_hash(update_map);

        let d = &obj.drawable;
        let raw = d.draw(sprites)?;
        for rd in raw {
            if !screen_bounds.contains(rd.pos) {
                continue;
            }

            let info = CharacterInfo {
                chr: rd.chr,
                layer: obj.layer,
                screen_layer: screen_layer,
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

    pub fn remove_from_buffer(
        &mut self,
        obj: &DrawObject,
        obj_id: ObjId,
        screen_bounds: &Rect<u16>,
        sprites: &AllSprites,
    ) {
        let drawable = &obj.drawable;

        let update_map = drawable.bounding_iv(sprites);
        self.update_handler.add_update_hash(update_map.clone());

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

    pub fn save_draw_to_buffer(&mut self, sd: SaveDrawInfo) {
        let SaveDrawInfo {
            draw_pos,
            bounding_box,
            info,
            obj_id,
        } = sd;

        info!("save draw info: {:#?}", &sd);

        let p1 = bounding_box.p1;
        let p2 = bounding_box.p2;
        let (cols, rows) = self.size;

        let x = draw_pos.x;
        let y = draw_pos.y;

        // terminal bounds
        if x >= cols || y >= rows {
            return;
        }

        // screen bounds
        if x < p1.x || x >= p2.x || y < p1.y || y >= p2.y {
            return;
        }

        // compute idx
        let idx = (y as usize) * (cols as usize) + (x as usize);

        // insert info
        self.info_screen[idx].info.insert(obj_id, info);
    }

    pub fn update_terminal(&mut self) -> Result<(), DrawError> {
        self.update_handler.merge_intervals();
        let intervals_per_row = self.update_handler.dump_intervals();

        let cols = self.size.0 as usize;
        let mut stdout = stdout();

        for (y, iv_list) in intervals_per_row {
            let y_usize = y as usize;

            for iv in iv_list {
                let (x_start, x_end) = iv.interval;

                let mut current_fg = None;
                let mut current_bg = None;
                let mut buffer = String::new();
                let mut batching = false;
                let mut batch_start_x = 0;

                for x in x_start as usize..x_end as usize {
                    let idx = y_usize * cols + x;

                    let cell_info = &self.info_screen[idx];
                    let option_top_ci = cell_info
                        .info
                        .iter()
                        .max_by_key(|(_, ci)| (ci.screen_layer, ci.layer));

                    let top_ci = match option_top_ci {
                        Some((_, info)) => &info.chr,
                        None => &TerminalChar {
                            chr: ' ',
                            fg_color: None,
                            bg_color: None,
                        },
                    };

                    if *top_ci != self.screen[idx] {
                        self.screen[idx] = *top_ci;

                        if !batching
                            || top_ci.fg_color != current_fg
                            || top_ci.bg_color != current_bg
                            || x != batch_start_x + buffer.chars().count()
                        {
                            if !buffer.is_empty() {
                                queue!(
                                    stdout,
                                    MoveTo(batch_start_x as u16, y_usize as u16),
                                    SetForegroundColor(current_fg.unwrap_or(Color::White)),
                                    SetBackgroundColor(current_bg.unwrap_or(Color::Black)),
                                    Print(&buffer),
                                    ResetColor
                                )?;
                                buffer.clear();
                            }

                            current_fg = top_ci.fg_color;
                            current_bg = top_ci.bg_color;
                            batch_start_x = x;
                            batching = true;
                        }

                        buffer.push(top_ci.chr);
                    } else {
                        info!("skipping unchanged cell ({x},{y_usize})");
                    }
                }

                if !buffer.is_empty() {
                    queue!(
                        stdout,
                        MoveTo(batch_start_x as u16, y_usize as u16),
                        SetForegroundColor(current_fg.unwrap_or(Color::White)),
                        SetBackgroundColor(current_bg.unwrap_or(Color::Black)),
                        Print(&buffer),
                        ResetColor
                    )?;
                }
            }
        }
        stdout.flush()?;
        Ok(())
    }
}
