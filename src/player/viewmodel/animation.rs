use std::f32::consts::PI;

use bevy::prelude::*;

use crate::{
    player::hotbar::PlayerHotbar,
    world::tick::WorldTickClock,
};

const BREAK_ANIMATION_DURATION_TICKS: u64 = 6;
const PLACE_ANIMATION_DURATION_TICKS: u64 = 9;
pub(super) const ITEM_SWITCH_ANIMATION_DURATION_TICKS: u64 = 12;

#[derive(Component)]
pub(super) struct PlayerViewModel;

#[derive(Clone, Copy)]
enum ViewModelAction {
    Break,
    Place,
}

#[derive(Resource, Default)]
pub(crate) struct ViewModelAnimation {
    action: Option<ViewModelAction>,
    elapsed_ticks: u64,
}

impl ViewModelAnimation {
    pub(crate) fn play_break(&mut self) {
        self.action = Some(ViewModelAction::Break);
        self.elapsed_ticks = 0;
    }

    pub(crate) fn play_place(&mut self) {
        self.action = Some(ViewModelAction::Place);
        self.elapsed_ticks = 0;
    }
}

#[derive(Resource, Default)]
pub(super) struct ViewModelItemSwitch {
    initialized: bool,
    displayed_block_id: Option<&'static str>,
    target_block_id: Option<&'static str>,
    elapsed_ticks: u64,
    active: bool,
}

impl ViewModelItemSwitch {
    pub(super) fn initialize(&mut self, block_id: Option<&'static str>) {
        self.initialized = true;
        self.displayed_block_id = block_id;
        self.target_block_id = block_id;
        self.elapsed_ticks = 0;
        self.active = false;
    }

    pub(super) fn displayed_block_id(&self) -> Option<&'static str> {
        self.displayed_block_id
    }

    pub(super) fn is_active(&self) -> bool {
        self.active
    }

    pub(super) fn elapsed_ticks(&self) -> u64 {
        self.elapsed_ticks
    }
}

pub(super) fn advance_item_switch(
    world_ticks: Res<WorldTickClock>,
    hotbar: Res<PlayerHotbar>,
    mut item_switch: ResMut<ViewModelItemSwitch>,
) {
    let selected_block_id = hotbar.item_at(hotbar.selected_slot());

    if !item_switch.initialized {
        item_switch.initialize(selected_block_id);
        return;
    }

    if selected_block_id != item_switch.target_block_id {
        item_switch.target_block_id = selected_block_id;
        item_switch.elapsed_ticks = 0;
        item_switch.active = true;
    }

    if !item_switch.active {
        return;
    }

    item_switch.elapsed_ticks = item_switch
        .elapsed_ticks
        .saturating_add(world_ticks.ticks_this_frame() as u64);
    let midpoint = ITEM_SWITCH_ANIMATION_DURATION_TICKS / 2;

    if item_switch.elapsed_ticks >= midpoint
        && item_switch.displayed_block_id != item_switch.target_block_id
    {
        item_switch.displayed_block_id = item_switch.target_block_id;
    }

    if item_switch.elapsed_ticks >= ITEM_SWITCH_ANIMATION_DURATION_TICKS {
        item_switch.displayed_block_id = item_switch.target_block_id;
        item_switch.elapsed_ticks = 0;
        item_switch.active = false;
    }
}

pub(super) fn animate_viewmodel(
    world_ticks: Res<WorldTickClock>,
    mut animation: ResMut<ViewModelAnimation>,
    item_switch: Res<ViewModelItemSwitch>,
    mut viewmodels: Query<&mut Transform, With<PlayerViewModel>>,
) {
    let interaction = animation.action.and_then(|action| {
        animation.elapsed_ticks = animation
            .elapsed_ticks
            .saturating_add(world_ticks.ticks_this_frame() as u64);
        let duration_ticks = match action {
            ViewModelAction::Break => BREAK_ANIMATION_DURATION_TICKS,
            ViewModelAction::Place => PLACE_ANIMATION_DURATION_TICKS,
        };
        let progress = (animation.elapsed_ticks as f32 / duration_ticks as f32).clamp(0.0, 1.0);

        if progress >= 1.0 {
            animation.action = None;
            animation.elapsed_ticks = 0;
            None
        } else {
            Some((action, (progress * PI).sin()))
        }
    });

    let switch_wave = if item_switch.is_active() {
        let progress = (item_switch.elapsed_ticks() as f32
            / ITEM_SWITCH_ANIMATION_DURATION_TICKS as f32)
            .clamp(0.0, 1.0);
        (progress * PI).sin()
    } else {
        0.0
    };

    for mut transform in &mut viewmodels {
        let mut animated = base_viewmodel_transform();

        if let Some((action, wave)) = interaction {
            match action {
                ViewModelAction::Break => {
                    animated.translation += Vec3::new(-0.03, 0.01, -0.14) * wave;
                    animated.rotation *=
                        Quat::from_euler(EulerRot::XYZ, -0.18 * wave, 0.05 * wave, -0.08 * wave);
                }
                ViewModelAction::Place => {
                    animated.translation += Vec3::new(-0.06, -0.10, -0.06) * wave;
                    animated.rotation *=
                        Quat::from_euler(EulerRot::XYZ, -0.68 * wave, 0.12 * wave, -0.34 * wave);
                }
            }
        }

        if switch_wave > 0.0 {
            animated.translation += Vec3::new(0.10, -0.54, 0.14) * switch_wave;
            animated.rotation *= Quat::from_euler(
                EulerRot::XYZ,
                0.34 * switch_wave,
                0.0,
                0.16 * switch_wave,
            );
        }

        *transform = animated;
    }
}

pub(super) fn base_viewmodel_transform() -> Transform {
    Transform::from_translation(Vec3::new(0.62, -0.80, -1.05)).with_rotation(Quat::from_euler(
        EulerRot::XYZ,
        -0.22,
        -0.10,
        0.28,
    ))
}
