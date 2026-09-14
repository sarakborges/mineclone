use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        biome::BiomeRegistry, block::BlockRegistry,
        secondary_property::SecondaryPropertyRegistry,
    },
    hud::block_icon::BlockIconMaterial,
    localization::{ActiveLanguage, Language, UiLocalization},
    rendering::{
        block_model::BlockModel,
        block_tint::{apply_secondary_property_tint, block_tint_at},
    },
    targeting::block::TargetedBlock,
    ui::{surface, typography},
    voxel::{secondary_properties::SecondaryProperties, world::VoxelWorld},
    world::biome_field::BiomeField,
};

const TARGET_SLOT_SIZE: f32 = 44.0;
const TARGET_ICON_SIZE: f32 = 34.0;
const TARGET_CROSSHAIR_OFFSET: f32 = 62.0;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TargetHudSnapshot {
    voxel: IVec3,
    block_id: &'static str,
    normal: IVec3,
    properties: SecondaryProperties,
    light_level: u8,
    language: Language,
}

#[derive(SystemParam)]
struct TargetHudState<'w> {
    targeted: Res<'w, TargetedBlock>,
    world: Res<'w, VoxelWorld>,
    localization: Res<'w, UiLocalization>,
    language: Res<'w, ActiveLanguage>,
}

#[derive(SystemParam)]
struct TargetHudContent<'w> {
    asset_server: Res<'w, AssetServer>,
    blocks: Res<'w, BlockRegistry>,
    secondary_properties: Res<'w, SecondaryPropertyRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
}

#[derive(SystemParam)]
struct TargetHudView<'w, 's> {
    root_visibility: Single<'w, 's, &'static mut Visibility, With<TargetHudRoot>>,
    target_text: Single<'w, 's, &'static mut Text, With<TargetBlockText>>,
    icon: Single<
        'w,
        's,
        (&'static mut BlockModel, &'static MaterialNode<BlockIconMaterial>),
        With<TargetBlockModel>,
    >,
    icon_materials: ResMut<'w, Assets<BlockIconMaterial>>,
}

fn spawn_target_hud(mut commands: Commands, mut icon_materials: ResMut<Assets<BlockIconMaterial>>) {
    let icon_material = icon_materials.add(BlockIconMaterial::empty());
    let (slot_background, slot_border) = surface::hud_control_static(false);

    commands
        .spawn((
            TargetHudRoot,
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(10),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    position_type: PositionType::Relative,
                    bottom: px(TARGET_CROSSHAIR_OFFSET),
                    align_items: AlignItems::Center,
                    column_gap: px(10),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|row| {
                row.spawn((
                    Node {
                        width: px(TARGET_SLOT_SIZE),
                        height: px(TARGET_SLOT_SIZE),
                        min_width: px(TARGET_SLOT_SIZE),
                        min_height: px(TARGET_SLOT_SIZE),
                        border: UiRect::all(px(2)),
                        border_radius: BorderRadius::all(px(4)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(slot_background),
                    BorderColor::all(slot_border),
                    Pickable::IGNORE,
                ))
                .with_children(|slot| {
                    slot.spawn((
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
                });

                row.spawn((
                    typography::hud(""),
                    typography::tooltip_shadow(),
                    TargetBlockText,
                    Pickable::IGNORE,
                ));
            });
        });
}

fn update_target_hud(
    state: TargetHudState,
    content: TargetHudContent,
    view: TargetHudView,
    mut cached: Local<Option<TargetHudSnapshot>>,
) {
    let TargetHudView {
        root_visibility,
        target_text,
        icon,
        mut icon_materials,
    } = view;
    let root_visibility = root_visibility.into_inner();
    let target_text = target_text.into_inner();
    let (model, material_handle) = icon.into_inner();

    let Some(hit) = state.targeted.0 else {
        *cached = None;
        if *root_visibility != Visibility::Hidden {
            *root_visibility = Visibility::Hidden;
        }
        return;
    };

    let language = state.language.get();
    let cell = state.world.cell_at(hit.voxel);
    let properties = cell
        .map(|cell| cell.secondary_properties())
        .unwrap_or_default();
    let light_position = if hit.normal == IVec3::ZERO {
        hit.voxel + IVec3::Y
    } else {
        hit.voxel + hit.normal
    };
    let light = state.world.light_at(light_position);
    let light_level = light.sky().max(light.block());
    let snapshot = TargetHudSnapshot {
        voxel: hit.voxel,
        block_id: hit.block_id,
        normal: hit.normal,
        properties,
        light_level,
        language,
    };
    let definitions_changed = content.blocks.is_changed()
        || content.secondary_properties.is_changed()
        || content.biomes.is_changed()
        || content.biome_field.is_changed()
        || state.language.is_changed();

    if cached.as_ref() == Some(&snapshot)
        && !definitions_changed
        && *root_visibility == Visibility::Visible
    {
        return;
    }
    *cached = Some(snapshot);

    if *root_visibility != Visibility::Visible {
        *root_visibility = Visibility::Visible;
    }

    let block = content.blocks.get(hit.block_id);
    let block_name = block.map_or(hit.block_id, |block| block.name.text(language));
    let coordinates = state
        .localization
        .text(language, "hud.coordinates")
        .replace("{x}", &hit.voxel.x.to_string())
        .replace("{z}", &hit.voxel.z.to_string())
        .replace("{y}", &hit.voxel.y.to_string());
    let mut property_labels = properties
        .iter()
        .map(|(property, value)| {
            let property_name = match property {
                "dyed" => state.localization.text(language, "secondaryProperty.dyed"),
                _ => property,
            };
            let value_name = content
                .secondary_properties
                .get(property, value)
                .map_or(value, |definition| definition.name.text(language));
            format!("{property_name}: {value_name}")
        })
        .collect::<Vec<_>>();
    property_labels.sort();
    let properties_text = if property_labels.is_empty() {
        String::new()
    } else {
        format!("\n{}", property_labels.join("\n"))
    };
    let next_text = format!(
        "{block_name}{properties_text}\n{}: {light_level}\n{coordinates}",
        state.localization.text(language, "hud.light"),
    );

    if target_text.0 != next_text {
        target_text.0 = next_text;
    }

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
