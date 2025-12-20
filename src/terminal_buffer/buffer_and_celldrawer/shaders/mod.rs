use ascii_assets::Color;
use common_stdx::Point;
use dyn_clone::DynClone;
use std::fmt::Debug;

use crate::BasicDraw;

pub trait Shader: Send + Sync + Debug + DynClone {
    fn apply(&self, draw: &mut BasicDraw, frame_size: (usize, usize), top_left: Point<i32>);
}

dyn_clone::clone_trait_object!(Shader);

#[derive(Debug, Clone, Copy)]
pub struct FlipHorizontal;

#[derive(Debug, Clone, Copy)]
pub struct FlipVertical;

impl Shader for FlipHorizontal {
    fn apply(&self, draw: &mut BasicDraw, frame_size: (usize, usize), top_left: Point<i32>) {
        let local_x = draw.pos.x - top_left.x;
        let flipped_x = (frame_size.0 - 1) as i32 - local_x;
        draw.pos.x = top_left.x + flipped_x;
    }
}

impl Shader for FlipVertical {
    fn apply(&self, draw: &mut BasicDraw, frame_size: (usize, usize), top_left: Point<i32>) {
        let local_y = draw.pos.y - top_left.y;
        let flipped_y = (frame_size.1 - 1) as i32 - local_y;
        draw.pos.y = top_left.y + flipped_y;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FlipDiagonal;

impl Shader for FlipDiagonal {
    fn apply(&self, draw: &mut BasicDraw, frame_size: (usize, usize), top_left: Point<i32>) {
        let local_x = draw.pos.x - top_left.x;
        let local_y = draw.pos.y - top_left.y;
        if local_x < frame_size.0 as i32 && local_y < frame_size.1 as i32 {
            draw.pos.x = top_left.x + local_y;
            draw.pos.y = top_left.y + local_x;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Checkerboard;

impl Shader for Checkerboard {
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize), _top_left: Point<i32>) {
        if (draw.pos.x + draw.pos.y) % 2 == 0 {
            draw.chr.bg_color = Some(Color::rgb(0, 0, 0));
        } else {
            draw.chr.bg_color = Some(Color::rgb(255, 255, 255));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StripesHorizontal;

impl Shader for StripesHorizontal {
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize), _top_left: Point<i32>) {
        if draw.pos.y % 2 == 0 {
            draw.chr.bg_color = Some(Color::rgb(100, 100, 100));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StripesVertical;

impl Shader for StripesVertical {
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize), _top_left: Point<i32>) {
        if draw.pos.x % 2 == 0 {
            draw.chr.bg_color = Some(Color::rgb(100, 100, 100));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ToUpperCase;

impl Shader for ToUpperCase {
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize), _top_left: Point<i32>) {
        draw.chr.chr = draw.chr.chr.to_ascii_uppercase();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ToLowerCase;

impl Shader for ToLowerCase {
    fn apply(
        &self,
        draw: &mut BasicDraw,
        _frame_size: (usize, usize),
        _top_left: common_stdx::Point<i32>,
    ) {
        draw.chr.chr = draw.chr.chr.to_ascii_lowercase();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Brighten(pub u8);

impl Shader for Brighten {
    fn apply(
        &self,
        draw: &mut BasicDraw,
        _frame_size: (usize, usize),
        _top_left: common_stdx::Point<i32>,
    ) {
        fn brighten_color(c: Color, amount: u8) -> Color {
            let (r, g, b) = c.rgb;
            Color::rgb(
                r.saturating_add(amount),
                g.saturating_add(amount),
                b.saturating_add(amount),
            )
        }
        if let Some(fg) = draw.chr.fg_color {
            draw.chr.fg_color = Some(brighten_color(fg, self.0));
        }
        if let Some(bg) = draw.chr.bg_color {
            draw.chr.bg_color = Some(brighten_color(bg, self.0));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Darken(pub u8);

impl Shader for Darken {
    fn apply(
        &self,
        draw: &mut BasicDraw,
        _frame_size: (usize, usize),
        _top_left: common_stdx::Point<i32>,
    ) {
        fn darken_color(c: Color, amount: u8) -> Color {
            let (r, g, b) = c.rgb;
            Color::rgb(
                r.saturating_sub(amount),
                g.saturating_sub(amount),
                b.saturating_sub(amount),
            )
        }
        if let Some(fg) = draw.chr.fg_color {
            draw.chr.fg_color = Some(darken_color(fg, self.0));
        }
        if let Some(bg) = draw.chr.bg_color {
            draw.chr.bg_color = Some(darken_color(bg, self.0));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InvertColors;

impl Shader for InvertColors {
    fn apply(
        &self,
        draw: &mut BasicDraw,
        _frame_size: (usize, usize),
        _top_left: common_stdx::Point<i32>,
    ) {
        fn invert(c: Color) -> Color {
            let (r, g, b) = c.rgb;
            Color::rgb(255 - r, 255 - g, 255 - b)
        }
        if let Some(fg) = draw.chr.fg_color {
            draw.chr.fg_color = Some(invert(fg));
        }
        if let Some(bg) = draw.chr.bg_color {
            draw.chr.bg_color = Some(invert(bg));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Grayscale;

impl Shader for Grayscale {
    fn apply(
        &self,
        draw: &mut BasicDraw,
        _frame_size: (usize, usize),
        _top_left: common_stdx::Point<i32>,
    ) {
        fn gray(c: Color) -> Color {
            let (r, g, b) = c.rgb;
            let avg = ((r as u16 + g as u16 + b as u16) / 3) as u8;
            Color::rgb(avg, avg, avg)
        }
        if let Some(fg) = draw.chr.fg_color {
            draw.chr.fg_color = Some(gray(fg));
        }
        if let Some(bg) = draw.chr.bg_color {
            draw.chr.bg_color = Some(gray(bg));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SwapColors;

impl Shader for SwapColors {
    fn apply(
        &self,
        draw: &mut BasicDraw,
        _frame_size: (usize, usize),
        _top_left: common_stdx::Point<i32>,
    ) {
        std::mem::swap(&mut draw.chr.fg_color, &mut draw.chr.bg_color);
    }
}
