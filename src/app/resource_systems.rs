use bevy::prelude::*;

pub(crate) fn reset_resource<R>(mut resource: ResMut<R>)
where
    R: Resource + Default,
{
    *resource = R::default();
}
