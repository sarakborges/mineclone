mod animation;
mod model;

use bevy::prelude::*;

use crate::{
    app::{game_state::GameState, pause_state::PauseState, resource_systems::reset_resource},
    content::{
        item::ItemRegistry,
        secondary_property::SecondaryPropertyRegistry,
        tool::ToolRegistry,
    },
    player::{
        camera::CameraPerspective,
        held_sprite::{
            HeldSpriteMesh, setup_held_sprite_mesh, spawn_held_sprite, sync_held_sprites,
        },
        hotbar::PlayerHotbar,
    },
    targeting::block::BlockTargetingSet,
    tools::BrushMode,
};

pub(crate) use animation::ViewModelAnimation;
use animation::{
    PlayerViewModel, ViewModelItemSwitch, advance_item_switch, advance_viewmodel_animation,
    animate_viewmodel,
};
use model::{
    VIEW_MODEL_ARM_GRIP_Y, VIEW_MODEL_RENDER_LAYER, attach_viewmodel_arm_model, spawn_viewmodel,
    sync_held_block,
};

pub struct PlayerViewModelPlugin;

impl Plugin for PlayerViewModelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewModelAnimation>()
            .init_resource::<ViewModelItemSwitch>()
            .add_systems(
                OnEnter(GameState::Gameplay),
                (
                    reset_resource::<ViewModelAnimation>,
                    reset_resource::<ViewModelItemSwitch>,
                    setup_held_sprite_mesh,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    spawn_viewmodel,
                    attach_viewmodel_arm_model,
                    spawn_first_person_held_sprite,
                    advance_item_switch,
                    sync_held_block,
                    sync_held_sprites,
                    animate_viewmodel,
                    sync_viewmodel_visibility,
                )
                    .chain()
                    .after(BlockTargetingSet::Interaction)
                    .run_if(in_state(GameState::Gameplay)),
            )
            .add_systems(
                PostUpdate,
                advance_viewmodel_animation.run_if(in_state(GameState::Gameplay)),
            );
    }
}


fn sync_viewmodel_visibility(
    perspective: Res<CameraPerspective>,
    pause: Res<State<PauseState>>,
    mut viewmodels: Query<&mut Visibility, With<PlayerViewModel>>,
) {
    let next_visibility =
        if !perspective.is_third_person() && *pause.get() == PauseState::Running {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

    for mut visibility in &mut viewmodels {
        if *visibility != next_visibility {
            *visibility = next_visibility;
        }
    }
}


fn spawn_first_person_held_sprite(
    mut commands: Commands,
    viewmodels: Query<Entity, Added<PlayerViewModel>>,
    hotbar: Res<PlayerHotbar>,
    items: Res<ItemRegistry>,
    tools: Res<ToolRegistry>,
    brush_mode: Res<BrushMode>,
    properties: Res<SecondaryPropertyRegistry>,
    asset_server: Res<AssetServer>,
    mesh: Res<HeldSpriteMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let rotation = animation::base_viewmodel_transform().rotation.inverse();
    let root_transform =
        Transform::from_translation(Vec3::new(-0.08, VIEW_MODEL_ARM_GRIP_Y, 0.21))
            .with_rotation(rotation);

    for viewmodel in &viewmodels {
        commands.entity(viewmodel).with_children(|hand| {
            spawn_held_sprite(
                hand,
                root_transform,
                bevy::camera::visibility::RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                &hotbar,
                &items,
                &tools,
                &brush_mode,
                &properties,
                &asset_server,
                &mesh.0,
                &mut materials,
            );
        });
    }
}
