use common_stdx::Rect;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct UpdateInterval {
    pub interval: (u16, u16),
}
#[derive(Debug)]
pub struct UpdateIntervalHandler {
    pub intervals: HashMap<u16, Vec<UpdateInterval>>,
}

impl UpdateIntervalHandler {
    pub fn new() -> Self {
        UpdateIntervalHandler {
            intervals: HashMap::new(),
        }
    }

    pub fn add_update_rect(&mut self, rect: Rect<u16>) {
        let iv = (rect.p1.x, rect.p2.x);

        for y in rect.p1.y..rect.p2.y {
            let intv = UpdateInterval { interval: iv };
            self.intervals.entry(y).or_default().push(intv);
        }
    }
    pub fn add_update_hash(&mut self, hash: HashMap<u16, Vec<UpdateInterval>>) {
        for (y, intervals) in hash {
            self.intervals.entry(y).or_default().extend(intervals);
        }
    }
    pub fn merge_intervals(&mut self) {
        for (_y, intervals) in self.intervals.iter_mut() {
            if intervals.len() <= 1 {
                continue;
            }

            // sort intervals by x pos
            intervals.sort_by_key(|iv| iv.interval.0);

            let mut merged = Vec::new();
            let mut current = intervals[0];

            for iv in intervals.iter().skip(1) {
                let (_, cur_end) = current.interval;
                let (new_start, new_end) = iv.interval;

                if new_start <= cur_end {
                    current.interval.1 = cur_end.max(new_end);
                } else {
                    // no overlap, push and start a new one
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
