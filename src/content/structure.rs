use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{
    block::BlockRegistry, block_id::intern_block_id, block_orientation::BlockOrientation,
    registry::DefinitionMap,
    structure_rules::{StructureGenerationRules, StructureRestrictions},
};

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

#[derive(Clone, Debug, Default)]
struct StructureRuntime {
    voxels: Vec<StructureVoxel>,
    horizontal_minimum: IVec2,
    horizontal_maximum: IVec2,
    horizontal_footprint: Vec<IVec2>,
    support_offsets: Vec<IVec2>,
    column_spans: Vec<StructureColumnSpan>,
    column_voxels: HashMap<(i32, i32), Vec<StructureVoxel>>,
    min_y_offset: i32,
    max_y_offset: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureDefinition {
    pub id: String,
    pub name: LocalizedText,
    pub locatable: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub conflict_groups: Vec<String>,
    #[serde(default)]
    pub restrictions: StructureRestrictions,
    #[serde(default)]
    pub generation: StructureGenerationRules,
    #[serde(default)]
    pub anchor: StructureAnchor,
    pub palette: HashMap<String, StructurePaletteEntry>,
    pub layers: Vec<StructureLayer>,
    #[serde(skip)]
    runtime: StructureRuntime,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StructureVoxel {
    pub offset: IVec3,
    pub block_id: &'static str,
    pub orientation: BlockOrientation,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StructureColumnSpan {
    pub offset: IVec2,
    pub min_y_offset: i32,
    pub max_y_offset: i32,
}

impl StructureDefinition {
    pub(crate) fn validate_references(&self, blocks: &BlockRegistry) {
        self.restrictions.validate_references(&self.id, blocks);

        for entry in self.palette.values() {
            assert!(
                blocks.get(&entry.block).is_some(),
                "structure {} references missing block: {}",
                self.id,
                entry.block
            );
        }
    }

    pub(crate) fn voxels(&self) -> &[StructureVoxel] {
        &self.runtime.voxels
    }

    pub(crate) fn horizontal_bounds(&self) -> (IVec2, IVec2) {
        (
            self.runtime.horizontal_minimum,
            self.runtime.horizontal_maximum,
        )
    }

    pub(crate) fn max_y_offset(&self) -> i32 {
        self.runtime.max_y_offset
    }

    pub(crate) fn min_y_offset(&self) -> i32 {
        self.runtime.min_y_offset
    }

    pub(crate) fn horizontal_footprint(&self) -> &[IVec2] {
        &self.runtime.horizontal_footprint
    }

    pub(crate) fn support_offsets(&self) -> &[IVec2] {
        &self.runtime.support_offsets
    }

    pub(crate) fn column_spans(&self) -> &[StructureColumnSpan] {
        &self.runtime.column_spans
    }

    pub(crate) fn column_voxels(&self, offset: IVec2) -> &[StructureVoxel] {
        self.runtime
            .column_voxels
            .get(&(offset.x, offset.y))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn rebuild_runtime(&mut self) {
        let mut voxels = Vec::new();
        let mut horizontal_minimum = IVec2::splat(i32::MAX);
        let mut horizontal_maximum = IVec2::splat(i32::MIN);
        let mut max_y_offset = i32::MIN;

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
                    let offset = IVec3::new(
                        x as i32 - self.anchor.x,
                        layer.y - self.anchor.y,
                        z as i32 - self.anchor.z,
                    );
                    let horizontal = IVec2::new(offset.x, offset.z);
                    horizontal_minimum = horizontal_minimum.min(horizontal);
                    horizontal_maximum = horizontal_maximum.max(horizontal);
                    max_y_offset = max_y_offset.max(offset.y);
                    voxels.push(StructureVoxel {
                        offset,
                        block_id: intern_block_id(&entry.block),
                        orientation: entry.orientation,
                    });
                }
            }
        }

        if voxels.is_empty() {
            self.runtime = StructureRuntime::default();
            return;
        }

        let min_y_offset = voxels
            .iter()
            .map(|voxel| voxel.offset.y)
            .min()
            .expect("non-empty structure must have a minimum y offset");
        let mut footprint = HashSet::new();
        let mut supports = HashSet::new();
        let mut spans = HashMap::<(i32, i32), (i32, i32)>::new();
        let mut column_voxels = HashMap::<(i32, i32), Vec<StructureVoxel>>::new();

        for voxel in &voxels {
            let horizontal = (voxel.offset.x, voxel.offset.z);
            footprint.insert(horizontal);
            column_voxels.entry(horizontal).or_default().push(*voxel);
            if voxel.offset.y == min_y_offset {
                supports.insert(horizontal);
            }
            spans
                .entry(horizontal)
                .and_modify(|span| {
                    span.0 = span.0.min(voxel.offset.y);
                    span.1 = span.1.max(voxel.offset.y);
                })
                .or_insert((voxel.offset.y, voxel.offset.y));
        }

        let mut horizontal_footprint = footprint
            .into_iter()
            .map(|(x, z)| IVec2::new(x, z))
            .collect::<Vec<_>>();
        horizontal_footprint.sort_by_key(|offset| (offset.y, offset.x));

        let mut support_offsets = supports
            .into_iter()
            .map(|(x, z)| IVec2::new(x, z))
            .collect::<Vec<_>>();
        support_offsets.sort_by_key(|offset| (offset.y, offset.x));

        for column in column_voxels.values_mut() {
            column.sort_unstable_by_key(|voxel| voxel.offset.y);
        }

        let mut column_spans = spans
            .into_iter()
            .map(|((x, z), (min_y_offset, max_y_offset))| StructureColumnSpan {
                offset: IVec2::new(x, z),
                min_y_offset,
                max_y_offset,
            })
            .collect::<Vec<_>>();
        column_spans.sort_by_key(|span| (span.offset.y, span.offset.x));

        self.runtime = StructureRuntime {
            voxels,
            horizontal_minimum,
            horizontal_maximum,
            horizontal_footprint,
            support_offsets,
            column_spans,
            column_voxels,
            min_y_offset,
            max_y_offset,
        };
    }

    fn validate_layout(&self) {
        assert!(!self.id.trim().is_empty(), "structure id cannot be empty");
        self.name.validate(&format!("structure {} name", self.id));
        self.restrictions.validate(&self.id);
        for (index, group) in self.conflict_groups.iter().enumerate() {
            assert!(
                !group.trim().is_empty(),
                "structure {} conflictGroups cannot contain empty values",
                self.id
            );
            assert!(
                !self.conflict_groups[..index].contains(group),
                "structure {} conflictGroups cannot contain duplicates",
                self.id
            );
        }
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

#[derive(Clone, Resource, Default)]
pub struct StructureRegistry {
    definitions: DefinitionMap<StructureDefinition>,
}

impl StructureRegistry {
    pub fn insert(&mut self, mut definition: StructureDefinition) {
        definition.validate_layout();
        definition.rebuild_runtime();
        self.definitions.insert(definition.id.clone(), definition);
    }

    pub fn get(&self, id: &str) -> Option<&StructureDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &StructureDefinition> {
        self.definitions.values()
    }

}
