use bevy::prelude::*;

use crate::player::{PlayerEntity, model::PlayerModelRoot};

use super::walking::WalkingState;

#[derive(Component, Clone, Copy)]
struct CrouchPoseBase {
    translation: Vec3,
}

pub(super) fn apply_player_crouch_pose(
    mut commands: Commands,
    walking: Single<&WalkingState, With<PlayerEntity>>,
    roots: Query<Entity, With<PlayerModelRoot>>,
    children: Query<&Children>,
    mut parts: Query<
        (Entity, &Name, &mut Transform, Option<&CrouchPoseBase>),
        Without<PlayerModelRoot>,
    >,
) {
    let blend = walking.crouch_blend();

    for root in &roots {
        for descendant in children.iter_descendants(root) {
            let Ok((entity, name, mut transform, base)) = parts.get_mut(descendant) else {
                continue;
            };
            let Some(drop) = crouch_drop(name.as_str()) else {
                continue;
            };

            let base_translation = base.map_or(transform.translation, |base| base.translation);
            if base.is_none() {
                commands.entity(entity).insert(CrouchPoseBase {
                    translation: base_translation,
                });
            }

            transform.translation = base_translation - Vec3::Y * drop * blend;
        }
    }
}

fn crouch_drop(name: &str) -> Option<f32> {
    Some(match name {
        "BodyPivot" => 0.18,
        "HeadPivot" => 0.28,
        "RightArmPivot" | "LeftArmPivot" => 0.24,
        "RightLegPivot" | "LeftLegPivot" => 0.02,
        _ => return None,
    })
}
