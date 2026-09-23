use bevy::{prelude::*, ui_widgets::ScrollArea};

use crate::{
    app::{
        controls_state::ControlsState,
        keybinds::{KeybindAction, Keybinds},
    },
    localization::{ActiveLanguage, Language, UiLocalization},
    ui::{
        button::{button, ButtonVariant, COMPACT_CONTROL_HEIGHT},
        cosmic_background::{self, STAR_FIELD},
        screen, surface, theme,
        transition::{ScreenTransition, ScreenTransitionTarget},
        typography,
    },
};

const SECTION_GAP: f32 = 18.0;
const COLUMN_GAP: f32 = 18.0;
const CARD_GAP: f32 = 14.0;
const ENTRY_GAP: f32 = 10.0;
const KEY_WIDTH: f32 = 92.0;
const KEY_HEIGHT: f32 = 36.0;

pub struct ControlsScreenPlugin;

impl Plugin for ControlsScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ControlsState::Open), spawn_controls_screen)
            .add_systems(
                Update,
                handle_controls_close.run_if(in_state(ControlsState::Open)),
            );
    }
}

#[derive(Component)]
struct ControlsBackButton;

#[derive(Clone, Copy)]
enum ControlBinding {
    Fixed(&'static str),
    Editable(KeybindAction),
    EditableDouble(KeybindAction),
}

#[derive(Clone, Copy)]
struct ControlSpec {
    binding: ControlBinding,
    action_key: &'static str,
}

const MOVEMENT_CONTROLS: &[ControlSpec] = &[
    ControlSpec {
        binding: ControlBinding::Fixed("WASD"),
        action_key: "controls.movement",
    },
    ControlSpec {
        binding: ControlBinding::Fixed("W ×2"),
        action_key: "controls.run",
    },
    ControlSpec {
        binding: ControlBinding::Editable(KeybindAction::Jump),
        action_key: "settings.keybind.jump",
    },
    ControlSpec {
        binding: ControlBinding::EditableDouble(KeybindAction::Jump),
        action_key: "controls.toggleFlight",
    },
    ControlSpec {
        binding: ControlBinding::Editable(KeybindAction::Descend),
        action_key: "settings.keybind.descend",
    },
    ControlSpec {
        binding: ControlBinding::Fixed("MOUSE"),
        action_key: "controls.look",
    },
];

const INTERFACE_CONTROLS: &[ControlSpec] = &[
    ControlSpec {
        binding: ControlBinding::Fixed("1–9"),
        action_key: "controls.hotbar",
    },
    ControlSpec {
        binding: ControlBinding::Editable(KeybindAction::Inventory),
        action_key: "settings.keybind.inventory",
    },
    ControlSpec {
        binding: ControlBinding::Fixed("ESC"),
        action_key: "controls.pauseClose",
    },
    ControlSpec {
        binding: ControlBinding::Editable(KeybindAction::ChangePerspective),
        action_key: "settings.keybind.changePerspective",
    },
];

const ACTION_CONTROLS: &[ControlSpec] = &[
    ControlSpec {
        binding: ControlBinding::Fixed("LMB"),
        action_key: "controls.primaryAction",
    },
    ControlSpec {
        binding: ControlBinding::Fixed("RMB"),
        action_key: "controls.secondaryAction",
    },
    ControlSpec {
        binding: ControlBinding::Fixed("MMB"),
        action_key: "controls.pickBlock",
    },
    ControlSpec {
        binding: ControlBinding::Editable(KeybindAction::ToolAction),
        action_key: "settings.keybind.toolAction",
    },
];

const CHAT_CONTROLS: &[ControlSpec] = &[
    ControlSpec {
        binding: ControlBinding::Editable(KeybindAction::Chat),
        action_key: "settings.keybind.chat",
    },
    ControlSpec {
        binding: ControlBinding::Fixed("ENTER"),
        action_key: "controls.sendChat",
    },
    ControlSpec {
        binding: ControlBinding::Fixed("↑ / ↓"),
        action_key: "controls.chatHistory",
    },
    ControlSpec {
        binding: ControlBinding::Fixed("TAB"),
        action_key: "controls.autocomplete",
    },
];

fn spawn_controls_screen(
    mut commands: Commands,
    keybinds: Res<Keybinds>,
    localization: Res<UiLocalization>,
    language: Res<ActiveLanguage>,
) {
    let language = language.get();

    commands
        .spawn((
            DespawnOnExit(ControlsState::Open),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(theme::SCREEN_BACKGROUND),
            theme::cosmic_background_gradient(),
            GlobalZIndex(2_000),
        ))
        .with_children(|root| {
            for &spec in STAR_FIELD {
                root.spawn(cosmic_background::star(spec));
            }

            root.spawn(screen::header()).with_children(|header| {
                header.spawn(typography::title(
                    localization.text(language, "controls.title").to_owned(),
                ));
            });

            root.spawn(screen::body()).with_children(|body| {
                let mut content = screen::content_column(SECTION_GAP);
                content.overflow = Overflow::scroll_y();
                content.padding = UiRect::right(px(12));

                body.spawn((
                    content,
                    ScrollPosition(Vec2::ZERO),
                    ScrollArea,
                ))
                .with_children(|sections| {
                    spawn_control_section(
                        sections,
                        "controls.category.movement",
                        MOVEMENT_CONTROLS,
                        &keybinds,
                        &localization,
                        language,
                    );
                    spawn_control_section(
                        sections,
                        "controls.category.interface",
                        INTERFACE_CONTROLS,
                        &keybinds,
                        &localization,
                        language,
                    );
                    spawn_control_section(
                        sections,
                        "controls.category.actions",
                        ACTION_CONTROLS,
                        &keybinds,
                        &localization,
                        language,
                    );
                    spawn_control_section(
                        sections,
                        "controls.category.chat",
                        CHAT_CONTROLS,
                        &keybinds,
                        &localization,
                        language,
                    );
                });
            });

            root.spawn(screen::footer()).with_children(|footer| {
                footer.spawn(button(
                    localization.text(language, "settings.return").to_owned(),
                    ControlsBackButton,
                    px(360),
                    COMPACT_CONTROL_HEIGHT,
                    ButtonVariant::Normal,
                ));
            });
        });
}

fn spawn_control_section(
    parent: &mut ChildSpawnerCommands,
    title_key: &'static str,
    controls: &[ControlSpec],
    keybinds: &Keybinds,
    localization: &UiLocalization,
    language: Language,
) {
    parent
        .spawn(surface::settings_content())
        .with_children(|card| {
            card.spawn((
                typography::heading(localization.text(language, title_key).to_owned()),
                Node {
                    margin: UiRect::bottom(px(CARD_GAP)),
                    ..default()
                },
            ));

            card.spawn(Node {
                display: Display::Grid,
                width: percent(100),
                grid_template_columns: vec![RepeatedGridTrack::flex(4, 1.0)],
                column_gap: px(COLUMN_GAP),
                row_gap: px(ENTRY_GAP),
                ..default()
            })
            .with_children(|grid| {
                for &control in controls {
                    spawn_control_entry(grid, control, keybinds, localization, language);
                }
            });
        });
}

fn spawn_control_entry(
    parent: &mut ChildSpawnerCommands,
    control: ControlSpec,
    keybinds: &Keybinds,
    localization: &UiLocalization,
    language: Language,
) {
    parent
        .spawn(Node {
            width: percent(100),
            min_height: px(KEY_HEIGHT),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(ENTRY_GAP),
            ..default()
        })
        .with_children(|entry| {
            entry
                .spawn((
                    Node {
                        width: px(KEY_WIDTH),
                        min_width: px(KEY_WIDTH),
                        height: px(KEY_HEIGHT),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BackgroundColor(theme::SURFACE_INSET),
                    BorderColor::all(theme::BORDER),
                ))
                .with_child(typography::button_label_light(binding_label(
                    control.binding,
                    keybinds,
                )));

            entry
                .spawn(Node {
                    flex_grow: 1.0,
                    min_width: px(0),
                    ..default()
                })
                .with_child(typography::muted(
                    localization.text(language, control.action_key).to_owned(),
                ));
        });
}

fn binding_label(binding: ControlBinding, keybinds: &Keybinds) -> String {
    match binding {
        ControlBinding::Fixed(label) => label.to_owned(),
        ControlBinding::Editable(action) => keybinds.label(action).to_owned(),
        ControlBinding::EditableDouble(action) => format!("{} ×2", keybinds.label(action)),
    }
}

fn handle_controls_close(
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<&Interaction, (Changed<Interaction>, With<ControlsBackButton>)>,
    mut transition: ResMut<ScreenTransition>,
) {
    if transition.is_active() {
        return;
    }

    let back_pressed = interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    if back_pressed || keys.just_pressed(KeyCode::Escape) {
        transition.request(ScreenTransitionTarget::controls(ControlsState::Closed));
    }
}
