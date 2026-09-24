use std::{collections::{HashMap, HashSet}, sync::Arc};

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

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) enum StructureRotation {
    #[default]
    Degrees0,
    Degrees90,
    Degrees180,
    Degrees270,
}

impl StructureRotation {
    pub(crate) const ALL: [Self; 4] = [
        Self::Degrees0,
        Self::Degrees90,
        Self::Degrees180,
        Self::Degrees270,
    ];

    pub(crate) fn from_hash(hash: u64) -> Self {
        match (hash >> 32) & 3 {
            1 => Self::Degrees90,
            2 => Self::Degrees180,
            3 => Self::Degrees270,
            _ => Self::Degrees0,
        }
    }

    pub(crate) fn inverse(self) -> Self {
        match self {
            Self::Degrees0 => Self::Degrees0,
            Self::Degrees90 => Self::Degrees270,
            Self::Degrees180 => Self::Degrees180,
            Self::Degrees270 => Self::Degrees90,
        }
    }

    pub(crate) fn rotate_horizontal(self, offset: IVec2) -> IVec2 {
        match self {
            Self::Degrees0 => offset,
            Self::Degrees90 => IVec2::new(-offset.y, offset.x),
            Self::Degrees180 => -offset,
            Self::Degrees270 => IVec2::new(offset.y, -offset.x),
        }
    }

    pub(crate) fn rotate_offset(self, offset: IVec3) -> IVec3 {
        let horizontal = self.rotate_horizontal(offset.xz());
        IVec3::new(horizontal.x, offset.y, horizontal.y)
    }

    pub(crate) fn rotate_orientation(self, orientation: BlockOrientation) -> BlockOrientation {
        match self {
            Self::Degrees0 | Self::Degrees180 => orientation,
            Self::Degrees90 | Self::Degrees270 => match orientation {
                BlockOrientation::X => BlockOrientation::Z,
                BlockOrientation::Z => BlockOrientation::X,
                BlockOrientation::Y => BlockOrientation::Y,
            },
        }
    }

