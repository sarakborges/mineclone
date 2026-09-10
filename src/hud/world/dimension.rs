use bevy::prelude::*;

use crate::{content::dimension::DimensionRegistry, world::dimension::CurrentDimension};

#[derive(Component)]
pub(super) struct DimensionHudText;

pub(super) fn update_dimension_hud(
    dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    mut dimension_text: Single<&mut Text, With<DimensionHudText>>,
) {
    let dimension_name = dimensions
        .get(&dimension.id)
        .map(|definition| definition.name.as_str())
        .unwrap_or(dimension.id.as_str());

    dimension_text.0 = dimension_name.to_string();
}
