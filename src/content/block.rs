use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use bevy::prelude::*;
use serde::Deserialize;

static BLOCK_ID_INTERNER: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockTextures {
    pub top: String,
    pub bottom: String,
    pub left: String,
    pub right: String,
    pub front: String,
    pub back: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDefinition {
    pub id: String,
    pub name: String,
    pub textures: BlockTextures,
    pub rotate_texture: bool,
    #[serde(default)]
    pub light_emission: u8,
}

#[derive(Resource, Default)]
pub struct BlockRegistry {
    definitions: HashMap<String, BlockDefinition>,
    static_ids: HashMap<String, &'static str>,
}

impl BlockRegistry {
    pub fn insert(&mut self, definition: BlockDefinition) {
        assert!(
            definition.light_emission <= 15,
            "block {} light emission must be between 0 and 15",
            definition.id
        );

        let static_id = intern_block_id(&definition.id);

        self.static_ids.insert(definition.id.clone(), static_id);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BlockDefinition> {
        self.definitions.get(id)
    }

    pub fn static_id(&self, id: &str) -> Option<&'static str> {
        self.static_ids.get(id).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockDefinition> {
        self.definitions.values()
    }
}

pub(crate) fn intern_block_id(id: &str) -> &'static str {
    let interner = BLOCK_ID_INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut ids = interner
        .lock()
        .expect("block ID interner lock was poisoned");

    if let Some(&interned) = ids.get(id) {
        return interned;
    }

    let interned = Box::leak(id.to_owned().into_boxed_str());
    ids.insert(id.to_owned(), interned);
    interned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_ids_are_interned_once_per_process() {
        let first = intern_block_id("mineclone:test");
        let second = intern_block_id("mineclone:test");

        assert!(std::ptr::eq(first.as_ptr(), second.as_ptr()));
    }
}
