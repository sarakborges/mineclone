use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    app::game_state::GameState,
    content::{
        builtin_ids::DYED_PROPERTY_ID,
        secondary_property::SecondaryPropertyRegistry,
    },
    hud::block_icon::BlockIconMaterial,
    localization::{ActiveLanguage, Language, UiLocalization},
    rendering::{
        block_model::BlockModel,
        block_tint::apply_secondary_property_tint,
        block_visual_content::BlockVisualContent,
    },
    targeting::block::TargetedBlock,
    ui::{surface, typography},
    voxel::{secondary_properties::SecondaryProperties, world::VoxelWorld},
};

use super::{HudSettings, TargetBlockPosition};

const TARGET_SLOT_SIZE: f32 = 44.0;
const TARGET_ICON_SIZE: f32 = 34.0;
const TARGET_CROSSHAIR_OFFSET: f32 = 62.0;
const TARGET_CORNER_MARGIN: f32 = 18.0;

pub struct TargetHudPlugin;

impl Plugin for TargetHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Gameplay), spawn_target_hud)
            .add_systems(
                Update,
                (sync_target_hud_layout, update_target_hud)
                    .chain()
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct TargetHudRoot;

#[derive(Component)]
struct TargetHudRow;

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
    settings: Res<'w, HudSettings>,
}

#[derive(SystemParam)]
struct TargetHudContent<'w> {
    visual: BlockVisualContent<'w>,
    secondary_properties: Res<'w, SecondaryPropertyRegistry>,
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

fn spawn_target_hud(
    mut commands: Commands,
    settings: Res<HudSettings>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let icon_material = icon_materials.add(BlockIconMaterial::empty());
    let (slot_background, slot_border) = surface::hud_control_static(false);
    let position = settings.target_block_position();

    commands
        .spawn((
            TargetHudRoot,
            Visibility::Hidden,
            target_hud_root_node(position),
            GlobalZIndex(10),
            Pickable::IGNORE,
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                TargetHudRow,
                target_hud_row_node(position),
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

fn sync_target_hud_layout(
    settings: Res<HudSettings>,
    mut root: Single<&mut Node, (With<TargetHudRoot>, Without<TargetHudRow>)>,
    mut row: Single<&mut Node, (With<TargetHudRow>, Without<TargetHudRoot>)>,
    mut cached_position: Local<Option<TargetBlockPosition>>,
) {
    let position = settings.target_block_position();
    if *cached_position == Some(position) {
        return;
    }
    *cached_position = Some(position);
    **root = target_hud_root_node(position);
    **row = target_hud_row_node(position);
}

fn target_hud_root_node(position: TargetBlockPosition) -> Node {
    match position {
        TargetBlockPosition::Center | TargetBlockPosition::Hidden => Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        TargetBlockPosition::TopRight => Node {
            position_type: PositionType::Absolute,
            right: px(TARGET_CORNER_MARGIN),
            top: px(TARGET_CORNER_MARGIN),
            ..default()
        },
    }
}

fn target_hud_row_node(position: TargetBlockPosition) -> Node {
    Node {
        position_type: PositionType::Relative,
        bottom: if position == TargetBlockPosition::Center {
            px(TARGET_CROSSHAIR_OFFSET)
        } else {
            Val::Auto
        },
        align_items: AlignItems::Center,
        column_gap: px(10),
        ..default()
    }
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
    let mut root_visibility = root_visibility.into_inner();

    if state.settings.target_block_position() == TargetBlockPosition::Hidden {
        if *root_visibility != Visibility::Hidden {
            *root_visibility = Visibility::Hidden;
        }
        return;
    }

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
    let block_definitions_changed = content.visual.block_definitions_changed();
    let definitions_changed = content.visual.inputs_changed()
        || content.secondary_properties.is_changed()
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

    let mut target_text = target_text.into_inner();
    let (mut model, material_handle) = icon.into_inner();
    let block = content.visual.blocks.get(hit.block_id);
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
                DYED_PROPERTY_ID => state.localization.text(language, "secondaryProperty.dyed"),
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

    if (model.set_block_id(Some(hit.block_id)) || block_definitions_changed)
        && let Some(block) = block
    {
        material.set_block(block, &content.visual.asset_server);
    }

    let tint_position = Vec2::new(hit.voxel.x as f32 + 0.5, hit.voxel.z as f32 + 0.5);
    let base_tint = content
        .visual
        .tint_at(hit.block_id, tint_position)
        .unwrap_or(Color::WHITE);
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
