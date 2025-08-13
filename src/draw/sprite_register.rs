use ascii_assets::{AsciiSprite, AsciiVideo};
#[derive(Clone, Debug)]
pub enum SpriteData {
    Sprite(AsciiSprite),
    SpriteVideo(AsciiVideo),
}

#[derive(Clone, Debug)]
pub struct SpriteEntry {
    pub(crate) info: SpriteData,
}

impl SpriteEntry {
    pub fn size(&self) -> (usize, usize, usize) {
        match &self.info {
            SpriteData::Sprite(sprite) => {
                let frames = 1;
                let height = sprite.height;
                let width = sprite.width;
                (frames, height as usize, width as usize)
            }
            SpriteData::SpriteVideo(video) => video.size(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpriteRegistry {
    pub sprites: Vec<SpriteEntry>,
}

impl SpriteRegistry {
    pub fn new() -> Self {
        SpriteRegistry {
            sprites: Vec::new(),
        }
    }

    pub fn add(&mut self, obj: SpriteEntry) -> usize {
        let idx = self.sprites.len();
        self.sprites.push(obj);

        idx
    }

    pub fn get(&self, id: &usize) -> Option<&SpriteEntry> {
        self.sprites.get(*id)
    }
}
