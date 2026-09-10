use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

use super::{block::BlockRegistry, block_orientation::BlockOrientation};

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureAnchor {
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub z: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructurePaletteEntry {
    pub block: String,
    #[serde(default)]
    pub orientation: BlockOrientation,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureLayer {
    pub y: i32,
    pub rows: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub anchor: StructureAnchor,
    pub palette: HashMap<String, StructurePaletteEntry>,
    pub layers: Vec<StructureLayer>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StructureVoxel<'a> {
    pub offset: IVec3,
    pub block_id: &'a str,
    pub orientation: BlockOrientation,
}

impl StructureDefinition {
    pub(crate) fn validate_references(&self, blocks: &BlockRegistry) {
        for entry in self.palette.values() {
            assert!(
                blocks.get(&entry.block).is_some(),
                "structure {} references missing block: {}",
                self.id,
                entry.block
            );
        }
    }

    pub(crate) fn voxels(&self) -> Vec<StructureVoxel<'_>> {
        let mut voxels = Vec::new();

        for layer in &self.layers {
            for (z, row) in layer.rows.iter().enumerate() {
                for (x, symbol) in row.chars().enumerate() {
                    if symbol == '.' {
                        continue;
                    }

                    let entry = self.palette_entry(symbol).unwrap_or_else(|| {
                        panic!(
                            "structure {} uses undefined palette symbol: {symbol}",
                            self.id
                        )
                    });
                    voxels.push(StructureVoxel {
                        offset: IVec3::new(
                            x as i32 - self.anchor.x,
                            layer.y - self.anchor.y,
                            z as i32 - self.anchor.z,
                        ),
                        block_id: entry.block.as_str(),
                        orientation: entry.orientation,
                    });
                }
            }
        }

        voxels
    }

    pub(crate) fn horizontal_bounds(&self) -> (IVec2, IVec2) {
        let mut minimum = IVec2::splat(i32::MAX);
        let mut maximum = IVec2::splat(i32::MIN);

        for voxel in self.voxels() {
            let horizontal = IVec2::new(voxel.offset.x, voxel.offset.z);
            minimum = minimum.min(horizontal);
            maximum = maximum.max(horizontal);
        }

        if minimum.x == i32::MAX {
            (IVec2::ZERO, IVec2::ZERO)
        } else {
            (minimum, maximum)
        }
    }

    pub(crate) fn max_y_offset(&self) -> i32 {
        self.voxels()
            .into_iter()
            .map(|voxel| voxel.offset.y)
            .max()
            .unwrap_or(0)
    }

    fn validate_layout(&self) {
        assert!(!self.id.trim().is_empty(), "structure id cannot be empty");
        assert!(
            !self.name.trim().is_empty(),
            "structure {} name cannot be empty",
            self.id
        );
        assert!(
            !self.palette.is_empty(),
            "structure {} palette cannot be empty",
            self.id
        );
        assert!(
            !self.layers.is_empty(),
            "structure {} must define at least one layer",
            self.id
        );

        for (symbol, entry) in &self.palette {
            assert!(
                symbol != "." && symbol.chars().count() == 1,
                "structure {} palette keys must be exactly one non-dot character",
                self.id
            );
            assert!(
                !entry.block.trim().is_empty(),
                "structure {} palette symbol {symbol} must reference a block",
                self.id
            );
        }

        let depth = self.layers[0].rows.len();
        assert!(depth > 0, "structure {} layers cannot be empty", self.id);
        let width = self.layers[0]
            .rows
            .first()
            .map(|row| row.chars().count())
            .unwrap_or(0);
        assert!(width > 0, "structure {} rows cannot be empty", self.id);
        assert!(
            self.anchor.x >= 0 && self.anchor.x < width as i32,
            "structure {} anchor.x must be inside the layer width",
            self.id
        );
        assert!(
            self.anchor.z >= 0 && self.anchor.z < depth as i32,
            "structure {} anchor.z must be inside the layer depth",
            self.id
        );

        let mut voxel_count = 0_usize;
        for (layer_index, layer) in self.layers.iter().enumerate() {
            assert!(
                self.layers[..layer_index]
                    .iter()
                    .all(|other| other.y != layer.y),
                "structure {} cannot define layer y={} more than once",
                self.id,
                layer.y
            );
            assert_eq!(
                layer.rows.len(),
                depth,
                "structure {} layers must all have the same depth",
                self.id
            );

            for row in &layer.rows {
                assert_eq!(
                    row.chars().count(),
                    width,
                    "structure {} rows must all have the same width",
                    self.id
                );

                for symbol in row.chars().filter(|symbol| *symbol != '.') {
                    assert!(
                        self.palette_entry(symbol).is_some(),
                        "structure {} uses undefined palette symbol: {symbol}",
                        self.id
                    );
                    voxel_count += 1;
                }
            }
        }

        assert!(
            voxel_count > 0,
            "structure {} must contain at least one voxel",
            self.id
        );
    }

    fn palette_entry(&self, symbol: char) -> Option<&StructurePaletteEntry> {
        let mut buffer = [0_u8; 4];
        self.palette.get(symbol.encode_utf8(&mut buffer))
    }
}

#[derive(Resource, Default)]
pub struct StructureRegistry {
    definitions: HashMap<String, StructureDefinition>,
}

impl StructureRegistry {
    pub fn insert(&mut self, definition: StructureDefinition) {
        definition.validate_layout();
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&StructureDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &StructureDefinition> {
        self.definitions.values()
    }

    pub(crate) fn max_height_above_anchor(&self) -> i32 {
        self.definitions
            .values()
            .map(StructureDefinition::max_y_offset)
            .max()
            .unwrap_or(0)
            .max(0)
    }
}
