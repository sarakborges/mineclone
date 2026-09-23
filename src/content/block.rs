use bevy::prelude::*;
use smallvec::SmallVec;
use serde::{Deserialize, Deserializer};

use crate::localization::LocalizedText;

use super::{
    asset_path::is_safe_relative_asset_path,
    block_id::intern_block_id,
    block_orientation::BlockOrientation,
    inventory_category::InventoryCategoryRegistry,
    registry::DefinitionMap,
    secondary_property::SecondaryPropertyRegistry,
    tool::ToolRegistry,
    tool_category::ToolCategoryRegistry,
};

const MAX_LIGHT_DAMPENING: u8 = 15;
pub const DEFAULT_BLOCK_BREAK_TICKS: u32 = 200;
pub const FRAGMENTABLE_BLOCK_TAG: &str = "fragmentable";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockMiningDefinition {
    #[serde(default = "default_mining_hardness")]
    pub hardness: f32,
    #[serde(default)]
    pub required_tools: Vec<String>,
    #[serde(default)]
    pub preferred_tools: Vec<String>,
}

impl Default for BlockMiningDefinition {
    fn default() -> Self {
        Self {
            hardness: default_mining_hardness(),
            required_tools: Vec::new(),
            preferred_tools: Vec::new(),
        }
    }
}

impl BlockMiningDefinition {
    fn validate(&self, block_id: &str) {
        assert!(
            self.hardness.is_finite() && self.hardness >= 0.0,
            "block {block_id} mining hardness must be finite and non-negative"
        );
        validate_mining_tool_tags(block_id, "requiredTools", &self.required_tools);
        validate_mining_tool_tags(block_id, "preferredTools", &self.preferred_tools);
    }
}

