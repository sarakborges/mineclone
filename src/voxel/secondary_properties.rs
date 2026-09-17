use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use super::microblock::CHISEL_MASK_PROPERTY;

const MAX_SECONDARY_PROPERTIES: usize = 8;

// Store each canonical token only once. The previous HashMap<String, &str>
// duplicated every token as an owned key in addition to the leaked value.
static SECONDARY_PROPERTY_TOKEN_INTERNER: OnceLock<Mutex<HashSet<&'static str>>> =
    OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SecondaryPropertyValue {
    property: &'static str,
    value: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SecondaryProperties {
    values: [Option<SecondaryPropertyValue>; MAX_SECONDARY_PROPERTIES],
}

impl Default for SecondaryProperties {
    fn default() -> Self {
        Self {
            values: [None; MAX_SECONDARY_PROPERTIES],
        }
    }
}

impl SecondaryProperties {
    pub(crate) fn get(self, property: &str) -> Option<&'static str> {
        self.values
            .iter()
            .flatten()
            .find(|entry| entry.property == property)
            .map(|entry| entry.value)
    }

    /// Only public properties for HUD and rendering; the Chisel shape must not
    /// appear as a normal block property in tooltips or visual definitions.
    pub(crate) fn iter(self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.values
            .into_iter()
            .flatten()
            .filter(|entry| entry.property != CHISEL_MASK_PROPERTY)
            .map(|entry| (entry.property, entry.value))
    }

    /// Snapshot serialization needs the complete cell state, including its
    /// private 8x8x8 occupancy mask. Keep this explicit instead of changing
    /// `iter()`, which also serves public presentation paths.
    pub(crate) fn iter_for_save(self) -> impl Iterator<Item = (&'static str, &'static str)> {
        self.values
            .into_iter()
            .flatten()
            .map(|entry| (entry.property, entry.value))
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

        if let Some(entry) = self
            .values
            .iter_mut()
            .flatten()
            .find(|entry| entry.property == property)
        {
            entry.value = value;
            return;
        }

        let Some(slot) = self.values.iter_mut().find(|slot| slot.is_none()) else {
            panic!(
                "voxel cannot hold more than {MAX_SECONDARY_PROPERTIES} secondary properties"
            );
        };

        *slot = Some(SecondaryPropertyValue { property, value });
    }

    pub(crate) fn remove(&mut self, property: &str) -> bool {
        let Some(slot) = self.values.iter_mut().find(|slot| {
            slot.as_ref()
                .is_some_and(|entry| entry.property == property)
        }) else {
            return false;
        };

        *slot = None;
        true
    }
}

fn intern_token(token: &str) -> &'static str {
    let interner = SECONDARY_PROPERTY_TOKEN_INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut tokens = interner
        .lock()
        .expect("secondary property token interner lock was poisoned");

    if let Some(&interned) = tokens.get(token) {
        return interned;
    }

    let interned = Box::leak(token.to_owned().into_boxed_str());
    tokens.insert(token.to_owned(), interned);
    interned
}

#[cfg(test)]
mod tests {
    use super::SecondaryProperties;

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
}
