use std::collections::{HashSet, VecDeque};

use bevy::prelude::*;

use crate::{
    app::crash_log::log_gameplay_event,
    content::{
        block::{BlockDefinition, BlockRegistry},
        builtin_ids::{
            DIMENSIONAL_SLICER_ITEM_ID, DIMENSIONAL_SLICER_TARGET_DIMENSION_METADATA_KEY,
            OVERWORLD_DIMENSION_ID, UMBRAL_DIMENSION_ID, UMBRAL_PORTAL_LAYER_ID,
        },
        dimension::DimensionRegistry,
        layer::LayerFace,
    },
    gameplay::availability::world_interaction_available,
    player::{hotbar::PlayerHotbar, viewmodel::ViewModelAnimation},
    voxel::{
        edit::VoxelTopologyRuntime, layer::LayerCell, read::VoxelRead,
        texture_rotation::TextureRotation,
    },
};

use super::block::{BlockTargetingSet, TargetedBlock};

const PORTAL_FRAME_TAG: &str = "is_portal_frame";
const ACCEPT_DIMENSION_TAG_PREFIX: &str = "accept_dimension:";
const MAX_PORTAL_FRAME_VOXELS: usize = 4096;
const MAX_PORTAL_INTERIOR_VOXELS: usize = 4096;
const MAX_PORTAL_SPAN: i32 = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PortalPlane {
    face: LayerFace,
    axis_u: IVec3,
    axis_v: IVec3,
}

const PORTAL_PLANES: [PortalPlane; 3] = [
    PortalPlane {
        face: LayerFace::Right,
        axis_u: IVec3::Y,
        axis_v: IVec3::Z,
    },
    PortalPlane {
        face: LayerFace::Top,
        axis_u: IVec3::X,
        axis_v: IVec3::Z,
    },
    PortalPlane {
        face: LayerFace::Front,
        axis_u: IVec3::X,
        axis_v: IVec3::Y,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PortalCell {
    Frame,
    Empty,
    Blocked,
    Unloaded,
}

#[derive(Debug, Eq, PartialEq)]
struct PortalShape {
    face: LayerFace,
    interior: Vec<IVec3>,
}

pub(super) struct PortalActivationPlugin;

impl Plugin for PortalActivationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            activate_dimensional_slicer
                .in_set(BlockTargetingSet::PlacementState)
                .run_if(world_interaction_available),
        );
    }
}

fn activate_dimensional_slicer(
    buttons: Res<ButtonInput<MouseButton>>,
    mut hotbar: ResMut<PlayerHotbar>,
    blocks: Res<BlockRegistry>,
    dimensions: Res<DimensionRegistry>,
    mut targeted: ResMut<TargetedBlock>,
    mut runtime: VoxelTopologyRuntime,
    mut viewmodel_animation: ResMut<ViewModelAnimation>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }

    let selected_slot = hotbar.selected_slot();
    let Some(stack) = hotbar.stack_at(selected_slot) else {
        return;
    };
    if stack.id() != DIMENSIONAL_SLICER_ITEM_ID {
        return;
    }
    let Some(target_dimension) = stack
        .metadata()
        .get(DIMENSIONAL_SLICER_TARGET_DIMENSION_METADATA_KEY)
        .map(str::to_owned)
    else {
        log_gameplay_event("portal.activate rejected reason=missing_target_dimension".to_owned());
        return;
    };
    if dimensions.get(&target_dimension).is_none() {
        log_gameplay_event(format!(
            "portal.activate rejected dimension={target_dimension} reason=unknown_target_dimension"
        ));
        return;
    }
    let Some(portal_layer_id) = portal_layer_for_target_dimension(&target_dimension) else {
        log_gameplay_event(format!(
            "portal.activate rejected dimension={target_dimension} reason=no_portal_layer"
        ));
        return;
    };
    let Some(hit) = targeted.0 else {
        return;
    };

    let shape = {
        let world = runtime.read();
        find_portal_shape(hit.voxel, &target_dimension, &blocks, &world)
    };
    let Some(shape) = shape else {
        log_gameplay_event(format!(
            "portal.activate rejected dimension={target_dimension} frame={:?} reason=no_closed_planar_frame",
            hit.voxel
        ));
        return;
    };

    if !shape
        .interior
        .iter()
        .all(|&voxel| runtime.can_add_layer(voxel, shape.face, portal_layer_id))
    {
        log_gameplay_event(format!(
            "portal.activate rejected dimension={target_dimension} frame={:?} reason=portal_interior_unavailable",
            hit.voxel
        ));
        return;
    }

    for &voxel in &shape.interior {
        let added = runtime.add_layer(
            voxel,
            shape.face,
            LayerCell::new(portal_layer_id, TextureRotation::default()),
        );
        assert!(
            added.is_some(),
            "preflighted portal layer insertion must succeed at {voxel:?}"
        );
    }

    let consumed = hotbar.consume_selected_item();
    debug_assert!(consumed, "successful portal activation must consume its slicer");
    targeted.0 = None;
    viewmodel_animation.play_place();
    log_gameplay_event(format!(
        "portal.activate dimension={target_dimension} frame={:?} face={:?} interior_voxels={}",
        hit.voxel,
        shape.face,
        shape.interior.len()
    ));
}

