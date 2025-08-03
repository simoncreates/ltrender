use std::collections::HashMap;

use ascii_assets::TerminalChar;

use super::ObjId;

#[derive(Clone, Debug, Copy)]
pub struct CharacterInfo {
    pub display_id: ObjId,
    pub layer: usize,
    pub screen_layer: usize,
    pub chr: TerminalChar,
}

#[derive(Clone, Debug)]
pub struct CharacterInfoList {
    pub info: HashMap<ObjId, CharacterInfo>,
}
