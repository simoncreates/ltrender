use std::collections::HashMap;

use crate::draw::update_interval_handler::UpdateIntervalCreator;

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

        for (y, iv_list) in creator.intervals.into_iter() {
            let y_usize = y as usize;

            // skip rows outside viewport
            if y_usize >= height {
                continue;
            }

            let y_offset = match y_usize.checked_mul(width) {
                Some(off) => off,
                None => continue,
            };

            for iv in iv_list {
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
