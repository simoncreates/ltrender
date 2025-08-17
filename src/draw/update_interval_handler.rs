use std::collections::HashMap;

use common_stdx::Rect;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum UpdateIntervalType {
    Forced,
    Optimized,
}

#[derive(Debug, Clone, Copy)]
pub struct UpdateInterval {
    pub interval: (usize, usize),
    pub iv_type: UpdateIntervalType,
}

#[derive(Debug, Default)]
pub struct UpdateIntervalHandler {
    width: u16,
    height: u16,
    intervals: Vec<UpdateInterval>,
}

impl UpdateIntervalHandler {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            intervals: Vec::new(),
        }
    }

    pub fn register_redraw_region(&mut self, rect: Rect<u16>, iv_type: UpdateIntervalType) {
        let width = self.width as usize;
        let clamped_p2_y = rect.p2.y.min(self.height);

        for y in rect.p1.y..clamped_p2_y {
            let start_x = if y == rect.p1.y { rect.p1.x } else { 0 };
            let end_x = if y == rect.p2.y - 1 {
                rect.p2.x
            } else {
                self.width
            };

            let start = y as usize * width + start_x as usize;
            let end = y as usize * width + end_x as usize;

            self.intervals.push(UpdateInterval {
                interval: (start, end),
                iv_type,
            });
        }
    }

    pub fn merge_redraw_regions(&mut self, hash: HashMap<u16, Vec<UpdateInterval>>) {
        let width = self.width as usize;

        for (y, ivs) in hash.into_iter() {
            let y_offset = y as usize * width;

            for iv in ivs {
                let (x1, x2) = iv.interval;

                let start = y_offset + x1;
                let end = y_offset + x2;

                self.intervals.push(UpdateInterval {
                    interval: (start, end),
                    iv_type: iv.iv_type,
                });
            }
        }
    }

    pub fn invalidate_entire_screen(&mut self) {
        let total = self.width as usize * self.height as usize;
        self.intervals.push(UpdateInterval {
            interval: (0, total),
            iv_type: UpdateIntervalType::Forced,
        });
    }

    pub fn merge_intervals(&mut self) {
        if self.intervals.len() <= 1 {
            return;
        }

        self.intervals.sort_by_key(|iv| (iv.interval.0, iv.iv_type));
        let mut merged = Vec::new();
        let mut current = self.intervals[0];

        for iv in self.intervals.iter().skip(1) {
            if iv.iv_type != current.iv_type {
                merged.push(current);
                current = *iv;
                continue;
            }

            let (_, cur_end) = current.interval;
            let (new_start, new_end) = iv.interval;

            if new_start <= cur_end {
                current.interval.1 = cur_end.max(new_end);
            } else {
                merged.push(current);
                current = *iv;
            }
        }

        merged.push(current);
        self.intervals = merged;
    }

    /// used for making drawcalls, which cover a bigger region
    pub fn expand_regions(&mut self, amount: usize) {
        for iv in &mut self.intervals {
            iv.interval.1 = iv.interval.1.saturating_add(amount);
        }
    }

    pub fn dump_intervals(&mut self) -> Vec<UpdateInterval> {
        std::mem::take(&mut self.intervals)
    }
}
