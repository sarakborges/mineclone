use std::f32::consts::PI;

use bevy::prelude::*;

use crate::player::hotbar::PlayerHotbar;

const BREAK_ANIMATION_DURATION: f32 = 0.16;
const PLACE_ANIMATION_DURATION: f32 = 0.22;
pub(super) const ITEM_SWITCH_ANIMATION_DURATION: f32 = 0.30;

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
    elapsed: f32,
}

impl ViewModelAnimation {
    pub(crate) fn play_break(&mut self) {
        self.action = Some(ViewModelAction::Break);
        self.elapsed = 0.0;
    }

    pub(crate) fn play_place(&mut self) {
        self.action = Some(ViewModelAction::Place);
        self.elapsed = 0.0;
    }
}

#[derive(Resource, Default)]
pub(super) struct ViewModelItemSwitch {
    initialized: bool,
    displayed_block_id: Option<&'static str>,
    target_block_id: Option<&'static str>,
    elapsed: f32,
    active: bool,
}

impl ViewModelItemSwitch {
    pub(super) fn initialize(&mut self, block_id: Option<&'static str>) {
        self.initialized = true;
        self.displayed_block_id = block_id;
        self.target_block_id = block_id;
        self.elapsed = 0.0;
        self.active = false;
    }

    pub(super) fn displayed_block_id(&self) -> Option<&'static str> {
        self.displayed_block_id
    }

    pub(super) fn is_active(&self) -> bool {
        self.active
    }

    pub(super) fn elapsed(&self) -> f32 {
        self.elapsed
    }
}

pub(super) fn advance_item_switch(
    time: Res<Time>,
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
        item_switch.elapsed = 0.0;
        item_switch.active = true;
    }

    if !item_switch.active {
        return;
    }

    item_switch.elapsed += time.delta_secs();
    let midpoint = ITEM_SWITCH_ANIMATION_DURATION * 0.5;

    if item_switch.elapsed >= midpoint
        && item_switch.displayed_block_id != item_switch.target_block_id
    {
        item_switch.displayed_block_id = item_switch.target_block_id;
    }

    if item_switch.elapsed >= ITEM_SWITCH_ANIMATION_DURATION {
        item_switch.displayed_block_id = item_switch.target_block_id;
        item_switch.elapsed = 0.0;
        item_switch.active = false;
    }
}

pub(super) fn animate_viewmodel(
    time: Res<Time>,
    mut animation: ResMut<ViewModelAnimation>,
    item_switch: Res<ViewModelItemSwitch>,
    mut viewmodels: Query<&mut Transform, With<PlayerViewModel>>,
) {
    let interaction = animation.action.and_then(|action| {
        animation.elapsed += time.delta_secs();
        let duration = match action {
            ViewModelAction::Break => BREAK_ANIMATION_DURATION,
            ViewModelAction::Place => PLACE_ANIMATION_DURATION,
        };
        let progress = (animation.elapsed / duration).clamp(0.0, 1.0);

        if progress >= 1.0 {
            animation.action = None;
            animation.elapsed = 0.0;
            None
        } else {
            Some((action, (progress * PI).sin()))
        }
    });

    let switch_wave = if item_switch.is_active() {
        let progress = (item_switch.elapsed() / ITEM_SWITCH_ANIMATION_DURATION).clamp(0.0, 1.0);
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
