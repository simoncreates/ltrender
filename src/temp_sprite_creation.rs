use ascii_assets::AsciiVideo;
use ascii_assets::{AsciiSprite, TerminalChar};
use crossterm::style::Color;

use crate::draw::DrawError;
pub fn generate_sprites() -> Result<(), DrawError> {
    let width = 2;
    let height = 2;
    let frame1 = vec![
        TerminalChar {
            chr: 'A',
            fg_color: Some(Color::Rgb { r: 255, g: 0, b: 0 }),
            bg_color: None,
        },
        TerminalChar {
            chr: 'B',
            fg_color: None,
            bg_color: Some(Color::Rgb { r: 0, g: 255, b: 0 }),
        },
        TerminalChar {
            chr: 'C',
            fg_color: None,
            bg_color: None,
        },
        TerminalChar {
            chr: 'D',
            fg_color: None,
            bg_color: None,
        },
    ];

    let frame2 = vec![
        TerminalChar {
            chr: 'E',
            fg_color: None,
            bg_color: None,
        },
        TerminalChar {
            chr: 'F',
            fg_color: None,
            bg_color: None,
        },
        TerminalChar {
            chr: 'G',
            fg_color: None,
            bg_color: None,
        },
        TerminalChar {
            chr: 'H',
            fg_color: None,
            bg_color: None,
        },
    ];

    let video = AsciiVideo {
        width,
        height,
        frames: vec![
            AsciiSprite {
                pixels: frame1,
                width,
                height,
            },
            AsciiSprite {
                pixels: frame2,
                width,
                height,
            },
        ],
    };

    let path = "assets/debugging/test_video.ascv";

    video.write_to_file(path)?;
    Ok(())
}
