use std::collections::HashMap;

use common_stdx::Rect;

use crate::draw::{UpdateInterval, update_interval_handler::UpdateIntervalType};

/// public api to safely create update intervals
/// and later merge them into UpdateIntervalHandler
#[derive(Debug, Default, Clone)]
pub struct UpdateIntervalCreator {
    pub intervals: HashMap<u16, UpdateInterval>,
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
