use bevy::prelude::*;

pub(crate) fn set_visibility<M, const VISIBLE: bool>(mut roots: Query<&mut Visibility, With<M>>)
where
    M: Component,
{
    let next = if VISIBLE {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut visibility in &mut roots {
        *visibility = next;
    }
}
