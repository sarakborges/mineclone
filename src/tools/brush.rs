use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState},
    content::{
        block::BlockRegistry, builtin_ids::BRUSH_TOOL_ID,
        secondary_property::SecondaryPropertyRegistry,
    },
    localization::{ActiveLanguage, UiLocalization},
    player::inventory::InventoryState,
    targeting::{ToolUse, ToolUseButton, block::BlockTargetingSet},
    ui::{theme, typography},
    voxel::{
        lighting::PendingLightingUpdates, neighbors::CARDINAL_NEIGHBORS, world::VoxelWorld,
    },
    world::chunk_remesh::ChunkRemeshQueue,
};

pub(crate) const DYED_PROPERTY_ID: &str = "dyed";

const DEFAULT_DYE_ID: &str = "red";
const PALETTE_COLUMNS: usize = 9;
const SWATCH_SIZE: f32 = 30.0;
const SWATCH_GAP: f32 = 4.0;
const PALETTE_WIDTH: f32 = PALETTE_COLUMNS as f32 * SWATCH_SIZE
    + (PALETTE_COLUMNS - 1) as f32 * SWATCH_GAP;

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum BrushPaletteState {
    #[default]
    Closed,
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BrushSelection {
    Clear,
    Dye(String),
}

#[derive(Resource, Clone, Debug)]
pub(crate) struct BrushMode {
    selection: BrushSelection,
}

impl Default for BrushMode {
    fn default() -> Self {
        Self {
            selection: BrushSelection::Dye(DEFAULT_DYE_ID.to_owned()),
        }
    }
}

impl BrushMode {
    pub(crate) fn dye_id(&self) -> Option<&str> {
        match &self.selection {
            BrushSelection::Clear => None,
            BrushSelection::Dye(id) => Some(id),
        }
    }
}

#[derive(Component)]
struct BrushPaletteRoot;

#[derive(Component, Clone)]
struct BrushPaletteChoice {
    selection: BrushSelection,
}

pub(super) struct BrushPlugin;

impl Plugin for BrushPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<BrushPaletteState>()
            .init_resource::<BrushMode>()
            .add_systems(
                Update,
                handle_brush_use
                    .after(BlockTargetingSet::Interaction)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(PauseState::Running))
                    .run_if(in_state(InventoryState::Closed))
                    .run_if(in_state(BrushPaletteState::Closed)),
            )
            .add_systems(
                OnEnter(BrushPaletteState::Open),
                spawn_brush_palette.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                Update,
                (handle_palette_selection, close_palette_with_escape)
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(BrushPaletteState::Open)),
            )
            .add_systems(
                OnEnter(PauseState::Paused),
                close_brush_palette.run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(OnExit(GameState::Gameplay), close_brush_palette);
    }
}

fn handle_brush_use(
    mut uses: MessageReader<ToolUse>,
    blocks: Res<BlockRegistry>,
    mode: Res<BrushMode>,
    mut world: ResMut<VoxelWorld>,
    mut lighting: ResMut<PendingLightingUpdates>,
    mut remesh_queue: ResMut<ChunkRemeshQueue>,
    mut next_palette: ResMut<NextState<BrushPaletteState>>,
) {
    for usage in uses.read() {
        if usage.tool_id != BRUSH_TOOL_ID {
            continue;
        }

        if usage.button == ToolUseButton::Right {
            next_palette.set(BrushPaletteState::Open);
            continue;
        }

        let Some(hit) = usage.target else {
            continue;
        };
        let Some(block) = blocks.get(hit.block_id) else {
            continue;
        };
        if !block
            .secondary_properties
            .iter()
            .any(|property| property == DYED_PROPERTY_ID)
        {
            continue;
        }

        let Some(cell) = world.cell_at(hit.voxel) else {
            continue;
        };
        let current_dye = cell.secondary_property(DYED_PROPERTY_ID);
        let updated = match mode.dye_id() {
            Some(dye_id) => {
                if current_dye == Some(dye_id) {
                    continue;
                }
                cell.with_secondary_property(DYED_PROPERTY_ID, dye_id)
            }
            None => {
                if current_dye.is_none() {
                    continue;
                }
                cell.without_secondary_property(DYED_PROPERTY_ID)
            }
        };

        if let Some(chunk) = world.set_block_at(hit.voxel, Some(updated)) {
            lighting.enqueue_voxel_edit(hit.voxel);
            remesh_queue.enqueue_priority(chunk);
            for offset in CARDINAL_NEIGHBORS {
                remesh_queue.enqueue_priority(chunk + offset);
            }
        }
    }
}

