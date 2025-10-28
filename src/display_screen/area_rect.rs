use common_stdx::{Point, Rect};

#[derive(Debug, Clone, Copy)]
pub enum AreaPoint {
    Point(Point<i32>),
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

impl AreaPoint {
    pub fn screen_point_to_point(&self, terminal_size: &(u16, u16)) -> Point<i32> {
        match self {
            AreaPoint::Point(p) => *p,
            AreaPoint::TopLeft => Point::from((0, 0)),
            AreaPoint::TopRight => Point::from((terminal_size.0 as i32, 0)),
            AreaPoint::BottomLeft => Point::from((0, terminal_size.1 as i32)),
            AreaPoint::BottomRight => Point::from((terminal_size.0 as i32, terminal_size.1 as i32)),
        }
    }
}

impl From<(usize, usize)> for AreaPoint {
    fn from(value: (usize, usize)) -> Self {
        let x = value.0.clamp(0, i32::MAX as usize) as i32;
        let y = value.1.clamp(0, i32::MAX as usize) as i32;
        AreaPoint::Point(Point::from((x, y)))
    }
}

impl From<(u16, u16)> for AreaPoint {
    fn from(value: (u16, u16)) -> Self {
        let x = value.0.clamp(0, i32::MAX as u16) as i32;
        let y = value.1.clamp(0, i32::MAX as u16) as i32;
        AreaPoint::Point(Point::from((x, y)))
    }
}

impl From<(i32, i32)> for AreaPoint {
    fn from(value: (i32, i32)) -> Self {
        AreaPoint::Point(Point::from(value))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AreaRect {
    FromPoints(AreaPoint, AreaPoint),
    FullScreen,
}

impl AreaRect {
    pub fn area_to_rect(&self, terminal_size: &(u16, u16)) -> Rect<i32> {
        match self {
            AreaRect::FromPoints(p1, p2) => Rect {
                p1: p1.screen_point_to_point(terminal_size),
                p2: p2.screen_point_to_point(terminal_size),
            },
            AreaRect::FullScreen => {
                let p1 = AreaPoint::TopLeft;
                let p2 = AreaPoint::BottomRight;
                Rect {
                    p1: p1.screen_point_to_point(terminal_size),
                    p2: p2.screen_point_to_point(terminal_size),
                }
            }
        }
    }
}
