use std::collections::HashMap;

use common_stdx::{Point, Rect};

use crate::{UpdateInterval, update_interval_handler::UpdateIntervalType};

/// public api to safely create update intervals
/// and later merge them into UpdateIntervalHandler
#[derive(Debug, Default, Clone)]
pub struct UpdateIntervalCreator {
    pub intervals: HashMap<u16, Vec<UpdateInterval>>,
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
            let start_x = rect_u16.p1.x;
            let end_x = rect_u16.p2.x.saturating_add(1);

            let intervals = self.intervals.entry(y).or_default();
            intervals.push(UpdateInterval {
                interval: (start_x as usize, end_x as usize),
                iv_type: UpdateIntervalType::Optimized,
            });
        }
    }

    /// Add a single interval expressed in i32 coordinates.
    /// - negative y is ignored,
    /// - negative interval endpoints are clamped to 0,
    /// - very large endpoints saturate to usize::MAX,
    /// - intervals are normalized (start <= end).
    pub fn add_interval(&mut self, y: i32, interval: (i32, i32)) {
        if y < 0 {
            return;
        }
        let y = y as u16;

        fn i32_to_usize_clamped(x: i32) -> usize {
            if x <= 0 {
                0usize
            } else {
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

        if start > end {
            core::mem::swap(&mut start, &mut end);
        }

        let intervals = self.intervals.entry(y).or_default();
        intervals.push(UpdateInterval {
            interval: (start, end),
            iv_type: UpdateIntervalType::Optimized,
        });
    }

    /// Shift all intervals by the given amount.
    /// This rebuilds the HashMap since y-values may change.
    pub fn shift_all_intervals_by(&mut self, shift_amount: Point<i32>) {
        let mut new_map: HashMap<u16, Vec<UpdateInterval>> = HashMap::new();
        let usize_max_i128 = usize::MAX as i128;

        for (y, iv_list) in self.intervals.drain() {
            let new_y_i32 = y as i32 + shift_amount.y;
            if new_y_i32 < 0 {
                continue;
            }
            let new_y = new_y_i32.min(u16::MAX as i32) as u16;

            for mut iv in iv_list {
                let mut start_i128 = iv.interval.0 as i128 + shift_amount.x as i128;
                let mut end_i128 = iv.interval.1 as i128 + shift_amount.x as i128;

                // clamp to [0, usize::MAX]
                start_i128 = start_i128.clamp(0, usize_max_i128);
                end_i128 = end_i128.clamp(0, usize_max_i128);

                if start_i128 > end_i128 {
                    core::mem::swap(&mut start_i128, &mut end_i128);
                }

                iv.interval.0 = start_i128 as usize;
                iv.interval.1 = end_i128 as usize;

                new_map.entry(new_y).or_default().push(iv);
            }
        }

        self.intervals = new_map;
    }

    pub fn dump_intervals(&mut self) -> HashMap<u16, Vec<UpdateInterval>> {
        std::mem::take(&mut self.intervals)
    }
}
