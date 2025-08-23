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
    pub fn merge_creator(&mut self, creator: UpdateIntervalCreator) {
        let width = self.width as usize;
        let height = self.height as usize;

        for (y, iv) in creator.intervals.into_iter() {
            let y_usize = y as usize;

            // skip rows outside viewport
            if y_usize >= height {
                continue;
            }

            let y_offset = match y_usize.checked_mul(width) {
                Some(off) => off,
                None => continue,
            };

            let (x1, x2) = iv.interval;

            let start_x = x1.min(width);
            let end_x = x2.min(width);

            if start_x >= end_x {
                continue;
            }

            let start = y_offset + start_x;
            let end = y_offset + end_x;

            self.intervals.push(UpdateInterval {
                interval: (start, end),
                iv_type: iv.iv_type,
            });
        }
    }

    pub fn merge_redraw_regions(&mut self, hash: HashMap<u16, Vec<UpdateInterval>>) {
        let width = self.width as usize;
        let height = self.height as usize;

        for (y, ivs) in hash.into_iter() {
            let y_usize = y as usize;

            if y_usize >= height {
                continue;
            }

            let y_offset = match y_usize.checked_mul(width) {
                Some(off) => off,
                None => continue,
            };

            for iv in ivs {
                let (x1, x2) = iv.interval;
                let start_x = x1.min(width);
                let end_x = x2.min(width);

                if start_x >= end_x {
                    continue;
                }

                let start = y_offset + start_x;
                let end = y_offset + end_x;

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
        self.intervals.sort_by_key(|iv| iv.interval.0);
        let mut merged: Vec<UpdateInterval> = Vec::new();
        let mut current = self.intervals[0];

        for iv in self.intervals.iter().skip(1) {
            let (_, cur_end) = current.interval;
            let (new_start, new_end) = iv.interval;

            if new_start <= cur_end {
                current.interval.1 = cur_end.max(new_end);
                if iv.iv_type == UpdateIntervalType::Forced {
                    current.iv_type = UpdateIntervalType::Forced;
                }
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

/// public api to safely create update intervals
/// and later merge them into UpdateIntervalHandler
#[derive(Debug, Default, Clone)]
pub struct UpdateIntervalCreator {
    intervals: HashMap<u16, UpdateInterval>,
}

impl UpdateIntervalCreator {
    pub fn new() -> Self {
        Self {
            intervals: HashMap::new(),
        }
    }

    pub fn register_redraw_region(&mut self, rect: Rect<i32>) {
        let rect = rect.normalized();
        let rect_u16 = rect.map(|x: i32| {
            if x < 0 {
                0u16
            } else if x > u16::MAX as i32 {
                u16::MAX
            } else {
                x as u16
            }
        });

        let p1_y = rect_u16.p1.y;
        let p2_y = rect_u16.p2.y;

        if p1_y > p2_y {
            return;
        }

        for y in p1_y..=p2_y {
            let start_x = if y == p1_y { rect_u16.p1.x } else { 0 };
            let end_x = if y == p2_y { rect_u16.p2.x } else { u16::MAX };

            let entry = self.intervals.entry(y).or_insert(UpdateInterval {
                interval: (start_x as usize, end_x as usize),
                iv_type: UpdateIntervalType::Optimized,
            });

            entry.interval.0 = entry.interval.0.min(start_x as usize);
            entry.interval.1 = entry.interval.1.max(end_x as usize);
        }
    }

    /// Add a single interval expressed in i32 coordinates.
    /// - negative y is ignored,
    /// - negative interval endpoints are clamped to 0,
    /// - very large endpoints saturate to usize::MAX,
    /// - intervals are normalized (start <= end).
    pub fn add_interval(&mut self, y: i32, interval: (i32, i32)) {
        if y < 0 {
            // out of bounds in y, ignore
            return;
        }
        let y = y as u16;

        // safe conversion helper: clamp negatives to 0 and saturate to usize::MAX
        fn i32_to_usize_clamped(x: i32) -> usize {
            if x <= 0 {
                0usize
            } else {
                // use i128 to compare against usize::MAX safely on all platforms
                let xi = x as i128;
                let umax_i128 = usize::MAX as i128;
                if xi > umax_i128 {
                    usize::MAX
                } else {
                    x as usize
                }
            }
        }

        let mut start = i32_to_usize_clamped(interval.0);
        let mut end = i32_to_usize_clamped(interval.1);

        // normalize interval so start <= end
        if start > end {
            core::mem::swap(&mut start, &mut end);
        }

        let entry = self.intervals.entry(y).or_insert(UpdateInterval {
            interval: (start, end),
            iv_type: UpdateIntervalType::Optimized,
        });

        entry.interval.0 = entry.interval.0.min(start);
        entry.interval.1 = entry.interval.1.max(end);
    }

    pub fn dump_intervals(&mut self) -> HashMap<u16, UpdateInterval> {
        std::mem::take(&mut self.intervals)
    }
}
