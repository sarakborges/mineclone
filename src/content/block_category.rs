use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BlockCategory {
    WoodenBlocks,
    Foliage,
    NaturalBlocks,
    LightSources,
    CraftedBlocks,
}
