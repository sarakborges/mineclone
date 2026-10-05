use std::{collections::BTreeSet, fs, path::Path};

use bevy::prelude::{IVec2, IVec3};
use serde::Serialize;

use crate::content::structure::StructureRotation;

use super::structure::{StructurePlacement, StructureQueries};

const DEFAULT_SEARCH_RADIUS: u32 = 8_192;
const DEFAULT_WINDOW_SIZE: u32 = 512;

#[derive(Clone, Debug)]
pub(crate) struct StructureDebugConfig {
    pub(crate) center_x: i32,
    pub(crate) center_z: i32,
    pub(crate) search_radius: u32,
    pub(crate) window_size: u32,
}

impl Default for StructureDebugConfig {
    fn default() -> Self {
        Self {
            center_x: 0,
            center_z: 0,
            search_radius: DEFAULT_SEARCH_RADIUS,
            window_size: DEFAULT_WINDOW_SIZE,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StructureDebugReport {
    center: [i32; 2],
    search_radius: u32,
    window_size: u32,
    probes: Vec<StructureProbeReport>,
}

impl StructureDebugReport {
    pub(crate) fn validation_passed(&self) -> bool {
        self.probes.iter().all(StructureProbeReport::validation_passed)
    }

    pub(crate) fn save(&self, path: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| format!("failed to encode Structure debug report: {error}"))?;
        fs::write(path, bytes)
            .map_err(|error| format!("failed to save {}: {error}", path.display()))
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StructureProbeReport {
    reference: String,
    found: bool,
    placement_anchor: Option<[i32; 2]>,
    representative_structure: Option<String>,
    expected_families: Vec<String>,
    observed_families: Vec<String>,
    expected_families_present: bool,
    x_axis: Option<AxisValidationReport>,
    z_axis: Option<AxisValidationReport>,
}

impl StructureProbeReport {
    fn validation_passed(&self) -> bool {
        self.found
            && self.expected_families_present
            && self
                .x_axis
                .as_ref()
                .is_some_and(AxisValidationReport::validation_passed)
            && self
                .z_axis
                .as_ref()
                .is_some_and(AxisValidationReport::validation_passed)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AxisValidationReport {
    first_query_count: usize,
    second_query_count: usize,
    overlap_count: usize,
    first_repeat_match: bool,
    second_repeat_match: bool,
    seam_match: bool,
}

impl AxisValidationReport {
    fn validation_passed(&self) -> bool {
        self.first_repeat_match && self.second_repeat_match && self.seam_match
    }
}

#[derive(Clone, Copy)]
struct ProbeDefinition {
    reference: &'static str,
    expected_families: &'static [&'static str],
}

const PROBES: [ProbeDefinition; 5] = [
    ProbeDefinition {
        reference: "asteria:tree_oak",
        expected_families: &[],
    },
    ProbeDefinition {
        reference: "asteria:enchanted_heart",
        expected_families: &["world_tree"],
    },
    ProbeDefinition {
        reference: "asteria:river_ocean_mouth",
        expected_families: &["river_segment"],
    },
    ProbeDefinition {
        reference: "asteria:lake",
        expected_families: &["river_segment"],
    },
    ProbeDefinition {
        reference: "asteria:mountain_waterfall",
        expected_families: &["mountain_pond", "river_segment"],
    },
];

pub(crate) fn validate_structure_debug(
    structures: StructureQueries<'_>,
    config: &StructureDebugConfig,
) -> StructureDebugReport {
    assert!(config.search_radius > 0, "Structure debug search radius must be positive");
    assert!(
        config.window_size >= 16 && config.window_size % 4 == 0,
        "Structure debug window size must be at least 16 and divisible by four"
    );

    let probes = PROBES
        .iter()
        .map(|probe| validate_probe(structures, config, *probe))
        .collect();

    StructureDebugReport {
        center: [config.center_x, config.center_z],
        search_radius: config.search_radius,
        window_size: config.window_size,
        probes,
    }
}

fn validate_probe(
    structures: StructureQueries<'_>,
    config: &StructureDebugConfig,
    probe: ProbeDefinition,
) -> StructureProbeReport {
    let Some(representative) = structures.find_nearest(
        probe.reference,
        config.center_x,
        config.center_z,
        config.search_radius,
    ) else {
        return StructureProbeReport {
            reference: probe.reference.to_owned(),
            found: false,
            placement_anchor: None,
            representative_structure: None,
            expected_families: probe
                .expected_families
                .iter()
                .map(|family| (*family).to_owned())
                .collect(),
            observed_families: Vec::new(),
            expected_families_present: false,
            x_axis: None,
            z_axis: None,
        };
    };

    let anchor = representative.placement_anchor();
    let (minimum, maximum) = representative.horizontal_bounds();
    let seam_x = midpoint(minimum.x, maximum.x);
    let seam_z = midpoint(minimum.y, maximum.y);
    let x_axis = validate_axis(
        structures,
        QueryAxis::X,
        IVec2::new(seam_x, anchor.y),
        config.window_size,
    );
    let z_axis = validate_axis(
        structures,
        QueryAxis::Z,
        IVec2::new(anchor.x, seam_z),
        config.window_size,
    );

    let mut observed_families = BTreeSet::new();
    collect_families_for_root(
        structures,
        anchor,
        probe.reference,
        config.window_size,
        &mut observed_families,
    );
    let expected_families_present = probe
        .expected_families
        .iter()
        .all(|family| observed_families.contains(*family));

    StructureProbeReport {
        reference: probe.reference.to_owned(),
        found: true,
        placement_anchor: Some([anchor.x, anchor.y]),
        representative_structure: Some(representative.structure_id().to_owned()),
        expected_families: probe
            .expected_families
            .iter()
            .map(|family| (*family).to_owned())
            .collect(),
        observed_families: observed_families.into_iter().collect(),
        expected_families_present,
        x_axis: Some(x_axis),
        z_axis: Some(z_axis),
    }
}

#[derive(Clone, Copy)]
enum QueryAxis {
    X,
    Z,
}

fn validate_axis(
    structures: StructureQueries<'_>,
    axis: QueryAxis,
    seam: IVec2,
    window_size: u32,
) -> AxisValidationReport {
    let quarter = i32::try_from(window_size / 4).expect("validated debug window must fit i32");
    let size = i32::try_from(window_size).expect("validated debug window must fit i32");
    let overlap_size = window_size / 2;
    let cross_origin = match axis {
        QueryAxis::X => IVec2::new(seam.x.saturating_sub(quarter * 3), seam.y - size / 2),
        QueryAxis::Z => IVec2::new(seam.x - size / 2, seam.y.saturating_sub(quarter * 3)),
    };
    let second_origin = match axis {
        QueryAxis::X => IVec2::new(seam.x.saturating_sub(quarter), seam.y - size / 2),
        QueryAxis::Z => IVec2::new(seam.x - size / 2, seam.y.saturating_sub(quarter)),
    };
    let overlap = match axis {
        QueryAxis::X => QueryRect::new(second_origin.x, cross_origin.y, overlap_size, window_size),
        QueryAxis::Z => QueryRect::new(cross_origin.x, second_origin.y, window_size, overlap_size),
    };
    let first_rect = QueryRect::new(cross_origin.x, cross_origin.y, window_size, window_size);
    let second_rect = QueryRect::new(second_origin.x, second_origin.y, window_size, window_size);

    let first = query_fingerprints(structures, first_rect);
    let second = query_fingerprints(structures, second_rect);
    let second_repeat = query_fingerprints(structures, second_rect);
    let first_repeat = query_fingerprints(structures, first_rect);
    let first_overlap = fingerprints_intersecting(&first, overlap);
    let second_overlap = fingerprints_intersecting(&second, overlap);

    AxisValidationReport {
        first_query_count: first.len(),
        second_query_count: second.len(),
        overlap_count: first_overlap.len(),
        first_repeat_match: first == first_repeat,
        second_repeat_match: second == second_repeat,
        seam_match: first_overlap == second_overlap,
    }
}

fn collect_families_for_root(
    structures: StructureQueries<'_>,
    anchor: IVec2,
    reference: &str,
    window_size: u32,
    families: &mut BTreeSet<String>,
) {
    let size = i32::try_from(window_size).expect("validated debug window must fit i32");
    let origin = IVec2::new(anchor.x - size / 2, anchor.y - size / 2);
    for placement in structures.placements_intersecting(
        origin.x,
        origin.y,
        window_size,
        window_size,
    ) {
        if placement.reference() != reference || placement.placement_anchor() != anchor {
            continue;
        }
        if let Some(group) = &placement.structure().group_id {
            families.insert(group.clone());
        }
        let id = placement.structure_id();
        if let Some(local) = id.strip_prefix("asteria:") {
            families.insert(local.to_owned());
            if let Some((family, _)) = local.rsplit_once('_') {
                families.insert(family.to_owned());
            }
        }
    }
}

#[derive(Clone, Copy)]
struct QueryRect {
    minimum: IVec2,
    maximum: IVec2,
    width: u32,
    depth: u32,
}

impl QueryRect {
    fn new(origin_x: i32, origin_z: i32, width: u32, depth: u32) -> Self {
        let maximum_x = origin_x
            .checked_add(i32::try_from(width - 1).expect("debug width must fit i32"))
            .expect("Structure debug rectangle X must fit i32");
        let maximum_z = origin_z
            .checked_add(i32::try_from(depth - 1).expect("debug depth must fit i32"))
            .expect("Structure debug rectangle Z must fit i32");
        Self {
            minimum: IVec2::new(origin_x, origin_z),
            maximum: IVec2::new(maximum_x, maximum_z),
            width,
            depth,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlacementFingerprint {
    reference: String,
    placement_anchor: [i32; 2],
    structure_id: String,
    group_id: Option<String>,
    rotation: u8,
    origin: [i32; 3],
    horizontal_minimum: [i32; 2],
    horizontal_maximum: [i32; 2],
    vertical_minimum: i32,
    vertical_maximum: i32,
}

fn query_fingerprints(
    structures: StructureQueries<'_>,
    rectangle: QueryRect,
) -> Vec<PlacementFingerprint> {
    let mut fingerprints = structures
        .placements_intersecting(
            rectangle.minimum.x,
            rectangle.minimum.y,
            rectangle.width,
            rectangle.depth,
        )
        .into_iter()
        .map(placement_fingerprint)
        .collect::<Vec<_>>();
    fingerprints.sort();
    fingerprints
}

fn placement_fingerprint(placement: StructurePlacement) -> PlacementFingerprint {
    let anchor = placement.placement_anchor();
    let origin = placement.origin();
    let (horizontal_minimum, horizontal_maximum) = placement.horizontal_bounds();
    let (vertical_minimum, vertical_maximum) = placement.vertical_bounds();
    PlacementFingerprint {
        reference: placement.reference().to_owned(),
        placement_anchor: [anchor.x, anchor.y],
        structure_id: placement.structure_id().to_owned(),
        group_id: placement.structure().group_id.clone(),
        rotation: rotation_index(placement.rotation()),
        origin: ivec3_array(origin),
        horizontal_minimum: [horizontal_minimum.x, horizontal_minimum.y],
        horizontal_maximum: [horizontal_maximum.x, horizontal_maximum.y],
        vertical_minimum,
        vertical_maximum,
    }
}

fn fingerprints_intersecting(
    fingerprints: &[PlacementFingerprint],
    rectangle: QueryRect,
) -> Vec<PlacementFingerprint> {
    fingerprints
        .iter()
        .filter(|fingerprint| {
            rectangles_overlap(
                IVec2::new(
                    fingerprint.horizontal_minimum[0],
                    fingerprint.horizontal_minimum[1],
                ),
                IVec2::new(
                    fingerprint.horizontal_maximum[0],
                    fingerprint.horizontal_maximum[1],
                ),
                rectangle.minimum,
                rectangle.maximum,
            )
        })
        .cloned()
        .collect()
}

fn rectangles_overlap(
    left_minimum: IVec2,
    left_maximum: IVec2,
    right_minimum: IVec2,
    right_maximum: IVec2,
) -> bool {
    left_minimum.x <= right_maximum.x
        && left_maximum.x >= right_minimum.x
        && left_minimum.y <= right_maximum.y
        && left_maximum.y >= right_minimum.y
}

fn midpoint(minimum: i32, maximum: i32) -> i32 {
    i32::try_from((i64::from(minimum) + i64::from(maximum)) / 2)
        .expect("Structure debug midpoint must fit i32")
}

fn rotation_index(rotation: StructureRotation) -> u8 {
    match rotation {
        StructureRotation::Degrees0 => 0,
        StructureRotation::Degrees90 => 1,
        StructureRotation::Degrees180 => 2,
        StructureRotation::Degrees270 => 3,
    }
}

const fn ivec3_array(value: IVec3) -> [i32; 3] {
    [value.x, value.y, value.z]
}
