use bevy::prelude::*;

use crate::{
    app::crash_log::{log_gameplay_event, log_gameplay_warn},
    content::{
        builtin_ids::BUCKET_FLUID_METADATA_KEY, fluid::FluidRegistry,
        tool_behavior::BUCKET_USE_BEHAVIOR_ID,
    },
    gameplay::availability::world_interaction_available,
    player::{
        camera::GameplayWorldCamera, hotbar::PlayerHotbar,
        viewmodel::ViewModelAnimation,
    },
    targeting::{block::BlockTargetingSet, ToolUse},
    voxel::{
        edit::VoxelTopologyRuntime,
        fluid::{FluidCell, MAX_FLUID_LEVEL},
        read::VoxelRead,
    },
};

const BUCKET_TARGET_RANGE: f32 = 8.0;

pub(super) struct BucketPlugin;

impl Plugin for BucketPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            handle_bucket_use
                .after(BlockTargetingSet::Interaction)
                .run_if(world_interaction_available),
        );
    }
}

fn handle_bucket_use(
    mut uses: MessageReader<ToolUse>,
    camera: Single<&GlobalTransform, With<GameplayWorldCamera>>,
    fluids: Res<FluidRegistry>,
    mut hotbar: ResMut<PlayerHotbar>,
    mut runtime: VoxelTopologyRuntime,
    mut viewmodel: ResMut<ViewModelAnimation>,
) {
    for usage in uses.read() {
        if usage.behavior_id != BUCKET_USE_BEHAVIOR_ID {
            continue;
        }

        let contained_fluid = hotbar
            .stack_at(hotbar.selected_slot())
            .and_then(|stack| stack.metadata().get(BUCKET_FLUID_METADATA_KEY))
            .map(str::to_owned);

        if let Some(fluid_definition_id) = contained_fluid {
            let Some(fluid_id) = fluids.id_of(&fluid_definition_id) else {
                log_gameplay_warn(format!(
                    "bucket.reject action=place fluid={} reason=unknown_fluid_metadata",
                    fluid_definition_id
                ));
                continue;
            };
            let Some(hit) = usage.target else {
                log_gameplay_event(format!(
                    "bucket.reject action=place fluid={} reason=no_target",
                    fluid_definition_id
                ));
                continue;
            };
            if runtime.read().block_id_at(hit.voxel) != Some(hit.block_id) {
                log_gameplay_event(format!(
                    "bucket.reject action=place fluid={} voxel={:?} reason=stale_target",
                    fluid_definition_id, hit.voxel
                ));
                continue;
            }

            let destination = hit.voxel + hit.normal;
            if destination.y < 0 {
                log_gameplay_event(format!(
                    "bucket.reject action=place fluid={} voxel={:?} reason=below_world",
                    fluid_definition_id, destination
                ));
                continue;
            }
            if runtime.read().cell_at(destination).is_some() {
                log_gameplay_event(format!(
                    "bucket.reject action=place fluid={} voxel={:?} reason=destination_blocked",
                    fluid_definition_id, destination
                ));
                continue;
            }
            if runtime.read().fluid_at(destination).is_some() {
                log_gameplay_event(format!(
                    "bucket.reject action=place fluid={} voxel={:?} reason=destination_has_fluid",
                    fluid_definition_id, destination
                ));
                continue;
            }

            let placed = FluidCell::source(fluid_id, MAX_FLUID_LEVEL);
            if runtime.set_fluid(destination, Some(placed)).is_none() {
                log_gameplay_warn(format!(
                    "bucket.reject action=place fluid={} voxel={:?} reason=fluid_mutation_rejected",
                    fluid_definition_id, destination
                ));
                continue;
            }
            if !transition_selected_bucket(&mut hotbar, None) {
                let restored = runtime.set_fluid(destination, None);
                debug_assert!(restored.is_some(), "bucket placement rollback must succeed");
                log_gameplay_event(format!(
                    "bucket.reject action=place fluid={} voxel={:?} reason=inventory_transition_failed rollback={}",
                    fluid_definition_id,
                    destination,
                    restored.is_some(),
                ));
                continue;
            }

            log_gameplay_event(format!(
                "bucket.place fluid={} voxel={:?}",
                fluid_definition_id, destination
            ));
            viewmodel.play_place();
            continue;
        }

        let origin = camera.translation();
        let direction = camera.forward().as_vec3();
        let Some((voxel, collected)) =
            raycast_collectible_fluid(&runtime.read(), origin, direction, BUCKET_TARGET_RANGE)
        else {
            log_gameplay_event("bucket.reject action=collect reason=no_collectible_source");
            continue;
        };
        let Some(definition) = fluids.get(collected.fluid_id) else {
            log_gameplay_warn(format!(
                "bucket.reject action=collect voxel={:?} runtime_fluid_id={} reason=missing_fluid_definition",
                voxel, collected.fluid_id
            ));
            continue;
        };
        let fluid_definition_id = definition.id.clone();

        if runtime.set_fluid(voxel, None).is_none() {
            log_gameplay_warn(format!(
                "bucket.reject action=collect fluid={} voxel={:?} reason=fluid_mutation_rejected",
                fluid_definition_id, voxel
            ));
            continue;
        }
        if !transition_selected_bucket(&mut hotbar, Some(&fluid_definition_id)) {
            let restored = runtime.set_fluid(voxel, Some(collected));
            debug_assert!(restored.is_some(), "bucket collection rollback must succeed");
            log_gameplay_event(format!(
                "bucket.reject action=collect fluid={} voxel={:?} reason=inventory_full rollback={}",
                fluid_definition_id,
                voxel,
                restored.is_some(),
            ));
            continue;
        }

        log_gameplay_event(format!(
            "bucket.collect fluid={} voxel={:?}",
            fluid_definition_id, voxel
        ));
        viewmodel.play_place();
    }
}

