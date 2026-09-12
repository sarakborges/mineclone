use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

const MAX_SECONDARY_PROPERTIES: usize = 8;

static SECONDARY_PROPERTY_TOKEN_INTERNER: OnceLock<Mutex<HashMap<String, &'static str>>> =
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
}