fn portal_layer_for_target_dimension(target_dimension: &str) -> Option<&'static str> {
    matches!(target_dimension, OVERWORLD_DIMENSION_ID | UMBRAL_DIMENSION_ID)
        .then_some(UMBRAL_PORTAL_LAYER_ID)
}

fn find_portal_shape(
    clicked_frame: IVec3,
    target_dimension: &str,
    blocks: &BlockRegistry,
    world: &impl VoxelRead,
) -> Option<PortalShape> {
    if classify_world_cell(clicked_frame, target_dimension, blocks, world) != PortalCell::Frame {
        return None;
    }

    let mut found = None;
    for plane in PORTAL_PLANES {
        let Some(interior) = find_plane_interior(clicked_frame, plane, |voxel| {
            classify_world_cell(voxel, target_dimension, blocks, world)
        }) else {
            continue;
        };
        if found.is_some() {
            return None;
        }
        found = Some(PortalShape {
            face: plane.face,
            interior,
        });
    }
    found
}

fn classify_world_cell(
    voxel: IVec3,
    target_dimension: &str,
    blocks: &BlockRegistry,
    world: &impl VoxelRead,
) -> PortalCell {
    let Some((cell, fluid, _)) = world.sample_at(voxel) else {
        return PortalCell::Unloaded;
    };
    if let Some(cell) = cell {
        return blocks
            .get(cell.block_id)
            .filter(|block| block_accepts_portal_dimension(block, target_dimension))
            .map_or(PortalCell::Blocked, |_| PortalCell::Frame);
    }
    if fluid.is_some() {
        PortalCell::Blocked
    } else {
        PortalCell::Empty
    }
}

fn block_accepts_portal_dimension(block: &BlockDefinition, target_dimension: &str) -> bool {
    block.tags.iter().any(|tag| tag == PORTAL_FRAME_TAG)
        && block.tags.iter().any(|tag| {
            tag.strip_prefix(ACCEPT_DIMENSION_TAG_PREFIX)
                .is_some_and(|dimension| dimension == target_dimension)
        })
}

fn find_plane_interior(
    clicked_frame: IVec3,
    plane: PortalPlane,
    mut classify: impl FnMut(IVec3) -> PortalCell,
) -> Option<Vec<IVec3>> {
    let frame = collect_connected_frame(clicked_frame, plane, &mut classify)?;
    let mut attempted_seeds = HashSet::new();

    for frame_voxel in frame {
        for seed in plane_neighbors(frame_voxel, plane) {
            if !attempted_seeds.insert(seed) || classify(seed) != PortalCell::Empty {
                continue;
            }
            if let Some(interior) = flood_closed_interior(seed, plane, &mut classify) {
                return Some(interior);
            }
        }
    }
    None
}

fn collect_connected_frame(
    clicked_frame: IVec3,
    plane: PortalPlane,
    classify: &mut impl FnMut(IVec3) -> PortalCell,
) -> Option<Vec<IVec3>> {
    if classify(clicked_frame) != PortalCell::Frame {
        return None;
    }

    let mut queue = VecDeque::from([clicked_frame]);
    let mut visited = HashSet::from([clicked_frame]);

    while let Some(voxel) = queue.pop_front() {
        if visited.len() > MAX_PORTAL_FRAME_VOXELS || exceeds_portal_span(clicked_frame, voxel) {
            return None;
        }
        for neighbor in plane_neighbors(voxel, plane) {
            if classify(neighbor) == PortalCell::Frame && visited.insert(neighbor) {
                queue.push_back(neighbor);
            }
        }
    }

    let mut frame = visited.into_iter().collect::<Vec<_>>();
    frame.sort_by_key(|voxel| (voxel.x, voxel.y, voxel.z));
    Some(frame)
}

