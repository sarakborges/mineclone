use std::collections::HashMap;

pub(super) struct DefinitionMap<T> {
    definitions: HashMap<String, T>,
}

impl<T> Default for DefinitionMap<T> {
    fn default() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }
}

impl<T> DefinitionMap<T> {
    pub(super) fn insert(&mut self, id: String, definition: T) {
        self.definitions.insert(id, definition);
    }

    pub(super) fn get(&self, id: &str) -> Option<&T> {
        self.definitions.get(id)
    }

    pub(super) fn values(&self) -> impl Iterator<Item = &T> {
        self.definitions.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacing_an_id_keeps_a_single_definition() {
        let mut definitions = DefinitionMap::default();
        definitions.insert("test".to_owned(), 1);
        definitions.insert("test".to_owned(), 2);

        assert_eq!(definitions.get("test"), Some(&2));
        assert_eq!(definitions.values().count(), 1);
    }
}
