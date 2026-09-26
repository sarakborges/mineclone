use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    content::attack::{AttackDefinition, AttackEffectKind},
    entity::EntityHealth,
    gameplay::random::next_unit_f32,
};

use super::{
    CreatureInstance,
    lifecycle::CreatureDeathTimer,
    motion::CreatureMotion,
    visual::CreatureAnimationState,
};

type DamageableCreatures<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut EntityHealth,
        &'static mut CreatureAnimationState,
        &'static mut CreatureMotion,
        &'static Transform,
    ),
    With<CreatureInstance>,
>;

#[derive(SystemParam)]
pub(crate) struct CreatureAttackRuntime<'w, 's> {
    commands: Commands<'w, 's>,
    creatures: DamageableCreatures<'w, 's>,
    random_state: Local<'s, u32>,
}

impl CreatureAttackRuntime<'_, '_> {
    pub(crate) fn apply(
        &mut self,
        entity: Entity,
        attack: &AttackDefinition,
        player_position: Vec3,
    ) -> bool {
        let Ok((mut health, mut animation, mut motion, creature_transform)) =
            self.creatures.get_mut(entity)
        else {
            return false;
        };
        if health.is_dead() {
            return false;
        }

        let dead = health.damage(attack.damage);
        let direction = creature_transform.translation - player_position;
        for effect in &attack.effects {
            if !effect_applies(effect.chance, &mut self.random_state) {
                continue;
            }
            match effect.effect {
                AttackEffectKind::Knockback => {
                    motion.apply_knockback(direction, effect.strength);
                }
            }
        }
        animation.trigger(if dead { "death" } else { "hurt" });
        if dead {
            self.commands
                .entity(entity)
                .insert(CreatureDeathTimer(Timer::from_seconds(
                    0.75,
                    TimerMode::Once,
                )));
        }
        true
    }
}

fn effect_applies(chance: f32, random_state: &mut u32) -> bool {
    if chance >= 1.0 {
        return true;
    }
    if *random_state == 0 {
        *random_state = 0x9E37_79B9;
    }
    next_unit_f32(random_state) <= chance
}