fn flood_closed_interior(
    seed: IVec3,
    plane: PortalPlane,
    classify: &mut impl FnMut(IVec3) -> PortalCell,
) -> Option<Vec<IVec3>> {
    let mut queue = VecDeque::from([seed]);
    let mut visited = HashSet::from([seed]);

    while let Some(voxel) = queue.pop_front() {
        if visited.len() > MAX_PORTAL_INTERIOR_VOXELS || exceeds_portal_span(seed, voxel) {
            return None;
        }

        for neighbor in plane_neighbors(voxel, plane) {
            match classify(neighbor) {
                PortalCell::Frame => {}
                PortalCell::Empty => {
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
                PortalCell::Blocked | PortalCell::Unloaded => return None,
            }
        }
    }

    let mut interior = visited.into_iter().collect::<Vec<_>>();
    interior.sort_by_key(|voxel| (voxel.x, voxel.y, voxel.z));
    Some(interior)
}

fn plane_neighbors(voxel: IVec3, plane: PortalPlane) -> [IVec3; 4] {
    [
        voxel + plane.axis_u,
        voxel - plane.axis_u,
        voxel + plane.axis_v,
        voxel - plane.axis_v,
    ]
}

fn exceeds_portal_span(origin: IVec3, voxel: IVec3) -> bool {
    let delta = (voxel - origin).abs();
    delta.x > MAX_PORTAL_SPAN || delta.y > MAX_PORTAL_SPAN || delta.z > MAX_PORTAL_SPAN
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle_frame(
        plane: PortalPlane,
        origin: IVec3,
        width: i32,
        height: i32,
    ) -> HashSet<IVec3> {
        let mut frame = HashSet::new();
        for u in 0..width {
            frame.insert(origin + plane.axis_u * u);
            frame.insert(origin + plane.axis_u * u + plane.axis_v * (height - 1));
        }
        for v in 0..height {
            frame.insert(origin + plane.axis_v * v);
            frame.insert(origin + plane.axis_u * (width - 1) + plane.axis_v * v);
        }
        frame
    }

    fn classify_test_frame(
        frame: &HashSet<IVec3>,
        plane: PortalPlane,
        origin: IVec3,
        voxel: IVec3,
    ) -> PortalCell {
        if frame.contains(&voxel) {
            PortalCell::Frame
        } else {
            let normal = match plane.face {
                LayerFace::Right => IVec3::X,
                LayerFace::Top => IVec3::Y,
                LayerFace::Front => IVec3::Z,
                _ => unreachable!("portal test planes only use canonical positive faces"),
            };
            if (voxel - origin).dot(normal).abs() > 0 {
                PortalCell::Unloaded
            } else if (voxel - origin).abs().max_element() > 8 {
                PortalCell::Unloaded
            } else {
                PortalCell::Empty
            }
        }
    }

    #[test]
    fn closed_frame_resolves_interior_from_edge_block() {
        let plane = PORTAL_PLANES[2];
        let origin = IVec3::new(10, 20, 30);
        let frame = rectangle_frame(plane, origin, 5, 4);
        let clicked = origin + plane.axis_u;
        let interior = find_plane_interior(clicked, plane, |voxel| {
            classify_test_frame(&frame, plane, origin, voxel)
        })
        .expect("closed frame should have an interior");

        assert_eq!(interior.len(), 6);
        assert!(interior.contains(&(origin + plane.axis_u + plane.axis_v)));
    }

    #[test]
    fn closed_frame_resolves_interior_from_corner_block() {
        let plane = PORTAL_PLANES[2];
        let origin = IVec3::new(10, 20, 30);
        let frame = rectangle_frame(plane, origin, 5, 4);
        let interior = find_plane_interior(origin, plane, |voxel| {
            classify_test_frame(&frame, plane, origin, voxel)
        })
        .expect("clicking a frame corner should still resolve the enclosed interior");

        assert_eq!(interior.len(), 6);
    }

    #[test]
    fn open_frame_is_rejected() {
        let plane = PORTAL_PLANES[2];
        let origin = IVec3::new(10, 20, 30);
        let mut frame = rectangle_frame(plane, origin, 5, 4);
        frame.remove(&(origin + plane.axis_u * 2 + plane.axis_v * 3));
        let clicked = origin + plane.axis_u;

        let interior = find_plane_interior(clicked, plane, |voxel| {
            classify_test_frame(&frame, plane, origin, voxel)
        });

        assert!(interior.is_none());
    }

    #[test]
    fn all_three_planes_map_to_their_centered_layer_face() {
        assert_eq!(PORTAL_PLANES[0].face, LayerFace::Right);
        assert_eq!(PORTAL_PLANES[1].face, LayerFace::Top);
        assert_eq!(PORTAL_PLANES[2].face, LayerFace::Front);
    }
}
