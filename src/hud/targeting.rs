use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{biome::BiomeRegistry, block::BlockRegistry},
    hud::block_icon::BlockIconMaterial,
    rendering::{
        block_model::BlockModelInstance,
        block_tint::block_tint_at,
    },
    targeting::block::TargetedBlock,
    ui::{surface, typography},
    voxel::world::VoxelWorld,
    world::biome_field::BiomeField,
};

const TARGET_ICON_SIZE: f32 = 46.0;

pub struct TargetHudPlugin;

impl Plugin for TargetHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_target_hud)
            .add_systems(
                Update,
                update_target_hud.run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct TargetHudRoot;

#[derive(Component)]
struct TargetBlockText;

#[derive(SystemParam)]
struct TargetHudContent<'w> {
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    world: Res<'w, VoxelWorld>,
}

fn spawn_target_hud(mut commands: Commands, mut icon_materials: ResMut<Assets<BlockIconMaterial>>) {
    let icon_material = icon_materials.add(BlockIconMaterial::empty());

    commands
        .spawn((
            TargetHudRoot,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                top: px(16),
                right: px(16),
                ..default()
            },
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn(surface::hud_panel()).with_children(|panel| {
                panel
                    .spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: px(10),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            BlockModelInstance::empty(),
                            MaterialNode(icon_material),
                            Node {
                                width: px(TARGET_ICON_SIZE),
                                height: px(TARGET_ICON_SIZE),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                        row.spawn((typography::hud(""), TargetBlockText));
                    });
            });
        });
}

fn update_target_hud(
    targeted: Res<TargetedBlock>,
    content: TargetHudContent,
    root_visibility: Single<&mut Visibility, With<TargetHudRoot>>,
    mut target_text: Single<&mut Text, With<TargetBlockText>>,
    mut icon: Single<(&mut BlockModelInstance, &MaterialNode<BlockIconMaterial>)>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let mut root_visibility = root_visibility.into_inner();

    let Some(hit) = targeted.0 else {
        if *root_visibility != Visibility::Hidden {
            *root_visibility = Visibility::Hidden;
        }
        return;
    };

    if *root_visibility != Visibility::Visible {
        *root_visibility = Visibility::Visible;
    }

    let block = content.blocks.get(hit.block_id);
    let block_name = block.map_or(hit.block_id, |block| block.name.as_str());
    let light_position = if hit.normal == IVec3::ZERO {
        hit.voxel + IVec3::Y
    } else {
        hit.voxel + hit.normal
    };
    let light = content.world.light_at(light_position);
    let light_level = light.sky().max(light.block());
    let next_text = format!(
        "{block_name}\nLight: {light_level}\nX: {} | Z: {} | Y: {}",
        hit.voxel.x, hit.voxel.z, hit.voxel.y
    );

    if target_text.0 != next_text {
        target_text.0 = next_text;
    }

    let (model, material_handle) = &mut *icon;
    let Some(mut material) = icon_materials.get_mut(&material_handle.0) else {
        return;
    };

    if model.set_block_id(Some(hit.block_id))
        && let Some(block) = block
    {
        material.set_block(block, &content.asset_server);
    }

    let tint_position = Vec2::new(hit.voxel.x as f32 + 0.5, hit.voxel.z as f32 + 0.5);
    material.set_tint(block_tint_at(
        hit.block_id,
        tint_position,
        &content.biome_field,
        &content.biomes,
    ));
}
