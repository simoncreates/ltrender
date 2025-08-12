use common_stdx::Rect;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum UpdateIntervalType {
    Forced,
    Optimized,
}

#[derive(Debug, Clone, Copy)]
pub struct UpdateInterval {
    pub interval: (u16, u16),
    pub iv_type: UpdateIntervalType,
}

#[derive(Debug, Default)]
pub struct UpdateIntervalHandler {
    pub intervals: HashMap<u16, Vec<UpdateInterval>>,
}

impl UpdateIntervalHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_redraw_region(&mut self, rect: Rect<u16>, iv_type: UpdateIntervalType) {
        let interval = (rect.p1.x, rect.p2.x);

        for y in rect.p1.y..rect.p2.y {
            let upd = UpdateInterval { interval, iv_type };
            self.intervals.entry(y).or_default().push(upd);
        }
    }

    pub fn merge_redraw_regions(&mut self, hash: HashMap<u16, Vec<UpdateInterval>>) {
        for (y, ivs) in hash.into_iter() {
            let entry = self.intervals.entry(y).or_default();
            entry.extend(ivs);
        }
    }

    pub fn invalidate_entire_screen(&mut self, terminal_size: (u16, u16)) {
        for y in 0..terminal_size.1 {
            self.intervals.entry(y).or_default().push(UpdateInterval {
                interval: (0, terminal_size.0),
                iv_type: UpdateIntervalType::Forced,
            });
        }
    }
    pub fn merge_intervals(&mut self) {
        for (_y, intervals) in self.intervals.iter_mut() {
            if intervals.len() <= 1 {
                continue;
            }
            intervals.sort_by_key(|iv| (iv.interval.0, iv.iv_type));

            let mut merged: Vec<UpdateInterval> = Vec::new();
            let mut current = intervals[0];

            for iv in intervals.iter().skip(1) {
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
            *intervals = merged;
        }
    }
    pub fn dump_intervals(&mut self) -> HashMap<u16, Vec<UpdateInterval>> {
        std::mem::take(&mut self.intervals)
    }
}
