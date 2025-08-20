use ascii_assets::Color;
use dyn_clone::DynClone;
use std::fmt::Debug;

use crate::draw::terminal_buffer::drawable::BasicDraw;

pub trait Shader: Send + Sync + Debug + DynClone {
    fn apply(&self, draw: &mut BasicDraw, frame_size: (usize, usize));
}

dyn_clone::clone_trait_object!(Shader);

#[derive(Debug, Clone, Copy)]
pub struct FlipHorizontal;

#[derive(Debug, Clone, Copy)]
pub struct FlipVertical;

impl Shader for FlipHorizontal {
    fn apply(&self, draw: &mut BasicDraw, frame_size: (usize, usize)) {
        let rel_x = draw.pos.x % frame_size.0 as i32;
        let new_x = (frame_size.0 - 1) - rel_x as usize;
        draw.pos.x = new_x as i32;
    }
}

impl Shader for FlipVertical {
    fn apply(&self, draw: &mut BasicDraw, frame_size: (usize, usize)) {
        let rel_y = draw.pos.y % frame_size.1 as i32;
        let new_y = (frame_size.1 - 1) - rel_y as usize;
        draw.pos.y = new_y as i32;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FlipDiagonal;

impl Shader for FlipDiagonal {
    fn apply(&self, draw: &mut BasicDraw, frame_size: (usize, usize)) {
        let new_x = draw.pos.y;
        let new_y = draw.pos.x;
        if new_x < frame_size.0 as i32 && new_y < frame_size.1 as i32 {
            draw.pos.x = new_x;
            draw.pos.y = new_y;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Checkerboard;

impl Shader for Checkerboard {
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize)) {
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
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize)) {
        if draw.pos.y % 2 == 0 {
            draw.chr.bg_color = Some(Color::rgb(100, 100, 100));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StripesVertical;

impl Shader for StripesVertical {
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize)) {
        if draw.pos.x % 2 == 0 {
            draw.chr.bg_color = Some(Color::rgb(100, 100, 100));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ToUpperCase;

impl Shader for ToUpperCase {
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize)) {
        draw.chr.chr = draw.chr.chr.to_ascii_uppercase();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ToLowerCase;

impl Shader for ToLowerCase {
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize)) {
        draw.chr.chr = draw.chr.chr.to_ascii_lowercase();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Brighten(pub u8);

impl Shader for Brighten {
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize)) {
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
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize)) {
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
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize)) {
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
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize)) {
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
    fn apply(&self, draw: &mut BasicDraw, _frame_size: (usize, usize)) {
        std::mem::swap(&mut draw.chr.fg_color, &mut draw.chr.bg_color);
    }
}
