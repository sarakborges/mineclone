use bevy::prelude::*;

pub(crate) fn reset_next_state<S>(mut next_state: ResMut<NextState<S>>)
where
    S: FreelyMutableState + Default,
{
    next_state.set(S::default());
}
