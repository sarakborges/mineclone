use bevy::{ecs::system::SystemParam, light::NotShadowCaster, prelude::*};

use crate::{
    app::game_state::GameState,
    content::block::BlockRegistry,
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    rendering::{
        block_model::{
            BlockModel, BlockModelMaterials, BlockModelMeshes, block_face_material_data,
            maximum_block_model_layers, set_block_model_tint,
        },
        block_model_material::BlockModelMaterial,
        block_visual_content::BlockVisualContent,
    },
    voxel::{block_face::BlockFace, orientation::orientation_rotation},
};

use super::{
    BlockTargetingScene,
    block::BlockTargetingSet,
    placement::placement_voxel,
    placement_orientation::PlacementOrientation,
};

const PREVIEW_OPACITY: f32 = 0.82;

type PreviewRoot<'w, 's> = Single<
    'w,
    's,
    (
        &'static mut BlockModel,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (
        With<PlacementPreviewRoot>,
        Without<PlacementPreviewFace>,
        Without<GameplayCamera>,
    ),
>;

#[derive(Component)]
struct PlacementPreviewRoot;

#[derive(Component)]
struct PlacementPreviewFace {
    face: BlockFace,
    layer_index: usize,
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

#[derive(SystemParam)]
struct PlacementPreviewSpawnContent<'w> {
    asset_server: Res<'w, AssetServer>,
    block_meshes: Res<'w, BlockModelMeshes>,
    blocks: Res<'w, BlockRegistry>,
    hotbar: Res<'w, PlayerHotbar>,
}

#[derive(SystemParam)]
struct PlacementPreviewSpawnAssets<'w> {
    block_materials: ResMut<'w, BlockModelMaterials>,
    materials: ResMut<'w, Assets<BlockModelMaterial>>,
}

fn spawn_placement_preview(
    mut commands: Commands,
    content: PlacementPreviewSpawnContent,
    assets: PlacementPreviewSpawnAssets,
) {
    let PlacementPreviewSpawnAssets {
        mut block_materials,
        mut materials,
    } = assets;
    let selected = content
        .hotbar
        .item_at(content.hotbar.selected_slot())
        .and_then(|block_id| {
            content
                .blocks
                .get(block_id)
                .map(|block| (block_id, block))
        });
    let block_model = selected
        .map(|(block_id, _)| BlockModel::world(block_id, PREVIEW_OPACITY))
        .unwrap_or_else(|| BlockModel::empty_world(PREVIEW_OPACITY));
    let initial_rotation = selected.map_or(Quat::IDENTITY, |(_, block)| {
        orientation_rotation(block.default_orientation())
    });

    commands
        .spawn((
            PlacementPreviewRoot,
            block_model,
            Transform::from_rotation(initial_rotation),
            Visibility::Hidden,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|preview| {
            for &face in block_model.faces() {
                let layer_count = maximum_block_model_layers(&content.blocks, face);
                let layer_materials =
                    block_materials.preview_for_face(face, layer_count, &mut materials);

                for (layer_index, material) in layer_materials.into_iter().enumerate() {
                    let visibility = if let Some((_, block)) = selected
                        && let Some(face_material) = block_face_material_data(
                            face,
                            layer_index,
                            block,
                            &content.asset_server,
                            block_model.opacity(),
                        )
                    {
                        if let Some(mut material_asset) = materials.get_mut(&material) {
                            *material_asset = face_material;
                        }
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    };

                    preview.spawn((
                        PlacementPreviewFace { face, layer_index },
                        Mesh3d(content.block_meshes.world_face(face)),
                        MeshMaterial3d(material),
                        visibility,
                        NotShadowCaster,
                    ));
                }
            }
        });
}

#[derive(SystemParam)]
struct PlacementPreviewSelection<'w, 's> {
    scene: BlockTargetingScene<'w, 's>,
    placement_orientation: Res<'w, PlacementOrientation>,
}

#[derive(SystemParam)]
struct PlacementPreviewView<'w, 's> {
    materials: ResMut<'w, Assets<BlockModelMaterial>>,
    root: PreviewRoot<'w, 's>,
    faces: Query<
        'w,
        's,
        (
            &'static PlacementPreviewFace,
            &'static MeshMaterial3d<BlockModelMaterial>,
            &'static mut Visibility,
        ),
    >,
}

fn update_placement_preview(
    selection: PlacementPreviewSelection,
    content: BlockVisualContent,
    view: PlacementPreviewView,
) {
    let PlacementPreviewView {
        mut materials,
        mut root,
        mut faces,
    } = view;
    let selected_slot = selection.scene.selected_slot();
    let selected = selection.scene.selected_item().and_then(|block_id| {
        content
            .blocks
            .get(block_id)
            .map(|block| (block_id, block))
    });
    let Some((block_id, block)) = selected else {
        root.0.set_block_id(None);
        root.1.translation = Vec3::ZERO;
        root.1.rotation = Quat::IDENTITY;
        for (_, _, mut visibility) in &mut faces {
            *visibility = Visibility::Hidden;
        }
        *root.2 = Visibility::Hidden;
        return;
    };

    let block_changed = root.0.set_block_id(Some(block_id));
    if block_changed {
        root.1.translation = Vec3::ZERO;
        *root.2 = Visibility::Hidden;

        for (face, material_handle, mut visibility) in &mut faces {
            let Some(mut material) = materials.get_mut(&material_handle.0) else {
                continue;
            };

            if let Some(face_material) = block_face_material_data(
                face.face,
                face.layer_index,
                block,
                &content.asset_server,
                root.0.opacity(),
            ) {
                *material = face_material;
                *visibility = Visibility::Visible;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }

    let orientation = selection
        .placement_orientation
        .for_block(selected_slot, block);
    root.1.rotation = orientation_rotation(orientation);

    let Some(hit) = selection.scene.hit() else {
        root.1.translation = Vec3::ZERO;
        *root.2 = Visibility::Hidden;
        return;
    };
    let Some(voxel) = placement_voxel(
        hit,
        selection.scene.world(),
        selection.scene.player_translation(),
    ) else {
        root.1.translation = Vec3::ZERO;
        *root.2 = Visibility::Hidden;
        return;
    };

    let tint_position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
    let tint = content.tint_at(block_id, tint_position).unwrap_or(Color::WHITE);

    for (_, material_handle, _) in &mut faces {
        let Some(mut material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        set_block_model_tint(&mut material, tint);
    }

    root.1.translation = voxel.as_vec3() + Vec3::splat(0.5);
    *root.2 = Visibility::Visible;
}