fn spawn_brush_palette(
    mut commands: Commands,
    properties: Res<SecondaryPropertyRegistry>,
    mode: Res<BrushMode>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    let language = language.get();
    let mut colors = properties.iter(DYED_PROPERTY_ID).collect::<Vec<_>>();
    colors.sort_by(|left, right| left.id.cmp(&right.id));

    commands
        .spawn((
            BrushPaletteRoot,
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
            GlobalZIndex(120),
            Pickable::IGNORE,
            DespawnOnExit(BrushPaletteState::Open),
            DespawnOnExit(GameState::Gameplay),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(8),
                    padding: UiRect::all(px(10)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(9)),
                    ..default()
                },
                BackgroundColor(theme::FROSTED_SURFACE),
                theme::frosted_surface_gradient(),
                BorderColor::all(Color::srgba(0.70, 0.72, 0.92, 0.20)),
                Pickable::IGNORE,
            ))
            .with_children(|panel| {
                panel.spawn((
                    typography::caption(localization.text(language, "brush.palette.title")),
                    Pickable::IGNORE,
                ));

                let clear_selected = mode.dye_id().is_none();
                panel
                    .spawn((
                        Button,
                        BrushPaletteChoice {
                            selection: BrushSelection::Clear,
                        },
                        Node {
                            width: px(PALETTE_WIDTH),
                            height: px(30),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(6)),
                            ..default()
                        },
                        BackgroundColor(theme::HUD_SURFACE),
                        BorderColor::all(if clear_selected {
                            theme::TEXT_PRIMARY
                        } else {
                            Color::srgba(0.70, 0.72, 0.82, 0.28)
                        }),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            typography::caption(localization.text(language, "brush.palette.clear")),
                            Pickable::IGNORE,
                        ));
                    });

                panel
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(SWATCH_GAP),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .with_children(|grid| {
                        for row in colors.chunks(PALETTE_COLUMNS) {
                            grid.spawn((
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    column_gap: px(SWATCH_GAP),
                                    ..default()
                                },
                                Pickable::IGNORE,
                            ))
                            .with_children(|row_node| {
                                for color in row {
                                    let selected = mode.dye_id() == Some(color.id.as_str());
                                    row_node.spawn((
                                        Button,
                                        BrushPaletteChoice {
                                            selection: BrushSelection::Dye(color.id.clone()),
                                        },
                                        Node {
                                            width: px(SWATCH_SIZE),
                                            height: px(SWATCH_SIZE),
                                            border: UiRect::all(px(2)),
                                            border_radius: BorderRadius::all(px(5)),
                                            ..default()
                                        },
                                        BackgroundColor(color.color.to_color()),
                                        BorderColor::all(if selected {
                                            theme::TEXT_PRIMARY
                                        } else {
                                            Color::srgba(0.70, 0.72, 0.82, 0.32)
                                        }),
                                    ));
                                }
                            });
                        }
                    });
            });
        });
}

fn handle_palette_selection(
    choices: Query<(&Interaction, &BrushPaletteChoice), Changed<Interaction>>,
    mut mode: ResMut<BrushMode>,
    mut next_palette: ResMut<NextState<BrushPaletteState>>,
) {
    for (interaction, choice) in &choices {
        if *interaction != Interaction::Pressed {
            continue;
        }

        mode.selection = choice.selection.clone();
        next_palette.set(BrushPaletteState::Closed);
        break;
    }
}

fn close_palette_with_escape(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_palette: ResMut<NextState<BrushPaletteState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_palette.set(BrushPaletteState::Closed);
    }
}

fn close_brush_palette(mut next_palette: ResMut<NextState<BrushPaletteState>>) {
    next_palette.set(BrushPaletteState::Closed);
}
