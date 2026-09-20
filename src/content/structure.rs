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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureVariantDefinition {
    pub id: String,
    #[serde(default = "default_variant_weight")]
    pub weight: f32,
    #[serde(default)]
    pub anchor: StructureAnchor,
    #[serde(default)]
    pub palette: HashMap<String, StructurePaletteEntry>,
    #[serde(default)]
    pub layers: Vec<StructureLayer>,
    #[serde(default)]
    pub variants: Vec<StructureVariantDefinition>,
    #[serde(skip)]
    runtime: StructureRuntime,
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

        if self.variants.is_empty() {
            validate_palette_references(&self.id, &self.palette, blocks);
        } else {
            for variant in &self.variants {
                validate_palette_references(
                    &format!("{} variant {}", self.id, variant.id),
                    &variant.palette,
                    blocks,
                );
            }
        }
    }

    pub(crate) fn variant_index_for_hash(&self, hash: u64) -> usize {
        if self.variants.is_empty() {
            return 0;
        }

        let total_weight = self
            .variants
            .iter()
            .map(|variant| f64::from(variant.weight))
            .sum::<f64>();
        let unit = (hash >> 11) as f64 * (1.0 / (1_u64 << 53) as f64);
        let mut target = unit * total_weight;

        for (index, variant) in self.variants.iter().enumerate() {
            let weight = f64::from(variant.weight);
            if target < weight {
                return index;
            }
            target -= weight;
        }

        self.variants.len() - 1
    }

    pub(crate) fn variant_id(&self, variant_index: usize) -> &str {
        if self.variants.is_empty() {
            assert_eq!(variant_index, 0, "base structure has only variant index 0");
            &self.id
        } else {
            &self.variants[variant_index].id
        }
    }

    pub(crate) fn variant_voxels(&self, variant_index: usize) -> &[StructureVoxel] {
        &self.variant_runtime(variant_index).voxels
    }

    pub(crate) fn horizontal_bounds(&self) -> (IVec2, IVec2) {
        if self.variants.is_empty() {
            return (
                self.runtime.horizontal_minimum,
                self.runtime.horizontal_maximum,
            );
        }

        let mut minimum = IVec2::splat(i32::MAX);
        let mut maximum = IVec2::splat(i32::MIN);
        for variant in &self.variants {
            minimum = minimum.min(variant.runtime.horizontal_minimum);
            maximum = maximum.max(variant.runtime.horizontal_maximum);
        }
        (minimum, maximum)
    }

    pub(crate) fn variant_horizontal_bounds(
        &self,
        variant_index: usize,
    ) -> (IVec2, IVec2) {
        let runtime = self.variant_runtime(variant_index);
        (runtime.horizontal_minimum, runtime.horizontal_maximum)
    }

    pub(crate) fn max_y_offset(&self) -> i32 {
        if self.variants.is_empty() {
            return self.runtime.max_y_offset;
        }

        self.variants
            .iter()
            .map(|variant| variant.runtime.max_y_offset)
            .max()
            .expect("validated structure variants cannot be empty")
    }

    pub(crate) fn variant_min_y_offset(&self, variant_index: usize) -> i32 {
        self.variant_runtime(variant_index).min_y_offset
    }

    pub(crate) fn variant_horizontal_footprint(
        &self,
        variant_index: usize,
    ) -> &[IVec2] {
        &self.variant_runtime(variant_index).horizontal_footprint
    }

    pub(crate) fn variant_support_offsets(
        &self,
        variant_index: usize,
    ) -> &[IVec2] {
        &self.variant_runtime(variant_index).support_offsets
    }

    pub(crate) fn variant_column_spans(
        &self,
        variant_index: usize,
    ) -> &[StructureColumnSpan] {
        &self.variant_runtime(variant_index).column_spans
    }

    pub(crate) fn variant_column_voxels(
        &self,
        variant_index: usize,
        offset: IVec2,
    ) -> &[StructureVoxel] {
        self.variant_runtime(variant_index)
            .column_voxels
            .get(&(offset.x, offset.y))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn variant_runtime(&self, variant_index: usize) -> &StructureRuntime {
        if self.variants.is_empty() {
            assert_eq!(variant_index, 0, "base structure has only variant index 0");
            &self.runtime
        } else {
            &self.variants[variant_index].runtime
        }
    }

    fn rebuild_runtime(&mut self) {
        if self.variants.is_empty() {
            self.runtime = build_runtime(
                &self.id,
                self.anchor,
                &self.palette,
                &self.layers,
            );
            return;
        }

        self.runtime = StructureRuntime::default();
        for variant in &mut self.variants {
            variant.runtime = build_runtime(
                &format!("{} variant {}", self.id, variant.id),
                variant.anchor,
                &variant.palette,
                &variant.layers,
            );
        }
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

        let has_base_layout = !self.palette.is_empty() || !self.layers.is_empty();
        assert!(
            has_base_layout != !self.variants.is_empty(),
            "structure {} must define either one base layout or variants, but not both",
            self.id
        );

        if has_base_layout {
            validate_layout_definition(
                &self.id,
                self.anchor,
                &self.palette,
                &self.layers,
            );
            return;
        }

        for (index, variant) in self.variants.iter().enumerate() {
            assert!(
                !variant.id.trim().is_empty(),
                "structure {} variants cannot have empty ids",
                self.id
            );
            assert!(
                self.variants[..index]
                    .iter()
                    .all(|other| other.id != variant.id),
                "structure {} variant id {} is duplicated",
                self.id,
                variant.id
            );
            assert!(
                variant.weight.is_finite() && variant.weight > 0.0,
                "structure {} variant {} weight must be finite and positive",
                self.id,
                variant.id
            );
            validate_layout_definition(
                &format!("{} variant {}", self.id, variant.id),
                variant.anchor,
                &variant.palette,
                &variant.layers,
            );
        }
    }
}