fn validate_mining_tool_tags(block_id: &str, field: &str, tags: &[String]) {
    for (index, tag) in tags.iter().enumerate() {
        assert!(
            !tag.trim().is_empty(),
            "block {block_id} mining {field} cannot contain empty values"
        );
        assert!(
            !tags[..index].contains(tag),
            "block {block_id} mining {field} cannot contain duplicates"
        );
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BlockTextureLayer {
    pub texture: String,
    #[serde(default, alias = "dyeable")]
    pub dyable: bool,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockTextures {
    #[serde(default, deserialize_with = "deserialize_texture_layers")]
    pub top: Vec<BlockTextureLayer>,
    #[serde(default, deserialize_with = "deserialize_texture_layers")]
    pub bottom: Vec<BlockTextureLayer>,
    #[serde(default, deserialize_with = "deserialize_texture_layers")]
    pub left: Vec<BlockTextureLayer>,
    #[serde(default, deserialize_with = "deserialize_texture_layers")]
    pub right: Vec<BlockTextureLayer>,
    #[serde(default, deserialize_with = "deserialize_texture_layers")]
    pub front: Vec<BlockTextureLayer>,
    #[serde(default, deserialize_with = "deserialize_texture_layers")]
    pub back: Vec<BlockTextureLayer>,
}

impl BlockTextures {
    pub fn is_empty(&self) -> bool {
        self.top.is_empty()
            && self.bottom.is_empty()
            && self.left.is_empty()
            && self.right.is_empty()
            && self.front.is_empty()
            && self.back.is_empty()
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TextureLayerValue {
    Legacy(String),
    Layer(BlockTextureLayer),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TextureLayersValue {
    Legacy(String),
    Layer(BlockTextureLayer),
    Multiple(Vec<TextureLayerValue>),
}

fn deserialize_texture_layers<'de, D>(deserializer: D) -> Result<Vec<BlockTextureLayer>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = TextureLayersValue::deserialize(deserializer)?;
    Ok(match value {
        TextureLayersValue::Legacy(texture) => vec![BlockTextureLayer {
            texture,
            dyable: false,
        }],
        TextureLayersValue::Layer(layer) => vec![layer],
        TextureLayersValue::Multiple(layers) => layers
            .into_iter()
            .map(|layer| match layer {
                TextureLayerValue::Legacy(texture) => BlockTextureLayer {
                    texture,
                    dyable: false,
                },
                TextureLayerValue::Layer(layer) => layer,
            })
            .collect(),
    })
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockTextureRotations {
    #[serde(default)]
    pub top: bool,
    #[serde(default)]
    pub bottom: bool,
    #[serde(default)]
    pub left: bool,
    #[serde(default)]
    pub right: bool,
    #[serde(default)]
    pub front: bool,
    #[serde(default)]
    pub back: bool,
}

impl BlockTextureRotations {
    pub fn any(self) -> bool {
        self.top || self.bottom || self.left || self.right || self.front || self.back
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BlockTint {
    #[default]
    None,
    Grass,
    Leaf,
    Foliage,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub category: String,
    #[serde(default)]
    pub mining: BlockMiningDefinition,
    /// Opt-in capabilities. Blocks without `fragmentable` cannot be sculpted.
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tint: BlockTint,
    #[serde(default)]
    pub textures: BlockTextures,
    #[serde(default)]
    pub rotate_texture: BlockTextureRotations,
    #[serde(default)]
    pub orientations: Vec<BlockOrientation>,
    #[serde(default)]
    pub secondary_properties: Vec<String>,
    #[serde(default)]
    pub alpha_cutoff: Option<f32>,
    #[serde(default)]
    pub alpha_blend: bool,
    #[serde(default)]
    pub light_emission: u8,
    #[serde(default = "default_light_dampening")]
    pub light_dampening: u8,
    #[serde(default = "default_casts_shadow")]
    pub casts_shadow: bool,
}

impl BlockDefinition {
    pub fn can_fragment(&self) -> bool {
        self.tags.iter().any(|tag| tag == FRAGMENTABLE_BLOCK_TAG)
    }

    pub fn alpha_mode(&self, opacity: f32) -> AlphaMode {
        if opacity < 1.0 || self.alpha_blend {
            AlphaMode::Blend
        } else if let Some(cutoff) = self.alpha_cutoff {
            AlphaMode::Mask(cutoff)
        } else {
            AlphaMode::Opaque
        }
    }

    pub fn default_orientation(&self) -> BlockOrientation {
        self.orientations.first().copied().unwrap_or_default()
    }

    pub fn is_rotatable(&self) -> bool {
        self.orientations.len() > 1
    }

    pub fn next_orientation(&self, current: BlockOrientation) -> BlockOrientation {
        let Some(&first) = self.orientations.first() else {
            return BlockOrientation::default();
        };

        let Some(index) = self
            .orientations
            .iter()
            .position(|orientation| *orientation == current)
        else {
            return first;
        };

        self.orientations[(index + 1) % self.orientations.len()]
    }

    pub(crate) fn validate_references(
        &self,
        inventory_categories: &InventoryCategoryRegistry,
        secondary_properties: &SecondaryPropertyRegistry,
        tool_categories: &ToolCategoryRegistry,
        tools: &ToolRegistry,
    ) {
        assert!(
            inventory_categories.get(&self.category).is_some(),
            "block {} references missing inventory category {}",
            self.id,
            self.category
        );

        for property in &self.secondary_properties {
            assert!(
                secondary_properties.contains_property(property),
                "block {} references missing secondary property {}",
                self.id,
                property
            );
        }

        for category in self
            .mining
            .required_tools
            .iter()
            .chain(self.mining.preferred_tools.iter())
        {
            assert!(
                tool_categories.get(category).is_some(),
                "block {} mining references unknown tool category {}",
                self.id,
                category
            );
            assert!(
                tools
                    .iter()
                    .any(|tool| tool.mining.matches_category(category)),
                "block {} mining references tool category {} with no matching tool",
                self.id,
                category
            );
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct BlockRegistry {
    definitions: DefinitionMap<BlockDefinition>,
}

impl BlockRegistry {
    pub fn insert(&mut self, definition: BlockDefinition) {
        assert!(!definition.id.trim().is_empty(), "block id cannot be empty");
        assert!(
            !definition.category.trim().is_empty(),
            "block {} category cannot be empty",
            definition.id
        );
        definition.name.validate(&format!("block {} name", definition.id));
        definition.mining.validate(&definition.id);
        assert!(
            definition.light_emission <= MAX_LIGHT_DAMPENING,
            "block {} light emission must be between 0 and 15",
            definition.id
        );
        assert!(
            definition.light_dampening <= MAX_LIGHT_DAMPENING,
            "block {} light dampening must be between 0 and 15",
            definition.id
        );
        if let Some(alpha_cutoff) = definition.alpha_cutoff {
            assert!(
                (0.0..=1.0).contains(&alpha_cutoff),
                "block {} alphaCutoff must be between 0 and 1",
                definition.id
            );
        }
        assert!(
            !definition.alpha_blend || definition.alpha_cutoff.is_none(),
            "block {} cannot use alphaBlend and alphaCutoff together",
            definition.id
        );
        for (index, tag) in definition.tags.iter().enumerate() {
            assert!(
                !tag.trim().is_empty(),
                "block {} tags cannot contain empty values",
                definition.id
            );
            assert!(
                !definition.tags[..index].contains(tag),
                "block {} tags cannot contain duplicates",
                definition.id
            );
        }
        for (index, orientation) in definition.orientations.iter().enumerate() {
            assert!(
                !definition.orientations[..index].contains(orientation),
                "block {} orientations cannot contain duplicates",
                definition.id
            );
        }
        for (index, property) in definition.secondary_properties.iter().enumerate() {
            assert!(
                !property.is_empty(),
                "block {} secondaryProperties cannot contain empty ids",
                definition.id
            );
            assert!(
                !definition.secondary_properties[..index].contains(property),
                "block {} secondaryProperties cannot contain duplicates",
                definition.id
            );
        }
        for (face, layers) in [
            ("top", &definition.textures.top),
            ("bottom", &definition.textures.bottom),
            ("left", &definition.textures.left),
            ("right", &definition.textures.right),
            ("front", &definition.textures.front),
            ("back", &definition.textures.back),
        ] {
            for (layer, texture) in layers.iter().enumerate() {
                assert!(
                    is_safe_relative_asset_path(&texture.texture),
                    "block {} textures.{face}[{layer}].texture must be a safe relative asset path: {}",
                    definition.id,
                    texture.texture
                );
            }
        }

        intern_block_id(&definition.id);
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&BlockDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &BlockDefinition> {
        self.definitions.values()
    }
}

pub(crate) struct BlockLookup<'a> {
    registry: &'a BlockRegistry,
    cache: SmallVec<[((usize, usize), &'a BlockDefinition); 16]>,
}

impl<'a> BlockLookup<'a> {
    pub(crate) fn new(registry: &'a BlockRegistry) -> Self {
        Self {
            registry,
            cache: SmallVec::new(),
        }
    }

    pub(crate) fn get(&mut self, id: &'static str) -> &'a BlockDefinition {
        let key = (id.as_ptr() as usize, id.len());
        if let Some((_, definition)) = self.cache.iter().find(|(candidate, _)| *candidate == key) {
            return definition;
        }

        let definition = self
            .registry
            .get(id)
            .unwrap_or_else(|| panic!("missing block definition: {id}"));
        self.cache.push((key, definition));
        definition
    }
}

fn default_mining_hardness() -> f32 {
    1.0
}

fn default_light_dampening() -> u8 {
    MAX_LIGHT_DAMPENING
}

fn default_casts_shadow() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn localized_name() -> LocalizedText {
        serde_json::from_str(
            r#"{"english":"Test","portuguese_brazil":"Teste","spanish":"Prueba"}"#,
        )
        .expect("test localization must parse")
    }

    #[test]
    fn blocks_without_orientation_list_use_default_y_orientation() {
        let mut registry = BlockRegistry::default();
        registry.insert(BlockDefinition {
            id: "asteria:test".to_owned(),
            name: localized_name(),
            category: "test".to_owned(),
            mining: BlockMiningDefinition::default(),
            tags: Vec::new(),
            tint: BlockTint::None,
            textures: BlockTextures::default(),
            rotate_texture: BlockTextureRotations::default(),
            orientations: Vec::new(),
            secondary_properties: Vec::new(),
            alpha_cutoff: None,
            alpha_blend: false,
            light_emission: 0,
            light_dampening: default_light_dampening(),
            casts_shadow: default_casts_shadow(),
        });

        let block = registry.get("asteria:test").expect("block must be registered");
        assert_eq!(block.default_orientation(), BlockOrientation::Y);
        assert!(!block.is_rotatable());
        assert_eq!(
            block.next_orientation(BlockOrientation::X),
            BlockOrientation::Y
        );
    }
}
