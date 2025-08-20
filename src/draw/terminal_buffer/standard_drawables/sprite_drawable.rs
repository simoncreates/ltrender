use std::collections::HashMap;

use common_stdx::{Point, Rect};

use crate::draw::{
    DrawError, SpriteEntry, SpriteId, SpriteRegistry, UpdateInterval,
    terminal_buffer::{
        Drawable,
        drawable::{BasicDraw, SinglePointed, convert_rect_to_update_intervals},
    },
};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VideoLoopType {
    Loop,
    NoLoop,
    Boomerang,
    KillOnFinish,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VideoSpeed {
    Fps(u16),
    MillisecondsPerFrame(u16),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FrameIdent {
    FirstFrame,
    SpecificFrame(u16),
    LastFrame,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AnimationInfo {
    Image {
        frame: FrameIdent,
    },
    Video {
        loop_type: VideoLoopType,
        speed: VideoSpeed,
        start_frame: FrameIdent,
        end_frame: FrameIdent,
    },
}
impl Default for AnimationInfo {
    fn default() -> Self {
        AnimationInfo::Video {
            loop_type: VideoLoopType::Loop,
            speed: VideoSpeed::Fps(1),
            start_frame: FrameIdent::FirstFrame,
            end_frame: FrameIdent::LastFrame,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SpriteDrawable {
    pub position: Point<i32>,
    pub sprite_id: SpriteId,
    pub last_state_change: std::time::Instant,
    pub animation_type: AnimationInfo,
}

impl SinglePointed for SpriteDrawable {
    fn position(&self) -> Point<i32> {
        self.position
    }
    fn set_position(&mut self, p: Point<i32>) {
        self.position = p;
    }
}

impl SpriteDrawable {
    fn get_frame_idx(&self, ident: &FrameIdent, sprite: &SpriteEntry) -> usize {
        match ident {
            FrameIdent::FirstFrame => 0,
            FrameIdent::SpecificFrame(idx) => *idx as usize,
            FrameIdent::LastFrame => sprite.info.size().0 - 1,
        }
    }
    fn draw_frame(
        &self,
        sprite: &SpriteEntry,
        frame_idx: usize,
        size: (usize, usize),
    ) -> Result<Vec<BasicDraw>, DrawError> {
        let frame_data =
            sprite
                .info
                .get_frame_flat(frame_idx)
                .ok_or(DrawError::SpriteFrameNotFound(
                    self.sprite_id,
                    frame_idx as u16,
                ))?;

        let mut out = Vec::with_capacity(size.0 * size.1);
        let origin_x = self.position.x;
        let origin_y = self.position.y;

        for (i, ch) in frame_data.iter().enumerate() {
            let dx = (i % size.0) as i32;
            let dy = (i / size.0) as i32;

            let abs_x = origin_x.saturating_add(dx);
            let abs_y = origin_y.saturating_add(dy);

            out.push(BasicDraw {
                pos: Point { x: abs_x, y: abs_y },
                chr: *ch,
            });
        }
        Ok(out)
    }

    fn get_frames_since_last_update(&self, speed: &VideoSpeed) -> u16 {
        let elapsed_ms = self.last_state_change.elapsed().as_millis();
        match speed {
            VideoSpeed::Fps(fps) => ((elapsed_ms * (*fps as u128)) / 1000) as u16,
            VideoSpeed::MillisecondsPerFrame(ms) => (elapsed_ms / (*ms as u128)) as u16,
        }
    }
}

impl Drawable for SpriteDrawable {
    fn size(&self, sprites: &SpriteRegistry) -> Result<(u16, u16), DrawError> {
        let sprite = sprites
            .get(&self.sprite_id)
            .ok_or(DrawError::SpriteNotFound(self.sprite_id))?;

        let size = sprite.info.size();
        Ok((size.1 as u16, size.2 as u16))
    }
    fn as_single_pointed_mut(&mut self) -> Option<&mut dyn SinglePointed> {
        Some(self)
    }
    fn draw(&mut self, sprites: &SpriteRegistry) -> Result<Vec<BasicDraw>, DrawError> {
        let sprite = sprites
            .get(&self.sprite_id)
            .ok_or(DrawError::SpriteNotFound(self.sprite_id))?;

        let size = sprite.info.size();
        let mut out = Vec::with_capacity(size.1 * size.2);
        match &self.animation_type {
            AnimationInfo::Image { frame } => {
                let frame_idx = self.get_frame_idx(frame, sprite);

                out = self.draw_frame(sprite, frame_idx, (size.1, size.2))?;
            }
            AnimationInfo::Video {
                loop_type,
                speed,
                start_frame,
                end_frame,
            } => {
                let sprite = sprites
                    .get(&self.sprite_id)
                    .ok_or(DrawError::SpriteNotFound(self.sprite_id))?;

                let real_start_frame = self.get_frame_idx(start_frame, sprite);
                let real_end_frame = self.get_frame_idx(end_frame, sprite);

                let amount_of_frames_since_start = self.get_frames_since_last_update(speed);
                let total_frame_range = real_end_frame - real_start_frame + 1;

                // if current_frame_in_range = None, nothing will be drawn
                let current_frame_in_range = match &loop_type {
                    VideoLoopType::Loop => {
                        Some(amount_of_frames_since_start % total_frame_range as u16)
                    }
                    VideoLoopType::NoLoop => {
                        if amount_of_frames_since_start >= total_frame_range as u16 {
                            Some((total_frame_range - 1) as u16)
                        } else {
                            Some(amount_of_frames_since_start)
                        }
                    }
                    VideoLoopType::KillOnFinish => {
                        if amount_of_frames_since_start >= total_frame_range as u16 {
                            None
                        } else {
                            Some(amount_of_frames_since_start)
                        }
                    }

                    VideoLoopType::Boomerang => {
                        unimplemented!("fuck this")
                    }
                };
                if let Some(frame_in_range) = current_frame_in_range {
                    let frame_to_draw = real_start_frame + frame_in_range as usize;
                    let size = sprite.info.size();
                    out = self.draw_frame(sprite, frame_to_draw, (size.1, size.2))?;
                }
            }
        }
        Ok(out)
    }

    fn bounding_iv(&self, sprites: &SpriteRegistry) -> HashMap<i32, Vec<UpdateInterval>> {
        let size = sprites
            .get(&self.sprite_id)
            .map(|s| s.size())
            .unwrap_or((0, 0, 0));
        convert_rect_to_update_intervals(Rect {
            p1: self.position,
            p2: Point {
                x: self.position.x + size.1 as i32,
                y: self.position.y + size.2 as i32,
            },
        })
    }

    fn shifted(&self, offset: Point<i32>) -> Box<dyn Drawable> {
        Box::new(SpriteDrawable {
            position: self.position + offset,
            sprite_id: self.sprite_id,
            last_state_change: self.last_state_change,
            animation_type: self.animation_type.clone(),
        })
    }
}