fn default_variant_weight() -> f32 {
    1.0
}

fn validate_palette_references(
    label: &str,
    palette: &HashMap<String, StructurePaletteEntry>,
    blocks: &BlockRegistry,
) {
    for entry in palette.values() {
        assert!(
            blocks.get(&entry.block).is_some(),
            "{label} references missing block: {}",
            entry.block
        );
    }
}

fn validate_layout_definition(
    label: &str,
    anchor: StructureAnchor,
    palette: &HashMap<String, StructurePaletteEntry>,
    layers: &[StructureLayer],
) {
    assert!(!palette.is_empty(), "{label} palette cannot be empty");
    assert!(!layers.is_empty(), "{label} must define at least one layer");

    for (symbol, entry) in palette {
        assert!(
            symbol != "." && symbol.chars().count() == 1,
            "{label} palette keys must be exactly one non-dot character"
        );
        assert!(
            !entry.block.trim().is_empty(),
            "{label} palette symbol {symbol} must reference a block"
        );
    }

    let depth = layers[0].rows.len();
    assert!(depth > 0, "{label} layers cannot be empty");
    let width = layers[0]
        .rows
        .first()
        .map(|row| row.chars().count())
        .unwrap_or(0);
    assert!(width > 0, "{label} rows cannot be empty");
    assert!(
        anchor.x >= 0 && anchor.x < width as i32,
        "{label} anchor.x must be inside the layer width"
    );
    assert!(
        anchor.z >= 0 && anchor.z < depth as i32,
        "{label} anchor.z must be inside the layer depth"
    );

    let mut voxel_count = 0_usize;
    for (layer_index, layer) in layers.iter().enumerate() {
        assert!(
            layers[..layer_index]
                .iter()
                .all(|other| other.y != layer.y),
            "{label} cannot define layer y={} more than once",
            layer.y
        );
        assert_eq!(
            layer.rows.len(),
            depth,
            "{label} layers must all have the same depth"
        );

        for row in &layer.rows {
            assert_eq!(
                row.chars().count(),
                width,
                "{label} rows must all have the same width"
            );

            for symbol in row.chars().filter(|symbol| *symbol != '.') {
                assert!(
                    palette_entry(palette, symbol).is_some(),
                    "{label} uses undefined palette symbol: {symbol}"
                );
                voxel_count += 1;
            }
        }
    }

    assert!(
        voxel_count > 0,
        "{label} must contain at least one voxel"
    );
}

fn build_runtime(
    label: &str,
    anchor: StructureAnchor,
    palette: &HashMap<String, StructurePaletteEntry>,
    layers: &[StructureLayer],
) -> StructureRuntime {
    let mut voxels = Vec::new();
    let mut horizontal_minimum = IVec2::splat(i32::MAX);
    let mut horizontal_maximum = IVec2::splat(i32::MIN);
    let mut max_y_offset = i32::MIN;

    for layer in layers {
        for (z, row) in layer.rows.iter().enumerate() {
            for (x, symbol) in row.chars().enumerate() {
                if symbol == '.' {
                    continue;
                }

                let entry = palette_entry(palette, symbol)
                    .unwrap_or_else(|| panic!("{label} uses undefined palette symbol: {symbol}"));
                let offset = IVec3::new(
                    x as i32 - anchor.x,
                    layer.y - anchor.y,
                    z as i32 - anchor.z,
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
        return StructureRuntime::default();
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

    StructureRuntime {
        voxels,
        horizontal_minimum,
        horizontal_maximum,
        horizontal_footprint,
        support_offsets,
        column_spans,
        column_voxels,
        min_y_offset,
        max_y_offset,
    }
}

fn palette_entry<'a>(
    palette: &'a HashMap<String, StructurePaletteEntry>,
    symbol: char,
) -> Option<&'a StructurePaletteEntry> {
    let mut buffer = [0_u8; 4];
    palette.get(symbol.encode_utf8(&mut buffer))
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
