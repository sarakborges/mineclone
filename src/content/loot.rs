use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LootEntryDefinition {
    pub(crate) item: String,
    #[serde(default = "default_loot_chance")]
    pub(crate) chance: f32,
    #[serde(default = "default_loot_quantity")]
    pub(crate) quantity: u32,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(transparent)]
pub(crate) struct LootTableDefinition(Vec<LootEntryDefinition>);

impl LootTableDefinition {
    pub(crate) fn entries(&self) -> &[LootEntryDefinition] {
        &self.0
    }

    pub(crate) fn validate(&self, owner: &str) {
        for (index, entry) in self.0.iter().enumerate() {
            assert!(
                !entry.item.trim().is_empty(),
                "{owner} lootTable[{index}].item cannot be empty"
            );
            assert!(
                entry.chance.is_finite() && (0.0..=1.0).contains(&entry.chance),
                "{owner} lootTable[{index}].chance must be between 0 and 1"
            );
            assert!(
                entry.quantity > 0,
                "{owner} lootTable[{index}].quantity must be greater than zero"
            );
        }
    }

    pub(crate) fn validate_references(
        &self,
        owner: &str,
        item_exists: impl Fn(&str) -> bool,
    ) {
        for (index, entry) in self.0.iter().enumerate() {
            assert!(
                item_exists(&entry.item),
                "{owner} lootTable[{index}] references unknown item {}",
                entry.item
            );
        }
    }
}

fn default_loot_chance() -> f32 {
    1.0
}

fn default_loot_quantity() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loot_entries_default_to_guaranteed_single_drop() {
        let table: LootTableDefinition =
            serde_json::from_str(r#"[{"item":"asteria:dirt"}]"#).unwrap();
        let entry = &table.entries()[0];
        assert_eq!(entry.chance, 1.0);
        assert_eq!(entry.quantity, 1);
    }

    #[test]
    #[should_panic(expected = "chance must be between 0 and 1")]
    fn invalid_chance_is_rejected() {
        let table: LootTableDefinition =
            serde_json::from_str(r#"[{"item":"asteria:dirt","chance":1.1}]"#).unwrap();
        table.validate("test block");
    }
}
