use bevy::{ecs::system::SystemParam, light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{block::BlockRegistry, builtin_ids::GRASS_BLOCK_ID},
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    rendering::block_model::{
        block_face_material, block_face_material_data, block_face_mesh, block_faces,
    },
    voxel::{mesh::BlockFace, world::VoxelWorld},
};

use super::{
    block::{BlockTargetingSet, TargetedBlock},
    placement::placement_voxel,
};

const PREVIEW_OPACITY: f32 = 0.68;

type PreviewRoot<'w, 's> = Single<
    'w,
    's,
    (
        &'static mut PlacementPreviewRoot,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (Without<PlacementPreviewFace>, Without<GameplayCamera>),
>;

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

#[derive(SystemParam)]
struct PlacementPreviewInput<'w, 's> {
    targeted: Res<'w, TargetedBlock>,
    hotbar: Res<'w, PlayerHotbar>,
    blocks: Res<'w, BlockRegistry>,
    asset_server: Res<'w, AssetServer>,
    world: Res<'w, VoxelWorld>,
    player: Single<'w, 's, &'static Transform, With<GameplayCamera>>,
}

fn update_placement_preview(
    input: PlacementPreviewInput,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut root: PreviewRoot,
    faces: Query<(&PlacementPreviewFace, &MeshMaterial3d<StandardMaterial>)>,
) {
    let Some(block_id) = input.hotbar.item_at(input.hotbar.selected_slot()) else {
        *root.2 = Visibility::Hidden;
        return;
    };

    if root.0.block_id != block_id {
        let block = input
            .blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("placement preview references missing block: {block_id}"));

        root.0.block_id = block_id;

        for (face, material_handle) in &faces {
            let Some(material) = materials.get_mut(&material_handle.0) else {
                continue;
            };

            *material =
                block_face_material_data(face.face, block, &input.asset_server, PREVIEW_OPACITY);
        }
    }

    let Some(hit) = input.targeted.0 else {
        *root.2 = Visibility::Hidden;
        return;
    };
    let Some(voxel) = placement_voxel(hit, &input.world, input.player.translation) else {
        *root.2 = Visibility::Hidden;
        return;
    };

    root.1.translation = voxel.as_vec3() + Vec3::splat(0.5);
    *root.2 = Visibility::Visible;
}
