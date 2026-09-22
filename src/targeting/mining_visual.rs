use bevy::{
    asset::RenderAssetUsages,
    light::NotShadowCaster,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

use crate::{
    app::game_state::GameState,
    content::block::{BlockRegistry, DEFAULT_BLOCK_BREAK_TICKS},
    voxel::{
        block_face::BlockFace,
        microblock::{MICROBLOCK_EDGE, MicroblockMask, occupied_cell, parent_voxel},
        quad::QUAD_TRIANGLE_INDICES,
        world::VoxelWorld,
    },
};

use super::{
    BlockMiningState,
    block::{BlockTargetingSet, TargetedBlock},
};

const DESTROY_STAGE_TEXTURES: [&str; 10] = [
    "textures/destroy_stage/destroy_stage_0.png",
    "textures/destroy_stage/destroy_stage_1.png",
    "textures/destroy_stage/destroy_stage_2.png",
    "textures/destroy_stage/destroy_stage_3.png",
    "textures/destroy_stage/destroy_stage_4.png",
    "textures/destroy_stage/destroy_stage_5.png",
    "textures/destroy_stage/destroy_stage_6.png",
    "textures/destroy_stage/destroy_stage_7.png",
    "textures/destroy_stage/destroy_stage_8.png",
    "textures/destroy_stage/destroy_stage_9.png",
];
const BREAK_SURFACE_OFFSET: f32 = 0.0015;
const BREAK_DEPTH_BIAS: f32 = 120.0;

#[derive(Resource)]
struct BreakingOverlayMaterials([Handle<StandardMaterial>; DESTROY_STAGE_TEXTURES.len()]);

#[derive(Component, Default)]
struct BreakingOverlay {
    target: Option<(IVec3, &'static str)>,
    stage: Option<usize>,
}

pub(super) struct BlockMiningVisualPlugin;

impl Plugin for BlockMiningVisualPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_breaking_overlay)
            .add_systems(
                Update,
                update_breaking_overlay
                    .in_set(BlockTargetingSet::Visuals)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn spawn_breaking_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let stage_materials = std::array::from_fn(|stage| {
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(asset_server.load(DESTROY_STAGE_TEXTURES[stage])),
            alpha_mode: AlphaMode::Mask(0.01),
            unlit: true,
            double_sided: true,
            cull_mode: None,
            depth_bias: BREAK_DEPTH_BIAS,
            ..default()
        })
    });

    commands.insert_resource(BreakingOverlayMaterials(stage_materials.clone()));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::ONE))),
        MeshMaterial3d(stage_materials[0].clone()),
        Transform::default(),
        Visibility::Hidden,
        NotShadowCaster,
        BreakingOverlay::default(),
        DespawnOnExit(GameState::Gameplay),
    ));
}

fn update_breaking_overlay(
    targeted: Res<TargetedBlock>,
    mining: Res<BlockMiningState>,
    blocks: Res<BlockRegistry>,
    world: Res<VoxelWorld>,
    stage_materials: Res<BreakingOverlayMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    overlay: Single<
        (
            &mut BreakingOverlay,
            &mut Transform,
            &mut Visibility,
            &Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<BreakingOverlay>,
    >,
) {
    let (mut overlay, mut transform, mut visibility, mesh_handle, mut material) =
        overlay.into_inner();

    let Some(hit) = targeted.0 else {
        hide(&mut visibility);
        overlay.stage = None;
        return;
    };
    let Some(block) = blocks.get(hit.block_id) else {
        hide(&mut visibility);
        overlay.stage = None;
        return;
    };

    let required_work = DEFAULT_BLOCK_BREAK_TICKS as f32 * block.mining.hardness;
    let Some(progress) = mining.progress_for(hit.voxel, hit.block_id, required_work) else {
        hide(&mut visibility);
        overlay.stage = None;
        return;
    };

    // Required-tool mismatches intentionally remain at 0% forever and should
    // not imply that the block is being damaged.
    if progress <= 0.0 {
        hide(&mut visibility);
        overlay.stage = None;
        return;
    }

    let target = (hit.voxel, hit.block_id);
    if overlay.target != Some(target) {
        let Some(next_mesh) = breaking_surface_mesh(&world, &blocks, hit.voxel) else {
            hide(&mut visibility);
            overlay.target = Some(target);
            overlay.stage = None;
            return;
        };
        if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
            *mesh = next_mesh;
        }
        transform.translation = hit.voxel.as_vec3();
        overlay.target = Some(target);
        overlay.stage = None;
    }

    let stage = ((progress * DESTROY_STAGE_TEXTURES.len() as f32).floor() as usize)
        .min(DESTROY_STAGE_TEXTURES.len() - 1);
    if overlay.stage != Some(stage) {
        material.0 = stage_materials.0[stage].clone();
        overlay.stage = Some(stage);
    }

    if *visibility != Visibility::Visible {
        *visibility = Visibility::Visible;
    }
}

