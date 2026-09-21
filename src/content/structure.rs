use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use serde::Deserialize;

use crate::localization::LocalizedText;

use super::{
    block::BlockRegistry, block_id::intern_block_id, block_orientation::BlockOrientation,
    fluid::FluidRegistry,
    layer::{LayerFace, LayerRegistry},
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
    #[serde(default)]
    pub surface_layers: Vec<StructureSurfaceLayer>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureSurfaceLayer {
    pub layer: String,
    pub faces: Vec<LayerFace>,
    #[serde(default = "default_surface_layer_chance")]
    pub chance: f32,
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
    #[serde(default, rename = "group_id")]
    pub group_id: Option<String>,
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
    palette_symbol: char,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StructureColumnSpan {
    pub offset: IVec2,
    pub min_y_offset: i32,
    pub max_y_offset: i32,
}

impl StructureDefinition {
    pub(crate) fn validate_references(
        &self,
        blocks: &BlockRegistry,
        layers: &LayerRegistry,
        fluids: &FluidRegistry,
    ) {
        self.restrictions
            .validate_references(&self.id, blocks, fluids);

        for entry in self.palette.values() {
            assert!(
                blocks.get(&entry.block).is_some(),
                "structure {} references missing block: {}",
                self.id,
                entry.block
            );
            for surface in &entry.surface_layers {
                let definition = layers.get(&surface.layer).unwrap_or_else(|| {
                    panic!(
                        "structure {} references missing surface layer: {}",
                        self.id, surface.layer
                    )
                });
                for &face in &surface.faces {
                    assert!(
                        definition.supports_face(face),
                        "structure {} surface layer {} does not support face {:?}",
                        self.id,
                        surface.layer,
                        face
                    );
                }
            }
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

    pub(crate) fn surface_layers_for_voxel(
        &self,
        voxel: &StructureVoxel,
    ) -> &[StructureSurfaceLayer] {
        self.palette_entry(voxel.palette_symbol)
            .expect("runtime structure voxel must retain a valid palette symbol")
            .surface_layers
            .as_slice()
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
                        palette_symbol: symbol,
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
        if let Some(group_id) = self.group_id.as_deref() {
            assert!(
                !group_id.trim().is_empty(),
                "structure {} group_id cannot be empty",
                self.id
            );
        }
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

            for (surface_index, surface) in entry.surface_layers.iter().enumerate() {
                assert!(
                    !surface.layer.trim().is_empty(),
                    "structure {} palette symbol {symbol} surfaceLayers[{surface_index}] must reference a layer",
                    self.id
                );
                assert!(
                    surface.chance.is_finite() && (0.0..=1.0).contains(&surface.chance),
                    "structure {} palette symbol {symbol} surfaceLayers[{surface_index}].chance must be between 0 and 1",
                    self.id
                );
                assert!(
                    !surface.faces.is_empty(),
                    "structure {} palette symbol {symbol} surfaceLayers[{surface_index}] must define at least one face",
                    self.id
                );
                for (face_index, face) in surface.faces.iter().enumerate() {
                    assert!(
                        !surface.faces[..face_index].contains(face),
                        "structure {} palette symbol {symbol} surfaceLayers[{surface_index}].faces cannot contain duplicates",
                        self.id
                    );
                }
                for previous in &entry.surface_layers[..surface_index] {
                    assert!(
                        previous.layer != surface.layer
                            || !previous.faces.iter().any(|face| surface.faces.contains(face)),
                        "structure {} palette symbol {symbol} cannot define the same surface layer on the same face more than once",
                        self.id
                    );
                }
            }
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
    groups: HashMap<String, Vec<String>>,
}

impl StructureRegistry {
    pub fn insert(&mut self, mut definition: StructureDefinition) {
        definition.validate_layout();
        definition.rebuild_runtime();

        let structure_id = definition.id.clone();
        let group_reference = definition.group_id.as_deref().map(|group_id| {
            structure_id.split_once(':').map_or_else(
                || group_id.to_owned(),
                |(namespace, _)| format!("{namespace}:{group_id}"),
            )
        });

        self.definitions.insert(structure_id.clone(), definition);
        if let Some(reference) = group_reference {
            let members = self.groups.entry(reference).or_default();
            let index = members
                .binary_search(&structure_id)
                .unwrap_or_else(|index| index);
            members.insert(index, structure_id);
        }
    }

    pub fn get(&self, id: &str) -> Option<&StructureDefinition> {
        self.definitions.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &StructureDefinition> {
        self.definitions.values()
    }

    pub(crate) fn resolves_reference(&self, reference: &str) -> bool {
        self.get(reference).is_some() || self.groups.contains_key(reference)
    }

    pub(crate) fn reference_contains_structure(
        &self,
        reference: &str,
        structure_id: &str,
    ) -> bool {
        self.get(reference).is_some_and(|structure| structure.id == structure_id)
            || self
                .groups
                .get(reference)
                .is_some_and(|members| members.iter().any(|member| member == structure_id))
    }

    pub(crate) fn select_for_reference(
        &self,
        reference: &str,
        hash: u64,
    ) -> Option<&StructureDefinition> {
        if let Some(structure) = self.get(reference) {
            return Some(structure);
        }

        let members = self.groups.get(reference)?;
        let id = &members[(hash as usize) % members.len()];
        self.get(id)
    }

    pub(crate) fn bounds_for_reference(
        &self,
        reference: &str,
    ) -> Option<(IVec2, IVec2)> {
        if let Some(structure) = self.get(reference) {
            return Some(structure.horizontal_bounds());
        }

        let members = self.groups.get(reference)?;
        let first = self
            .get(members.first()?)
            .expect("group index references registered structures");
        let (mut minimum, mut maximum) = first.horizontal_bounds();
        for id in members.iter().skip(1) {
            let structure = self
                .get(id)
                .expect("group index references registered structures");
            let (candidate_minimum, candidate_maximum) = structure.horizontal_bounds();
            minimum = minimum.min(candidate_minimum);
            maximum = maximum.max(candidate_maximum);
        }
        Some((minimum, maximum))
    }

}

fn default_surface_layer_chance() -> f32 {
    1.0
}
