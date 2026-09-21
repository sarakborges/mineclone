use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use super::microblock::CHISEL_MASK_PROPERTY;

const MAX_SECONDARY_PROPERTIES: usize = 8;
const EMPTY_PROPERTY_VALUE: u64 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SecondaryPropertyToken(u32);

#[derive(Default)]
struct SecondaryPropertyTokenInterner {
    by_value: HashMap<&'static str, u32>,
    values: Vec<&'static str>,
}

static SECONDARY_PROPERTY_TOKEN_INTERNER: OnceLock<Mutex<SecondaryPropertyTokenInterner>> =
    OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SecondaryProperties {
    // Zero means an unused slot. Token IDs start at one, so a property/value
    // pair fits in one u64 instead of storing two fat string pointers.
    values: [u64; MAX_SECONDARY_PROPERTIES],
}

impl Default for SecondaryProperties {
    fn default() -> Self {
        Self {
            values: [EMPTY_PROPERTY_VALUE; MAX_SECONDARY_PROPERTIES],
        }
    }
}

impl SecondaryProperties {
    pub(crate) fn token(property: &str) -> SecondaryPropertyToken {
        SecondaryPropertyToken(intern_token(property))
    }

    pub(crate) fn get(self, property: &str) -> Option<&'static str> {
        let property = lookup_token(property)?;
        self.get_token(SecondaryPropertyToken(property))
    }

    pub(crate) fn get_token(self, property: SecondaryPropertyToken) -> Option<&'static str> {
        self.values
            .iter()
            .copied()
            .find(|packed| unpack_property(*packed) == property.0)
            .map(unpack_value)
            .map(resolve_token)
    }

    pub(crate) fn contains_token(self, property: SecondaryPropertyToken) -> bool {
        self.values
            .iter()
            .copied()
            .any(|packed| unpack_property(packed) == property.0)
    }

    pub(crate) fn len(self) -> usize {
        self.values
            .iter()
            .filter(|&&packed| packed != EMPTY_PROPERTY_VALUE)
            .count()
    }

    /// Only public properties for HUD and rendering; the Chisel shape must not
    /// appear as a normal block property in tooltips or visual definitions.
    pub(crate) fn iter(self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.values
            .into_iter()
            .filter(|&packed| packed != EMPTY_PROPERTY_VALUE)
            .map(|packed| {
                (
                    resolve_token(unpack_property(packed)),
                    resolve_token(unpack_value(packed)),
                )
            })
            .filter(|(property, _)| *property != CHISEL_MASK_PROPERTY)
    }

    /// Snapshot serialization needs the complete cell state, including its
    /// private 8x8x8 occupancy mask. Keep this explicit instead of changing
    /// `iter()`, which also serves public presentation paths.
    pub(crate) fn iter_for_save(
        self,
    ) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.values
            .into_iter()
            .filter(|&packed| packed != EMPTY_PROPERTY_VALUE)
            .map(|packed| {
                (
                    resolve_token(unpack_property(packed)),
                    resolve_token(unpack_value(packed)),
                )
            })
    }

    #[cfg(test)]
    pub(crate) fn with(mut self, property: &str, value: &str) -> Self {
        self.set(property, value);
        self
    }

    pub(crate) fn set(&mut self, property: &str, value: &str) {
        assert!(!property.is_empty(), "secondary property id cannot be empty");
        assert!(!value.is_empty(), "secondary property value cannot be empty");

        let property = intern_token(property);
        let value = intern_token(value);
        let packed = pack(property, value);

        if let Some(entry) = self
            .values
            .iter_mut()
            .find(|entry| unpack_property(**entry) == property)
        {
            *entry = packed;
            return;
        }

        let Some(slot) = self
            .values
            .iter_mut()
            .find(|slot| **slot == EMPTY_PROPERTY_VALUE)
        else {
            panic!(
                "voxel cannot hold more than {MAX_SECONDARY_PROPERTIES} secondary properties"
            );
        };

        *slot = packed;
    }

    pub(crate) fn remove(&mut self, property: &str) -> bool {
        let Some(property) = lookup_token(property) else {
            return false;
        };
        let Some(slot) = self
            .values
            .iter_mut()
            .find(|slot| unpack_property(**slot) == property)
        else {
            return false;
        };

        *slot = EMPTY_PROPERTY_VALUE;
        true
    }
}

fn pack(property: u32, value: u32) -> u64 {
    debug_assert!(property > 0 && value > 0);
    (u64::from(property) << 32) | u64::from(value)
}

fn unpack_property(packed: u64) -> u32 {
    (packed >> 32) as u32
}

fn unpack_value(packed: u64) -> u32 {
    packed as u32
}

fn token_interner() -> &'static Mutex<SecondaryPropertyTokenInterner> {
    SECONDARY_PROPERTY_TOKEN_INTERNER
        .get_or_init(|| Mutex::new(SecondaryPropertyTokenInterner::default()))
}

fn lookup_token(token: &str) -> Option<u32> {
    token_interner()
        .lock()
        .expect("secondary property token interner lock was poisoned")
        .by_value
        .get(token)
        .copied()
}

fn intern_token(token: &str) -> u32 {
    let mut interner = token_interner()
        .lock()
        .expect("secondary property token interner lock was poisoned");

    if let Some(&interned) = interner.by_value.get(token) {
        return interned;
    }

    let next = interner
        .values
        .len()
        .checked_add(1)
        .and_then(|value| u32::try_from(value).ok())
        .expect("secondary property token id space exhausted");
    let interned = Box::leak(token.to_owned().into_boxed_str());
    interner.values.push(interned);
    interner.by_value.insert(interned, next);
    next
}

fn resolve_token(token: u32) -> &'static str {
    assert!(token > 0, "secondary property token zero is reserved");
    token_interner()
        .lock()
        .expect("secondary property token interner lock was poisoned")
        .values
        .get(token as usize - 1)
        .copied()
        .unwrap_or_else(|| panic!("missing secondary property token {token}"))
}

#[cfg(test)]
mod tests {
    use super::{MAX_SECONDARY_PROPERTIES, SecondaryProperties};

    #[test]
    fn stores_and_replaces_secondary_property_values() {
        let properties = SecondaryProperties::default()
            .with("dyed", "red")
            .with("dyed", "blue");

        assert_eq!(properties.get("dyed"), Some("blue"));
    }

    #[test]
    fn iterates_secondary_property_values() {
        let properties = SecondaryProperties::default()
            .with("dyed", "blue")
            .with("variant", "mossy");
        let values = properties.iter().collect::<Vec<_>>();

        assert!(values.contains(&("dyed", "blue")));
        assert!(values.contains(&("variant", "mossy")));
    }

    #[test]
    fn removes_secondary_property_values() {
        let mut properties = SecondaryProperties::default().with("dyed", "red");
        assert!(properties.remove("dyed"));
        assert_eq!(properties.get("dyed"), None);
        assert!(!properties.remove("dyed"));
    }

    #[test]
    fn compact_storage_keeps_fixed_property_slots_small() {
        assert_eq!(
            std::mem::size_of::<SecondaryProperties>(),
            MAX_SECONDARY_PROPERTIES * std::mem::size_of::<u64>()
        );
    }
}