fn breaking_surface_mesh(
    world: &VoxelWorld,
    blocks: &BlockRegistry,
    voxel: IVec3,
) -> Option<Mesh> {
    let cell = world.cell_at(voxel)?;
    let mask = MicroblockMask::from_cell(cell);
    let edge = MICROBLOCK_EDGE as usize;

    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut indices = Vec::<u32>::new();

    for z in 0..edge {
        for y in 0..edge {
            for x in 0..edge {
                let local = [x, y, z];
                if !mask.contains(local) {
                    continue;
                }

                for face in BlockFace::ALL {
                    if !surface_face_visible(world, blocks, voxel, local, face) {
                        continue;
                    }
                    push_surface_quad(
                        &mut positions,
                        &mut normals,
                        &mut uvs,
                        &mut indices,
                        local,
                        face,
                    );
                }
            }
        }
    }

    if positions.is_empty() {
        return None;
    }

    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices)),
    )
}

fn surface_face_visible(
    world: &VoxelWorld,
    blocks: &BlockRegistry,
    voxel: IVec3,
    local: [usize; 3],
    face: BlockFace,
) -> bool {
    let fine = voxel * MICROBLOCK_EDGE
        + IVec3::new(local[0] as i32, local[1] as i32, local[2] as i32);
    let neighbor_fine = fine + face.offset();
    let neighbor_voxel = parent_voxel(neighbor_fine);
    let Some(neighbor) = occupied_cell(world, neighbor_fine) else {
        return true;
    };

    if neighbor_voxel == voxel {
        return false;
    }

    blocks
        .get(neighbor.block_id)
        .is_none_or(|definition| definition.alpha_blend || definition.alpha_cutoff.is_some())
}

fn push_surface_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    local: [usize; 3],
    face: BlockFace,
) {
    let edge = MICROBLOCK_EDGE as f32;
    let min = Vec3::new(
        local[0] as f32 / edge,
        local[1] as f32 / edge,
        local[2] as f32 / edge,
    );
    let max = min + Vec3::splat(1.0 / edge);
    let normal = Vec3::from_array(face.normal());
    let base = positions.len() as u32;

    for corner in face.unit_vertices() {
        let corner = Vec3::from_array(corner);
        let point = Vec3::new(
            if corner.x == 0.0 { min.x } else { max.x },
            if corner.y == 0.0 { min.y } else { max.y },
            if corner.z == 0.0 { min.z } else { max.z },
        );
        let displaced = point + normal * BREAK_SURFACE_OFFSET;
        positions.push(displaced.to_array());
        normals.push(face.normal());
        uvs.push(macro_uv(face, point));
    }

    indices.extend(QUAD_TRIANGLE_INDICES.map(|index| base + index));
}

fn macro_uv(face: BlockFace, point: Vec3) -> [f32; 2] {
    match face {
        BlockFace::Right => [1.0 - point.z, 1.0 - point.y],
        BlockFace::Left => [point.z, 1.0 - point.y],
        BlockFace::Top => [point.x, point.z],
        BlockFace::Bottom => [point.x, 1.0 - point.z],
        BlockFace::Front => [point.x, 1.0 - point.y],
        BlockFace::Back => [1.0 - point.x, 1.0 - point.y],
    }
}

fn hide(visibility: &mut Visibility) {
    if *visibility != Visibility::Hidden {
        *visibility = Visibility::Hidden;
    }
}
