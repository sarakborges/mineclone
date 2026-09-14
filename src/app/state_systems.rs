use bevy::prelude::*;

pub(crate) fn reset_next_state<S>(mut next_state: ResMut<NextState<S>>)
where
    S: FreelyMutableState + Default,
{
    next_state.set(S::default());
}

pub(crate) fn reset_next_state_on_escape<S>(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<S>>,
) where
    S: FreelyMutableState + Default,
{
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(S::default());
    }
}