fn transition_selected_bucket(hotbar: &mut PlayerHotbar, fluid_id: Option<&str>) -> bool {
    let selected_slot = hotbar.selected_slot();
    let Some(selected) = hotbar.stack_at(selected_slot).cloned() else {
        return false;
    };

    let quantity = selected.quantity();
    let mut transitioned = selected.clone().with_quantity(1);
    transitioned = if let Some(fluid_id) = fluid_id {
        transitioned.with_metadata(BUCKET_FLUID_METADATA_KEY, fluid_id)
    } else {
        transitioned.without_metadata(BUCKET_FLUID_METADATA_KEY)
    };

    if quantity == 1 {
        hotbar.set_selected_stack(Some(transitioned));
        return true;
    }

    if hotbar.try_insert_stack(transitioned).is_err() {
        return false;
    }
    hotbar.consume_selected_item()
}

fn raycast_collectible_fluid(
    world: &impl VoxelRead,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<(IVec3, FluidCell)> {
    if direction.length_squared() == 0.0 || max_distance < 0.0 {
        return None;
    }

    let direction = direction.normalize();
    let mut voxel = origin.floor().as_ivec3();
    let step = IVec3::new(
        direction.x.signum() as i32,
        direction.y.signum() as i32,
        direction.z.signum() as i32,
    );
    let t_delta = Vec3::new(
        reciprocal_abs(direction.x),
        reciprocal_abs(direction.y),
        reciprocal_abs(direction.z),
    );
    let mut t_max = Vec3::new(
        first_boundary_distance(origin.x, voxel.x, direction.x),
        first_boundary_distance(origin.y, voxel.y, direction.y),
        first_boundary_distance(origin.z, voxel.z, direction.z),
    );

    loop {
        let distance = if t_max.x <= t_max.y && t_max.x <= t_max.z {
            let distance = t_max.x;
            voxel.x += step.x;
            t_max.x += t_delta.x;
            distance
        } else if t_max.y <= t_max.z {
            let distance = t_max.y;
            voxel.y += step.y;
            t_max.y += t_delta.y;
            distance
        } else {
            let distance = t_max.z;
            voxel.z += step.z;
            t_max.z += t_delta.z;
            distance
        };

        if distance > max_distance {
            return None;
        }
        if world.cell_at(voxel).is_some() {
            return None;
        }
        if let Some(fluid) = world.fluid_at(voxel) {
            return fluid.is_source().then_some((voxel, fluid));
        }
    }
}

fn reciprocal_abs(value: f32) -> f32 {
    if value == 0.0 {
        f32::INFINITY
    } else {
        1.0 / value.abs()
    }
}

fn first_boundary_distance(origin: f32, voxel: i32, direction: f32) -> f32 {
    if direction > 0.0 {
        (voxel as f32 + 1.0 - origin) / direction
    } else if direction < 0.0 {
        (origin - voxel as f32) / -direction
    } else {
        f32::INFINITY
    }
}
