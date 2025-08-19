use ascii_assets::AsciiVideo;

#[derive(Clone, Debug)]
pub struct SpriteEntry {
    pub(crate) info: AsciiVideo,
}

impl SpriteEntry {
    pub fn size(&self) -> (usize, usize, usize) {
        self.info.size()
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
