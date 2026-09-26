use bevy::{ecs::component::Mutable, prelude::*};

pub(crate) fn reset_resource<R>(mut resource: ResMut<R>)
where
    R: Resource<Mutability = Mutable> + Default,
{
    *resource = R::default();
}
