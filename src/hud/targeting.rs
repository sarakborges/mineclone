use bevy::{ecs::system::SystemParam, prelude::*};
use smallvec::SmallVec;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::{
        block::{BlockDefinition, DEFAULT_BLOCK_BREAK_TICKS},
        builtin_ids::DYED_PROPERTY_ID,
        layer::{LayerFace, LayerRegistry},
        object::ObjectRegistry,
        secondary_property::SecondaryPropertyRegistry,
        tool_category::ToolCategoryRegistry,
    },
    hud::block_icon::BlockIconMaterial,
    localization::{ActiveLanguage, Language, UiLocalization},
    rendering::{
        block_model::BlockModel,
        block_tint::{apply_secondary_property_tint, block_tint_at},
        block_visual_content::BlockVisualContent,
    },
    targeting::{
        BlockMiningState,
        block::{BlockTargetingSet, TargetedBlock},
    },
    ui::{selectable, typography, visibility::set_visibility},
    voxel::{cell::VoxelCell, secondary_properties::SecondaryProperties, world::VoxelWorld},
    world_objects::{TargetedWorldObject, WorldObjectInstance},
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
            .add_systems(OnEnter(PauseState::Paused), set_visibility::<TargetHudRoot, false>.run_if(in_state(GameState::Gameplay)))
            .add_systems(OnEnter(PauseState::Running), set_visibility::<TargetHudRoot, true>.run_if(in_state(GameState::Gameplay)))
            .add_systems(
                Update,
                sync_target_hud_layout.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                update_target_hud
                    .after(BlockTargetingSet::Interaction)
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

#[derive(Component)]
struct TargetHudRoot;

#[derive(Component)]
struct TargetHudRow;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct AppliedTargetHudPosition(TargetBlockPosition);

type TargetHudRootLayout<'w, 's> = Single<
    'w,
    's,
    (&'static mut Node, &'static mut AppliedTargetHudPosition),
    (With<TargetHudRoot>, Without<TargetHudRow>),
>;

#[derive(Component)]
struct TargetBlockText;

#[derive(Component)]
struct TargetBlockModel;

#[derive(Component)]
struct TargetObjectIcon;

#[derive(Clone, Debug, PartialEq, Eq)]
struct TargetHudSnapshot {
    voxel: IVec3,
    block_id: &'static str,
    normal: IVec3,
    properties: SecondaryProperties,
    layers: SmallVec<[(LayerFace, &'static str); 8]>,
    light_level: u8,
    breaking_progress: Option<u8>,
    language: Language,
}

#[derive(Clone, Debug, PartialEq)]
struct TargetHudIconSnapshot {
    block_id: &'static str,
    tint: Color,
}

#[derive(Clone, Debug, PartialEq)]
struct TargetObjectHudSnapshot {
    support: IVec3,
    light_level: u8,
    language: Language,
    icon: String,
    tint: Color,
}

#[derive(SystemParam)]
struct TargetHudState<'w, 's> {
    targeted: Res<'w, TargetedBlock>,
    object_target: Res<'w, TargetedWorldObject>,
    object_instances: Query<'w, 's, &'static WorldObjectInstance>,
    world: Res<'w, VoxelWorld>,
    localization: Res<'w, UiLocalization>,
    language: Res<'w, ActiveLanguage>,
    settings: Res<'w, HudSettings>,
    mining: Res<'w, BlockMiningState>,
}

#[derive(SystemParam)]
struct TargetHudContent<'w> {
    visual: BlockVisualContent<'w>,
    objects: Res<'w, ObjectRegistry>,
    layers: Res<'w, LayerRegistry>,
    secondary_properties: Res<'w, SecondaryPropertyRegistry>,
    tool_categories: Res<'w, ToolCategoryRegistry>,
}

#[derive(SystemParam)]
struct TargetHudView<'w, 's> {
    root_visibility: Single<'w, 's, &'static mut Visibility, With<TargetHudRoot>>,
    target_text: Single<'w, 's, &'static mut Text, With<TargetBlockText>>,
    block_icon: Single<
        'w,
        's,
        (
            &'static mut BlockModel,
            &'static MaterialNode<BlockIconMaterial>,
            &'static mut Visibility,
        ),
        (With<TargetBlockModel>, Without<TargetObjectIcon>),
    >,
    object_icon: Single<
        'w,
        's,
        (&'static mut ImageNode, &'static mut Visibility),
        (With<TargetObjectIcon>, Without<TargetBlockModel>),
    >,
    icon_materials: ResMut<'w, Assets<BlockIconMaterial>>,
}

fn spawn_target_hud(
    mut commands: Commands,
    settings: Res<HudSettings>,
    mut icon_materials: ResMut<Assets<BlockIconMaterial>>,
) {
    let icon_material = icon_materials.add(BlockIconMaterial::empty());
    let (slot_background, slot_border) = selectable::static_colors(false);
    let position = settings.target_block_position();

    commands
        .spawn((
            TargetHudRoot,
            AppliedTargetHudPosition(position),
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
                    slot.spawn((
                        TargetObjectIcon,
                        ImageNode::default(),
                        Node {
                            width: px(TARGET_ICON_SIZE),
                            height: px(TARGET_ICON_SIZE),
                            ..default()
                        },
                        Visibility::Hidden,
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
    root: TargetHudRootLayout,
    mut row: Single<&mut Node, (With<TargetHudRow>, Without<TargetHudRoot>)>,
) {
    let position = settings.target_block_position();
    let (mut root_node, mut applied_position) = root.into_inner();
    if applied_position.0 == position {
        return;
    }

    *root_node = target_hud_root_node(position);
    **row = target_hud_row_node(position);
    applied_position.0 = position;
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
    mut cached_icon: Local<Option<TargetHudIconSnapshot>>,
    mut cached_object: Local<Option<TargetObjectHudSnapshot>>,
) {
    let TargetHudView {
        root_visibility,
        target_text,
        block_icon,
        object_icon,
        mut icon_materials,
    } = view;
    let mut root_visibility = root_visibility.into_inner();

    if state.settings.target_block_position() == TargetBlockPosition::Hidden {
        *cached = None;
        *cached_icon = None;
        *cached_object = None;
        if *root_visibility != Visibility::Hidden {
            *root_visibility = Visibility::Hidden;
        }
        return;
    }

    if let Some(entity) = state.object_target.0
        && let Ok(instance) = state.object_instances.get(entity)
        && let Some(object) = content.objects.get(instance.object_id())
    {
        let language = state.language.get();
        let support = instance.support();
        let light = state.world.light_at(support + IVec3::Y);
        let light_level = light.sky().max(light.block());
        let tint_position = Vec2::new(support.x as f32 + 0.5, support.z as f32 + 0.5);
        let tint = block_tint_at(
            object.tint,
            tint_position,
            &content.visual.biome_field,
            &content.visual.biomes,
        );
        let snapshot = TargetObjectHudSnapshot {
            support,
            light_level,
            language,
            icon: object.icon.clone(),
            tint,
        };
        let definitions_changed = content.objects.is_changed()
            || content.visual.biomes.is_changed()
            || content.visual.biome_field.is_changed()
            || state.language.is_changed();

        if cached_object.as_ref() == Some(&snapshot)
            && !definitions_changed
            && *root_visibility == Visibility::Visible
        {
            return;
        }
        if *root_visibility != Visibility::Visible {
            *root_visibility = Visibility::Visible;
        }

        let mut target_text = target_text.into_inner();
        let next_text = target_object_hud_text(&snapshot, object, &state);
        if target_text.0 != next_text {
            target_text.0 = next_text;
        }

        let (_, _, mut block_visibility) = block_icon.into_inner();
        if *block_visibility != Visibility::Hidden {
            *block_visibility = Visibility::Hidden;
        }
        let (mut image, mut object_visibility) = object_icon.into_inner();
        if *object_visibility != Visibility::Visible {
            *object_visibility = Visibility::Visible;
        }
        image.image = content.visual.asset_server.load(snapshot.icon.clone());
        image.color = snapshot.tint;

        *cached = None;
        *cached_icon = None;
        *cached_object = Some(snapshot);
        return;
    }

    let Some(hit) = state.targeted.0 else {
        *cached = None;
        *cached_icon = None;
        *cached_object = None;
        if *root_visibility != Visibility::Hidden {
            *root_visibility = Visibility::Hidden;
        }
        return;
    };
    *cached_object = None;

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
    let mut applied_layers = state
        .world
        .layers_at(hit.voxel)
        .iter()
        .map(|attached| (attached.face, attached.cell.layer_id))
        .collect::<SmallVec<[_; 8]>>();
    applied_layers.sort_unstable_by(|left, right| {
        left.1
            .cmp(right.1)
            .then_with(|| left.0.index().cmp(&right.0.index()))
    });
    applied_layers.dedup();

    let block = content.visual.blocks.get(hit.block_id);
    let breaking_progress = block
        .and_then(|block| {
            let required_work = DEFAULT_BLOCK_BREAK_TICKS as f32 * block.mining.hardness;
            state
                .mining
                .progress_for(hit.voxel, hit.block_id, required_work)
        })
        .map(|progress| (progress * 100.0).round().clamp(0.0, 100.0) as u8);

    let snapshot = TargetHudSnapshot {
        voxel: hit.voxel,
        block_id: hit.block_id,
        normal: hit.normal,
        properties,
        layers: applied_layers.clone(),
        light_level,
        breaking_progress,
        language,
    };
    let block_definitions_changed = content.visual.block_definitions_changed();
    let definitions_changed = content.visual.inputs_changed()
        || content.layers.is_changed()
        || content.secondary_properties.is_changed()
        || content.tool_categories.is_changed()
        || state.language.is_changed();

    if cached.as_ref() == Some(&snapshot)
        && !definitions_changed
        && *root_visibility == Visibility::Visible
    {
        return;
    }
    if *root_visibility != Visibility::Visible {
        *root_visibility = Visibility::Visible;
    }

    let mut target_text = target_text.into_inner();
    let next_text = target_hud_text(&snapshot, block, &state, &content);
    if target_text.0 != next_text {
        target_text.0 = next_text;
    }

    let (mut object_image, mut object_visibility) = object_icon.into_inner();
    if *object_visibility != Visibility::Hidden {
        *object_visibility = Visibility::Hidden;
    }
    object_image.color = Color::WHITE;

    let icon_snapshot = target_hud_icon_snapshot(&snapshot, cell, block, &content);
    *cached = Some(snapshot);
    let block_changed = cached_icon
        .as_ref()
        .is_none_or(|previous| previous.block_id != icon_snapshot.block_id);
    let tint_changed = cached_icon
        .as_ref()
        .is_none_or(|previous| previous.tint != icon_snapshot.tint);

    let (mut model, material_handle, mut block_visibility) = block_icon.into_inner();
    if *block_visibility != Visibility::Visible {
        *block_visibility = Visibility::Visible;
    }

    if !block_changed && !block_definitions_changed && !tint_changed {
        return;
    }

    let Some(mut material) = icon_materials.get_mut(&material_handle.0) else {
        return;
    };

    if (model.set_block_id(Some(hit.block_id)) || block_definitions_changed)
        && let Some(block) = block
    {
        material.set_block(block, &content.visual.asset_server);
    }
    if tint_changed {
        material.set_tint(icon_snapshot.tint);
    }
    *cached_icon = Some(icon_snapshot);
}

fn target_object_hud_text(
    snapshot: &TargetObjectHudSnapshot,
    object: &crate::content::object::ObjectDefinition,
    state: &TargetHudState<'_, '_>,
) -> String {
    let language = snapshot.language;
    let coordinates = state
        .localization
        .text(language, "hud.coordinates")
        .replace("{x}", &snapshot.support.x.to_string())
        .replace("{z}", &snapshot.support.z.to_string())
        .replace("{y}", &snapshot.support.y.to_string());

    format!(
        "{}\n{}: {}\n{coordinates}",
        object.name.text(language),
        state.localization.text(language, "hud.light"),
        snapshot.light_level,
    )
}

fn target_hud_text(
    snapshot: &TargetHudSnapshot,
    block: Option<&BlockDefinition>,
    state: &TargetHudState<'_, '_>,
    content: &TargetHudContent<'_>,
) -> String {
    let language = snapshot.language;
    let block_name = block.map_or(snapshot.block_id, |block| block.name.text(language));
    let coordinates = state
        .localization
        .text(language, "hud.coordinates")
        .replace("{x}", &snapshot.voxel.x.to_string())
        .replace("{z}", &snapshot.voxel.z.to_string())
        .replace("{y}", &snapshot.voxel.y.to_string());

    let mut property_labels = snapshot
        .properties
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

    let mut layer_lines = Vec::new();
    let mut index = 0;
    while index < snapshot.layers.len() {
        let layer_id = snapshot.layers[index].1;
        let layer_name = content
            .layers
            .get(layer_id)
            .map_or(layer_id, |layer| layer.name.text(language));
        let mut faces = Vec::new();
        while index < snapshot.layers.len() && snapshot.layers[index].1 == layer_id {
            faces.push(layer_face_name(
                &state.localization,
                language,
                snapshot.layers[index].0,
            ));
            index += 1;
        }
        layer_lines.push(format!("{layer_name}: {}", faces.join(", ")));
    }
    let layers_text = if layer_lines.is_empty() {
        String::new()
    } else {
        format!(
            "\n{}:\n{}",
            state.localization.text(language, "hud.layers"),
            layer_lines.join("\n")
        )
    };

    let mining_text = block.map_or_else(String::new, |block| {
        let mut lines = Vec::new();
        if !block.mining.required_tools.is_empty() {
            lines.push(format!(
                "{}: {}",
                state.localization.text(language, "hud.requiredTools"),
                tool_category_names(
                    &block.mining.required_tools,
                    &content.tool_categories,
                    language,
                ),
            ));
        }
        if !block.mining.preferred_tools.is_empty() {
            lines.push(format!(
                "{}: {}",
                state.localization.text(language, "hud.preferredTools"),
                tool_category_names(
                    &block.mining.preferred_tools,
                    &content.tool_categories,
                    language,
                ),
            ));
        }
        if lines.is_empty() {
            String::new()
        } else {
            format!("\n{}", lines.join("\n"))
        }
    });

    let breaking_text = snapshot
        .breaking_progress
        .map(|progress| {
            format!(
                "\n{}: {progress}%",
                state.localization.text(language, "hud.breakingProgress"),
            )
        })
        .unwrap_or_default();

    format!(
        "{block_name}{properties_text}{layers_text}{mining_text}{breaking_text}\n{}: {}\n{coordinates}",
        state.localization.text(language, "hud.light"),
        snapshot.light_level,
    )
}

fn target_hud_icon_snapshot(
    snapshot: &TargetHudSnapshot,
    cell: Option<VoxelCell>,
    block: Option<&BlockDefinition>,
    content: &TargetHudContent<'_>,
) -> TargetHudIconSnapshot {
    let tint_position =
        Vec2::new(snapshot.voxel.x as f32 + 0.5, snapshot.voxel.z as f32 + 0.5);
    let base_tint = content
        .visual
        .tint_at(snapshot.block_id, tint_position)
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

    TargetHudIconSnapshot {
        block_id: snapshot.block_id,
        tint,
    }
}

fn tool_category_names(
    category_ids: &[String],
    categories: &ToolCategoryRegistry,
    language: Language,
) -> String {
    category_ids
        .iter()
        .map(|category_id| {
            categories
                .get(category_id)
                .map_or(category_id.as_str(), |category| category.name.text(language))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn layer_face_name(
    localization: &UiLocalization,
    language: Language,
    face: LayerFace,
) -> &str {
    let key = match face {
        LayerFace::Right => "hud.layerFace.right",
        LayerFace::Left => "hud.layerFace.left",
        LayerFace::Top => "hud.layerFace.top",
        LayerFace::Bottom => "hud.layerFace.bottom",
        LayerFace::Front => "hud.layerFace.front",
        LayerFace::Back => "hud.layerFace.back",
    };
    localization.text(language, key)
}
