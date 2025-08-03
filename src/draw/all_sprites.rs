use ascii_assets::{AsciiSprite, AsciiVideo};
#[derive(Clone, Debug)]
pub enum SpriteObjectType {
    Sprite(AsciiSprite),
    SpriteVideo(AsciiVideo),
}

#[derive(Clone, Debug)]
pub struct SpriteObject {
    pub(crate) info: SpriteObjectType,
}

impl SpriteObject {
    pub fn size(&self) -> (usize, usize, usize) {
        match &self.info {
            SpriteObjectType::Sprite(sprite) => {
                let frames = 1;
                let height = sprite.height;
                let width = sprite.width;
                (frames, height as usize, width as usize)
            }
            SpriteObjectType::SpriteVideo(video) => video.size(),
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct AllSprites {
    pub sprites: Vec<SpriteObject>,
}

impl AllSprites {
    pub fn new() -> Self {
        AllSprites {
            sprites: Vec::new(),
        }
    }

    pub fn add(&mut self, obj: SpriteObject) -> usize {
        let idx = self.sprites.len();
        self.sprites.push(obj);

        idx
    }

    pub fn get(&self, id: &usize) -> Option<&SpriteObject> {
        self.sprites.get(*id)
    }
}