    pub(crate) fn rotate_face(self, face: LayerFace) -> LayerFace {
        match self {
            Self::Degrees0 => face,
            Self::Degrees90 => match face {
                LayerFace::Right => LayerFace::Front,
                LayerFace::Front => LayerFace::Left,
                LayerFace::Left => LayerFace::Back,
                LayerFace::Back => LayerFace::Right,
                LayerFace::Top => LayerFace::Top,
                LayerFace::Bottom => LayerFace::Bottom,
            },
            Self::Degrees180 => match face {
                LayerFace::Right => LayerFace::Left,
                LayerFace::Left => LayerFace::Right,
                LayerFace::Front => LayerFace::Back,
                LayerFace::Back => LayerFace::Front,
                LayerFace::Top => LayerFace::Top,
                LayerFace::Bottom => LayerFace::Bottom,
            },
            Self::Degrees270 => match face {
                LayerFace::Right => LayerFace::Back,
                LayerFace::Back => LayerFace::Left,
                LayerFace::Left => LayerFace::Front,
                LayerFace::Front => LayerFace::Right,
                LayerFace::Top => LayerFace::Top,
                LayerFace::Bottom => LayerFace::Bottom,
            },
        }
    }
}

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
    #[serde(default)]
    pub block: Option<String>,
    #[serde(default)]
    pub fluid: Option<String>,
    #[serde(default)]
    pub clear: bool,
    #[serde(default)]
    pub layers_only: bool,
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
    #[serde(default)]
    pub rotation_group: Option<String>,
    #[serde(skip)]
    runtime_hash: u64,
    #[serde(skip)]
    runtime_rotation_hash: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureLayer {
    pub y: i32,
    pub rows: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct StructureRuntime {
    id_hash: u64,
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
    pub rotation: bool,
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
    #[serde(default)]
    pub ground_anchor_y: Option<i32>,
    #[serde(default)]
    pub clear_above: u32,
    pub palette: HashMap<String, StructurePaletteEntry>,
    pub layers: Vec<StructureLayer>,
    #[serde(skip)]
    runtime: StructureRuntime,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StructureVoxel {
    pub offset: IVec3,
    pub block_id: Option<&'static str>,
    pub orientation: BlockOrientation,
    palette_symbol: char,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StructureColumnSpan {
    pub offset: IVec2,
    pub min_y_offset: i32,
    pub max_y_offset: i32,
}

impl StructureSurfaceLayer {
    pub(crate) fn runtime_hash(&self) -> u64 {
        self.runtime_hash
    }

    pub(crate) fn runtime_rotation_hash(&self) -> u64 {
        self.runtime_rotation_hash
    }
}

impl StructureDefinition {
    pub(crate) fn runtime_hash(&self) -> u64 {
        self.runtime.id_hash
    }

    pub(crate) fn rotation_for_hash(&self, hash: u64) -> StructureRotation {
        if self.rotation {
            StructureRotation::from_hash(hash)
        } else {
            StructureRotation::Degrees0
        }
    }

    pub(crate) fn supported_rotations(&self) -> &'static [StructureRotation] {
        if self.rotation {
            &StructureRotation::ALL
        } else {
            &StructureRotation::ALL[..1]
        }
    }

    pub(crate) fn validate_references(
        &self,
        blocks: &BlockRegistry,
        layers: &LayerRegistry,
        fluids: &FluidRegistry,
    ) {
        self.restrictions
            .validate_references(&self.id, blocks, fluids);

        for entry in self.palette.values() {
            if let Some(block) = entry.block.as_deref() {
                assert!(
                    blocks.get(block).is_some(),
                    "structure {} references missing block: {}",
                    self.id,
                    block
                );
            }
            if let Some(fluid) = entry.fluid.as_deref() {
                assert!(
                    fluids.id_of(fluid).is_some(),
                    "structure {} references missing fluid: {}",
                    self.id,
                    fluid
                );
            }
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

    pub(crate) fn horizontal_bounds_for_rotation(
        &self,
        rotation: StructureRotation,
    ) -> (IVec2, IVec2) {
        let (minimum, maximum) = self.horizontal_bounds();
        let corners = [
            minimum,
            IVec2::new(maximum.x, minimum.y),
            maximum,
            IVec2::new(minimum.x, maximum.y),
        ];
        corners.into_iter().map(|corner| rotation.rotate_horizontal(corner)).fold(
            (IVec2::splat(i32::MAX), IVec2::splat(i32::MIN)),
            |(minimum, maximum), corner| (minimum.min(corner), maximum.max(corner)),
        )
    }

    pub(crate) fn effective_max_y_offset(&self) -> i32 {
        self.runtime.max_y_offset + self.clear_above as i32
    }

    pub(crate) fn min_y_offset(&self) -> i32 {
        self.runtime.min_y_offset
    }

    pub(crate) fn ground_anchor_y_offset(&self) -> i32 {
        self.ground_anchor_y
            .map(|ground_y| ground_y - self.anchor.y)
            .unwrap_or(self.runtime.min_y_offset)
    }

    pub(crate) fn support_offsets_for_rotation(
        &self,
        rotation: StructureRotation,
    ) -> Vec<IVec2> {
        self.runtime
            .support_offsets
            .iter()
            .map(|offset| rotation.rotate_horizontal(*offset))
            .collect()
    }

    pub(crate) fn horizontal_footprint_for_rotation(
        &self,
        rotation: StructureRotation,
    ) -> Vec<IVec2> {
        self.runtime
            .horizontal_footprint
            .iter()
            .map(|offset| rotation.rotate_horizontal(*offset))
            .collect()
    }

    pub(crate) fn column_spans(&self) -> &[StructureColumnSpan] {
        &self.runtime.column_spans
    }

    pub(crate) fn clear_above_positions(
        &self,
        rotation: StructureRotation,
        origin: IVec3,
    ) -> impl Iterator<Item = IVec3> + '_ {
        let clear_above = self.clear_above as i32;
        self.runtime.column_spans.iter().flat_map(move |span| {
            let horizontal = origin.xz() + rotation.rotate_horizontal(span.offset);
            (1..=clear_above).map(move |delta_y| {
                IVec3::new(
                    horizontal.x,
                    origin.y + span.max_y_offset + delta_y,
                    horizontal.y,
                )
            })
        })
    }

    pub(crate) fn column_voxels(&self, offset: IVec2) -> &[StructureVoxel] {
        self.runtime
            .column_voxels
            .get(&(offset.x, offset.y))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn fluid_for_voxel(&self, voxel: &StructureVoxel) -> Option<&str> {
        self.palette_entry(voxel.palette_symbol)
            .and_then(|entry| entry.fluid.as_deref())
    }

    pub(crate) fn clears_voxel(&self, voxel: &StructureVoxel) -> bool {
        self.palette_entry(voxel.palette_symbol)
            .is_some_and(|entry| entry.clear)
    }

    pub(crate) fn layers_only_voxel(&self, voxel: &StructureVoxel) -> bool {
        self.palette_entry(voxel.palette_symbol)
            .is_some_and(|entry| entry.layers_only)
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
        let id_hash = stable_structure_hash(&self.id);
        for entry in self.palette.values_mut() {
            for surface in &mut entry.surface_layers {
                surface.runtime_hash = stable_structure_hash(&surface.layer);
                surface.runtime_rotation_hash = stable_structure_hash(
                    surface
                        .rotation_group
                        .as_deref()
                        .unwrap_or(surface.layer.as_str()),
                );
            }
        }
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
                        block_id: entry.block.as_deref().map(intern_block_id),
                        orientation: entry.orientation,
                        palette_symbol: symbol,
                    });
                }
            }
        }

        if voxels.is_empty() {
            self.runtime = StructureRuntime {
                id_hash,
                ..Default::default()
            };
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
            id_hash,
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
        assert!(
            self.clear_above <= 64,
            "structure {} clearAbove must be <= 64",
            self.id
        );

        for (symbol, entry) in &self.palette {
            assert!(
                symbol != "." && symbol.chars().count() == 1,
                "structure {} palette keys must be exactly one non-dot character",
                self.id
            );
            let content_count = usize::from(entry.block.is_some())
                + usize::from(entry.fluid.is_some())
                + usize::from(entry.clear)
                + usize::from(entry.layers_only);
            assert_eq!(
                content_count, 1,
                "structure {} palette symbol {symbol} must define exactly one of block, fluid, clear, or layersOnly",
                self.id
            );
            if let Some(block) = entry.block.as_deref() {
                assert!(
                    !block.trim().is_empty(),
                    "structure {} palette symbol {symbol} block cannot be empty",
                    self.id
                );
            }
            if let Some(fluid) = entry.fluid.as_deref() {
                assert!(
                    !fluid.trim().is_empty(),
                    "structure {} palette symbol {symbol} fluid cannot be empty",
                    self.id
                );
                assert!(
                    entry.surface_layers.is_empty(),
                    "structure {} palette symbol {symbol} fluid entries cannot define surfaceLayers",
                    self.id
                );
            }
            if entry.clear {
                assert!(
                    entry.surface_layers.is_empty(),
                    "structure {} palette symbol {symbol} clear entries cannot define surfaceLayers",
                    self.id
                );
            }
            if entry.layers_only {
                assert!(
                    !entry.surface_layers.is_empty(),
                    "structure {} palette symbol {symbol} layersOnly entries must define surfaceLayers",
                    self.id
                );
            }

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
                if let Some(rotation_group) = surface.rotation_group.as_deref() {
                    assert!(
                        !rotation_group.trim().is_empty(),
                        "structure {} palette symbol {symbol} surfaceLayers[{surface_index}].rotationGroup cannot be empty",
                        self.id
                    );
                }
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
    groups: Arc<HashMap<String, Vec<String>>>,
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
            let members = Arc::make_mut(&mut self.groups)
                .entry(reference)
                .or_default();
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

    pub(crate) fn references_overlap(&self, left: &str, right: &str) -> bool {
        if left == right {
            return self.resolves_reference(left);
        }
        if let Some(structure) = self.get(left) {
            return self.reference_contains_structure(right, &structure.id);
        }
        if let Some(structure) = self.get(right) {
            return self.reference_contains_structure(left, &structure.id);
        }

        let Some(left_members) = self.groups.get(left) else {
            return false;
        };
        let Some(right_members) = self.groups.get(right) else {
            return false;
        };
        left_members
            .iter()
            .any(|member| right_members.binary_search(member).is_ok())
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

    pub(crate) fn variation_count(&self, reference: &str) -> Option<usize> {
        if self.get(reference).is_some() {
            return Some(1);
        }

        self.groups.get(reference).map(Vec::len)
    }

    pub(crate) fn variation(
        &self,
        reference: &str,
        variation: usize,
    ) -> Option<&StructureDefinition> {
        let index = variation.checked_sub(1)?;
        if let Some(structure) = self.get(reference) {
            return (index == 0).then_some(structure);
        }

        let members = self.groups.get(reference)?;
        self.get(members.get(index)?)
    }

    pub(crate) fn select_for_manual_placement(
        &self,
        reference: &str,
        variation: Option<usize>,
        hash: u64,
    ) -> Option<&StructureDefinition> {
        match variation {
            Some(variation) => self.variation(reference, variation),
            None => self.select_for_reference(reference, hash),
        }
    }

    pub(crate) fn group_references(
        &self,
    ) -> impl Iterator<Item = (&str, &StructureDefinition, usize)> {
        self.groups.iter().filter_map(|(reference, members)| {
            let first = members.first()?;
            let structure = self.get(first)?;
            Some((reference.as_str(), structure, members.len()))
        })
    }

    pub(crate) fn bounds_for_reference(
        &self,
        reference: &str,
    ) -> Option<(IVec2, IVec2)> {
        let include_structure = |structure: &StructureDefinition,
                                 bounds: &mut Option<(IVec2, IVec2)>| {
            for &rotation in structure.supported_rotations() {
                let candidate = structure.horizontal_bounds_for_rotation(rotation);
                *bounds = Some(match *bounds {
                    Some((minimum, maximum)) => {
                        (minimum.min(candidate.0), maximum.max(candidate.1))
                    }
                    None => candidate,
                });
            }
        };

        let mut bounds = None;
        if let Some(structure) = self.get(reference) {
            include_structure(structure, &mut bounds);
            return bounds;
        }

        let members = self.groups.get(reference)?;
        for id in members {
            let structure = self
                .get(id)
                .expect("group index references registered structures");
            include_structure(structure, &mut bounds);
        }
        bounds
    }

}

fn stable_structure_hash(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn default_surface_layer_chance() -> f32 {
    1.0
}
