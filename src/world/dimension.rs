use std::fmt;

use bevy::prelude::*;

use crate::content::builtin_ids::OVERWORLD_DIMENSION_ID;

pub const DEFAULT_DIMENSION_ID: &str = OVERWORLD_DIMENSION_ID;

/// Stable runtime identity for a world dimension.
///
/// Authored content may still deserialize textual IDs at the content boundary,
/// but runtime world state should not confuse an arbitrary `String` with a
/// dimension identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DimensionId(String);

impl DimensionId {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.trim().is_empty(), "dimension id cannot be empty");
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for DimensionId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for DimensionId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for DimensionId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for DimensionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Resource)]
pub struct CurrentDimension {
    pub id: DimensionId,
}

impl Default for CurrentDimension {
    fn default() -> Self {
        Self {
            id: DimensionId::from(DEFAULT_DIMENSION_ID),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_id_preserves_valid_identity() {
        let id = DimensionId::new("asteria:overworld");
        assert_eq!(id.as_str(), "asteria:overworld");
        assert_eq!(id.to_string(), "asteria:overworld");
    }

    #[test]
    #[should_panic(expected = "dimension id cannot be empty")]
    fn dimension_id_rejects_blank_identity() {
        let _ = DimensionId::new("   ");
    }
}
