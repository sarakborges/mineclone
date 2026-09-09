use bevy::{light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    content::block::BlockRegistry,
    player::{
        camera::GameplayCamera,
        hotbar::{GRASS_BLOCK_ID, PlayerHotbar},
    },
    rendering::block_model::{block_face_material, block_face_mesh, block_faces},
    voxel::mesh::BlockFace,
};

use super::{
    block::{BlockTargetingSet, TargetedBlock},
    placement::placement_voxel,
};

const PREVIEW_OPACITY: f32 = 0.68;

#[derive(Component)]
struct PlacementPreviewRoot {
    block_id: &'static str,
}

#[derive(Component)]
struct PlacementPreviewFace {
    face: BlockFace,
}

pub struct PlacementPreviewPlugin;

impl Plugin for PlacementPreviewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_placement_preview)
            .add_systems(
                Update,
                update_placement_preview
                    .in_set(BlockTargetingSet::Visuals)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

fn spawn_placement_preview(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    blocks: Res<BlockRegistry>,
    hotbar: Res<PlayerHotbar>,
) {
    let block_id = hotbar
        .item_at(hotbar.selected_slot())
        .unwrap_or(GRASS_BLOCK_ID);
    let block = blocks
        .get(block_id)
        .unwrap_or_else(|| panic!("placement preview references missing block: {block_id}"));

    commands
        .spawn((
            PlacementPreviewRoot { block_id },
            Transform::default(),
            Visibility::Hidden,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|preview| {
            for face in block_faces() {
                preview.spawn((
                    PlacementPreviewFace { face },
                    Mesh3d(meshes.add(block_face_mesh(face))),
                    MeshMaterial3d(block_face_material(
                        face,
                        block,
                        &asset_server,
                        &mut materials,
                        PREVIEW_OPACITY,
                    )),
                    NotShadowCaster,
                ));
            }
        });
}

fn update_placement_preview(
    targeted: Res<TargetedBlock>,
    hotbar: Res<PlayerHotbar>,
    blocks: Res<BlockRegistry>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world: Res<crate::voxel::world::VoxelWorld>,
    player: Single<&Transform, With<GameplayCamera>>,
    mut root: Single<
        (&mut PlacementPreviewRoot, &mut Transform, &mut Visibility),
        (Without<PlacementPreviewFace>, Without<GameplayCamera>),
    >,
    mut faces: Query<(&PlacementPreviewFace, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let Some(block_id) = hotbar.item_at(hotbar.selected_slot()) else {
        *root.2 = Visibility::Hidden;
        return;
    };

    if root.0.block_id != block_id {
        let block = blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("placement preview references missing block: {block_id}"));

        root.0.block_id = block_id;

        for (face, mut material) in &mut faces {
            material.0 = block_face_material(
                face.face,
                block,
                &asset_server,
                &mut materials,
                PREVIEW_OPACITY,
            );
        }
    }

    let Some(hit) = targeted.0 else {
        *root.2 = Visibility::Hidden;
        return;
    };
    let Some(voxel) = placement_voxel(hit, &world, player.translation) else {
        *root.2 = Visibility::Hidden;
        return;
    };

    root.1.translation = voxel.as_vec3() + Vec3::splat(0.5);
    *root.2 = Visibility::Visible;
}
