use std::collections::HashMap;

use ascii_assets::TerminalChar;

use crate::draw::ObjectId;

#[derive(Clone, Debug, Copy)]
pub struct CharacterInfo {
    pub display_id: ObjectId,
    pub layer: usize,
    pub screen_layer: usize,
    pub chr: TerminalChar,
}

#[derive(Clone, Debug)]
pub struct CharacterInfoList {
    pub info: HashMap<ObjectId, CharacterInfo>,
}
