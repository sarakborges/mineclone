use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

const MAX_BLOCK_STATE_ENTRIES: usize = 8;
const EMPTY_STATE_VALUE: u64 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BlockStateToken(u32);

#[derive(Default)]
struct BlockStateTokenInterner {
    by_value: HashMap<&'static str, u32>,
    values: Vec<&'static str>,
}

static BLOCK_STATE_TOKEN_INTERNER: OnceLock<Mutex<BlockStateTokenInterner>> = OnceLock::new();

/// Compact, palette-friendly state stored directly in a voxel.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct BlockState {
    // Zero means an unused slot. Token IDs start at one, so a key/value pair
    // fits in one u64 instead of storing two fat string pointers.
    values: [u64; MAX_BLOCK_STATE_ENTRIES],
}

impl Default for BlockState {
    fn default() -> Self {
        Self {
            values: [EMPTY_STATE_VALUE; MAX_BLOCK_STATE_ENTRIES],
        }
    }
}

impl BlockState {
    pub(crate) fn token(key: &str) -> BlockStateToken {
        BlockStateToken(intern_token(key))
    }

    pub(crate) fn get(self, key: &str) -> Option<&'static str> {
        let key = lookup_token(key)?;
        self.get_token(BlockStateToken(key))
    }

    pub(crate) fn get_token(self, key: BlockStateToken) -> Option<&'static str> {
        self.values
            .iter()
            .copied()
            .find(|packed| unpack_key(*packed) == key.0)
            .map(unpack_value)
            .map(resolve_token)
    }

    pub(crate) fn len(self) -> usize {
        self.values
            .iter()
            .filter(|&&packed| packed != EMPTY_STATE_VALUE)
            .count()
    }

    pub(crate) fn iter(self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.values
            .into_iter()
            .filter(|&packed| packed != EMPTY_STATE_VALUE)
            .map(|packed| {
                (
                    resolve_token(unpack_key(packed)),
                    resolve_token(unpack_value(packed)),
                )
            })
    }

    pub(crate) fn iter_for_save(self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.iter()
    }

    #[cfg(test)]
    pub(crate) fn with(mut self, key: &str, value: &str) -> Self {
        self.set(key, value);
        self
    }

    pub(crate) fn set(&mut self, key: &str, value: &str) {
        assert!(!key.is_empty(), "block state key cannot be empty");
        assert!(!value.is_empty(), "block state value cannot be empty");

        let key = intern_token(key);
        let value = intern_token(value);
        let packed = pack(key, value);

        if let Some(entry) = self
            .values
            .iter_mut()
            .find(|entry| unpack_key(**entry) == key)
        {
            *entry = packed;
            return;
        }

        let Some(slot) = self
            .values
            .iter_mut()
            .find(|slot| **slot == EMPTY_STATE_VALUE)
        else {
            panic!("voxel cannot hold more than {MAX_BLOCK_STATE_ENTRIES} block state entries");
        };

        *slot = packed;
    }

    pub(crate) fn remove(&mut self, key: &str) -> bool {
        let Some(key) = lookup_token(key) else {
            return false;
        };
        let Some(slot) = self
            .values
            .iter_mut()
            .find(|slot| unpack_key(**slot) == key)
        else {
            return false;
        };

        *slot = EMPTY_STATE_VALUE;
        true
    }
}

fn pack(key: u32, value: u32) -> u64 {
    debug_assert!(key > 0 && value > 0);
    (u64::from(key) << 32) | u64::from(value)
}

fn unpack_key(packed: u64) -> u32 {
    (packed >> 32) as u32
}

fn unpack_value(packed: u64) -> u32 {
    packed as u32
}

fn token_interner() -> &'static Mutex<BlockStateTokenInterner> {
    BLOCK_STATE_TOKEN_INTERNER.get_or_init(|| Mutex::new(BlockStateTokenInterner::default()))
}

fn lookup_token(token: &str) -> Option<u32> {
    token_interner()
        .lock()
        .expect("block state token interner lock was poisoned")
        .by_value
        .get(token)
        .copied()
}

fn intern_token(token: &str) -> u32 {
    let mut interner = token_interner()
        .lock()
        .expect("block state token interner lock was poisoned");

    if let Some(&interned) = interner.by_value.get(token) {
        return interned;
    }

    let next = interner
        .values
        .len()
        .checked_add(1)
        .and_then(|value| u32::try_from(value).ok())
        .expect("block state token id space exhausted");
    let interned = Box::leak(token.to_owned().into_boxed_str());
    interner.values.push(interned);
    interner.by_value.insert(interned, next);
    next
}

fn resolve_token(token: u32) -> &'static str {
    assert!(token > 0, "block state token zero is reserved");
    token_interner()
        .lock()
        .expect("block state token interner lock was poisoned")
        .values
        .get(token as usize - 1)
        .copied()
        .unwrap_or_else(|| panic!("missing block state token {token}"))
}

#[cfg(test)]
mod tests {
    use super::{BlockState, MAX_BLOCK_STATE_ENTRIES};

    #[test]
    fn stores_and_replaces_state_values() {
        let state = BlockState::default()
            .with("dyed", "red")
            .with("dyed", "blue");

        assert_eq!(state.get("dyed"), Some("blue"));
    }

    #[test]
    fn iterates_state_values() {
        let state = BlockState::default()
            .with("dyed", "blue")
            .with("variant", "mossy");
        let values = state.iter().collect::<Vec<_>>();

        assert!(values.contains(&("dyed", "blue")));
        assert!(values.contains(&("variant", "mossy")));
    }

    #[test]
    fn removes_state_values() {
        let mut state = BlockState::default().with("dyed", "red");
        assert!(state.remove("dyed"));
        assert_eq!(state.get("dyed"), None);
        assert!(!state.remove("dyed"));
    }

    #[test]
    fn compact_storage_keeps_fixed_state_slots_small() {
        assert_eq!(
            std::mem::size_of::<BlockState>(),
            MAX_BLOCK_STATE_ENTRIES * std::mem::size_of::<u64>()
        );
    }
}
