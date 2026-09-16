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
    BlockTargetingScene, BlockTargetingVisualSnapshot, block::BlockTargetingSet,
    placement::placement_voxel, placement_orientation::PlacementOrientation,
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
        .and_then(|block_id| content.blocks.get(block_id).map(|block| (block_id, block)));
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
                        ) {
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
    mut tint_target: Local<Option<(&'static str, IVec2)>>,
    mut last_scene: Local<Option<BlockTargetingVisualSnapshot>>,
) {
    let scene_snapshot = selection.scene.visual_snapshot();
    let scene_changed = last_scene.as_ref() != Some(&scene_snapshot);
    let content_changed = content.inputs_changed();
    if !scene_changed && !selection.placement_orientation.is_changed() && !content_changed {
        return;
    }
    *last_scene = Some(scene_snapshot);

    let PlacementPreviewView {
        mut materials,
        mut root,
        mut faces,
    } = view;
    let selected_slot = selection.scene.selected_slot();
    let selected = selection
        .scene
        .selected_item()
        .and_then(|block_id| content.blocks.get(block_id).map(|block| (block_id, block)));
    let Some((block_id, block)) = selected else {
        let block_changed = if root.0.block_id().is_some() {
            root.0.set_block_id(None)
        } else {
            false
        };
        reset_transform_if_needed(&mut root.1);
        hide_if_visible(&mut root.2);
        if block_changed {
            for (_, _, mut visibility) in &mut faces {
                hide_if_visible(&mut visibility);
            }
        }
        *tint_target = None;
        return;
    };

    let block_changed = if root.0.block_id() != Some(block_id) {
        root.0.set_block_id(Some(block_id))
    } else {
        false
    };
    let block_definitions_changed = content.block_definitions_changed();
    if block_changed || block_definitions_changed {
        if root.1.translation != Vec3::ZERO {
            root.1.translation = Vec3::ZERO;
        }
        hide_if_visible(&mut root.2);

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
                show_if_hidden(&mut visibility);
            } else {
                hide_if_visible(&mut visibility);
            }
        }
    }

    let orientation = selection
        .placement_orientation
        .for_block(selected_slot, block);
    let rotation = orientation_rotation(orientation);
    if root.1.rotation != rotation {
        root.1.rotation = rotation;
    }

    let Some(hit) = selection.scene.hit() else {
        if root.1.translation != Vec3::ZERO {
            root.1.translation = Vec3::ZERO;
        }
        hide_if_visible(&mut root.2);
        *tint_target = None;
        return;
    };
    let Some(voxel) = placement_voxel(
        hit,
        selection.scene.world(),
        selection.scene.player_translation(),
    ) else {
        if root.1.translation != Vec3::ZERO {
            root.1.translation = Vec3::ZERO;
        }
        hide_if_visible(&mut root.2);
        *tint_target = None;
        return;
    };

    let horizontal = IVec2::new(voxel.x, voxel.z);
    let tint_target_changed =
        tint_target
            .as_ref()
            .is_none_or(|(cached_block_id, cached_horizontal)| {
                *cached_block_id != block_id || *cached_horizontal != horizontal
            });
    if block_changed || content_changed || tint_target_changed {
        let tint_position = Vec2::new(voxel.x as f32 + 0.5, voxel.z as f32 + 0.5);
        let tint = content
            .tint_at(block_id, tint_position)
            .unwrap_or(Color::WHITE);

        for (_, material_handle, _) in &mut faces {
            let Some(mut material) = materials.get_mut(&material_handle.0) else {
                continue;
            };

            set_block_model_tint(&mut material, tint);
        }
        *tint_target = Some((block_id, horizontal));
    }

    let translation = voxel.as_vec3() + Vec3::splat(0.5);
    if root.1.translation != translation {
        root.1.translation = translation;
    }
    show_if_hidden(&mut root.2);
}

fn reset_transform_if_needed(transform: &mut Transform) {
    if transform.translation != Vec3::ZERO {
        transform.translation = Vec3::ZERO;
    }
    if transform.rotation != Quat::IDENTITY {
        transform.rotation = Quat::IDENTITY;
    }
}

fn hide_if_visible(visibility: &mut Visibility) {
    if *visibility != Visibility::Hidden {
        *visibility = Visibility::Hidden;
    }
}

fn show_if_hidden(visibility: &mut Visibility) {
    if *visibility != Visibility::Visible {
        *visibility = Visibility::Visible;
    }
}
