use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry, block::BlockRegistry,
        secondary_property::SecondaryPropertyRegistry,
    },
    hud::block_icon::BlockIconMaterial,
    localization::{ActiveLanguage, UiLocalization},
    rendering::{
        block_model::BlockModel,
        block_tint::{apply_secondary_property_tint, block_tint_at},
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

#[derive(Component)]
struct TargetBlockModel;

#[derive(SystemParam)]
struct TargetHudContent<'w> {
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    secondary_properties: Res<'w, SecondaryPropertyRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    world: Res<'w, VoxelWorld>,
    localization: Res<'w, UiLocalization>,
    language: Res<'w, ActiveLanguage>,
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
                            TargetBlockModel,
                            BlockModel::empty_display(),
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
    mut icon: Single<
        (&mut BlockModel, &MaterialNode<BlockIconMaterial>),
        With<TargetBlockModel>,
    >,
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

    let language = content.language.get();
    let block = content.blocks.get(hit.block_id);
    let block_name = block.map_or(hit.block_id, |block| block.name.text(language));
    let cell = content.world.cell_at(hit.voxel);
    let light_position = if hit.normal == IVec3::ZERO {
        hit.voxel + IVec3::Y
    } else {
        hit.voxel + hit.normal
    };
    let light = content.world.light_at(light_position);
    let light_level = light.sky().max(light.block());
    let coordinates = content
        .localization
        .text(language, "hud.coordinates")
        .replace("{x}", &hit.voxel.x.to_string())
        .replace("{z}", &hit.voxel.z.to_string())
        .replace("{y}", &hit.voxel.y.to_string());
    let properties = cell
        .map(|cell| {
            let mut properties = cell
                .secondary_properties()
                .iter()
                .map(|(property, value)| {
                    let value_name = content
                        .secondary_properties
                        .get(property, value)
                        .map_or(value, |definition| definition.name.text(language));
                    format!("{property}: {value_name}")
                })
                .collect::<Vec<_>>();
            properties.sort();
            properties
        })
        .unwrap_or_default();
    let properties_text = if properties.is_empty() {
        String::new()
    } else {
        format!("\n{}", properties.join("\n"))
    };
    let next_text = format!(
        "{block_name}\n{}: {light_level}\n{coordinates}{properties_text}",
        content.localization.text(language, "hud.light"),
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
    let base_tint = block_tint_at(
        block.map(|block| block.tint).unwrap_or_default(),
        tint_position,
        &content.biome_field,
        &content.biomes,
    );
    let tint = match (block, cell) {
        (Some(block), Some(cell)) => apply_secondary_property_tint(
            base_tint,
            block,
            cell,
            &content.secondary_properties,
        ),
        _ => base_tint,
    };
    material.set_tint(tint);
}
