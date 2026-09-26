use std::f32::consts::PI;

use bevy::prelude::*;

use crate::{
    player::hotbar::PlayerHotbar,
    world::tick::WorldTickClock,
};

const BREAK_ANIMATION_DURATION_TICKS: u64 = 22;
const FAST_BREAK_ANIMATION_DURATION_TICKS: u64 = 8;
const HIT_ANIMATION_DURATION_TICKS: u64 = 17;
const PLACE_ANIMATION_DURATION_TICKS: u64 = 19;
pub(super) const ITEM_SWITCH_ANIMATION_DURATION_TICKS: u64 = 12;

#[derive(Component)]
pub(super) struct PlayerViewModel;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewModelAction {
    Break,
    Hit,
    Place,
}

#[derive(Resource, Default)]
pub(crate) struct ViewModelAnimation {
    action: Option<ViewModelAction>,
    elapsed_ticks: u64,
    duration_ticks: u64,
    playback_speed: f32,
    revision: u64,
}

impl ViewModelAnimation {
    pub(crate) fn play_break(&mut self) {
        self.play(ViewModelAction::Break, BREAK_ANIMATION_DURATION_TICKS, 1.0);
    }

    pub(crate) fn play_break_fast(&mut self) {
        self.play(
            ViewModelAction::Break,
            FAST_BREAK_ANIMATION_DURATION_TICKS,
            BREAK_ANIMATION_DURATION_TICKS as f32 / FAST_BREAK_ANIMATION_DURATION_TICKS as f32,
        );
    }

    pub(crate) fn play_hit(&mut self) {
        self.play(ViewModelAction::Hit, HIT_ANIMATION_DURATION_TICKS, 1.0);
    }

    pub(crate) fn play_place(&mut self) {
        self.play(ViewModelAction::Place, PLACE_ANIMATION_DURATION_TICKS, 1.0);
    }

    fn play(&mut self, action: ViewModelAction, duration_ticks: u64, playback_speed: f32) {
        self.action = Some(action);
        self.elapsed_ticks = 0;
        self.duration_ticks = duration_ticks;
        self.playback_speed = playback_speed;
        self.revision = self.revision.wrapping_add(1);
    }

    pub(crate) fn action_name(&self) -> Option<&'static str> {
        match self.action {
            Some(ViewModelAction::Break) => Some("break"),
            Some(ViewModelAction::Hit) => Some("hit"),
            Some(ViewModelAction::Place) => Some("place"),
            None => None,
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn playback_speed(&self) -> f32 {
        self.playback_speed.max(f32::EPSILON)
    }

    fn progress(&self) -> Option<(ViewModelAction, f32)> {
        let action = self.action?;
        let duration_ticks = self.duration_ticks.max(1);
        let progress = (self.elapsed_ticks as f32 / duration_ticks as f32).clamp(0.0, 1.0);
        Some((action, progress))
    }
}

#[derive(Resource, Default)]
pub(super) struct ViewModelItemSwitch {
    initialized: bool,
    observed_item_id: Option<&'static str>,
    elapsed_ticks: u64,
    active: bool,
}

impl ViewModelItemSwitch {
    pub(super) fn initialize(&mut self, block_id: Option<&'static str>) {
        self.initialized = true;
        self.observed_item_id = block_id;
        self.elapsed_ticks = 0;
        self.active = false;
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
    let selected_item_id = hotbar.item_at(hotbar.selected_slot());

    if !item_switch.initialized {
        item_switch.initialize(selected_item_id);
        return;
    }

    if selected_item_id != item_switch.observed_item_id {
        item_switch.observed_item_id = selected_item_id;
        item_switch.elapsed_ticks = 0;
        item_switch.active = true;
    }

    if !item_switch.active {
        return;
    }

    item_switch.elapsed_ticks = item_switch
        .elapsed_ticks
        .saturating_add(world_ticks.ticks_this_frame() as u64);

    if item_switch.elapsed_ticks >= ITEM_SWITCH_ANIMATION_DURATION_TICKS {
        item_switch.elapsed_ticks = 0;
        item_switch.active = false;
    }
}

pub(super) fn advance_viewmodel_animation(
    world_ticks: Res<WorldTickClock>,
    mut animation: ResMut<ViewModelAnimation>,
) {
    if animation.action.is_none() {
        return;
    }
    animation.elapsed_ticks = animation
        .elapsed_ticks
        .saturating_add(world_ticks.ticks_this_frame() as u64);
    if animation.elapsed_ticks >= animation.duration_ticks.max(1) {
        animation.action = None;
        animation.elapsed_ticks = 0;
        animation.duration_ticks = 0;
        animation.playback_speed = 1.0;
    }
}

pub(super) fn animate_viewmodel(
    animation: Res<ViewModelAnimation>,
    item_switch: Res<ViewModelItemSwitch>,
    mut viewmodels: Query<&mut Transform, With<PlayerViewModel>>,
    mut was_animating: Local<bool>,
) {
    let interaction = animation
        .progress()
        .map(|(action, progress)| (action, (progress * PI).sin()));

    let switch_active = item_switch.is_active();
    let switch_wave = if switch_active {
        let progress = (item_switch.elapsed_ticks() as f32
            / ITEM_SWITCH_ANIMATION_DURATION_TICKS as f32)
            .clamp(0.0, 1.0);
        (progress * PI).sin()
    } else {
        0.0
    };
    let is_animating = interaction.is_some() || switch_active;

    if !is_animating && !*was_animating {
        return;
    }
    *was_animating = is_animating;

    for mut transform in &mut viewmodels {
        let mut animated = base_viewmodel_transform();

        if let Some((action, wave)) = interaction {
            match action {
                ViewModelAction::Break => {
                    animated.translation += Vec3::new(-0.03, 0.01, -0.14) * wave;
                    animated.rotation *=
                        Quat::from_euler(EulerRot::XYZ, -0.18 * wave, 0.05 * wave, -0.08 * wave);
                }
                ViewModelAction::Hit => {
                    animated.translation += Vec3::new(-0.05, -0.02, -0.18) * wave;
                    animated.rotation *=
                        Quat::from_euler(EulerRot::XYZ, -0.30 * wave, 0.10 * wave, -0.14 * wave);
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

        if *transform != animated {
            *transform = animated;
        }
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
