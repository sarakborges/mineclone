use bevy::prelude::*;

use crate::{
    content::dimension::DimensionRegistry,
    localization::ActiveLanguage,
    world::dimension::CurrentDimension,
};

#[derive(Component)]
pub(super) struct DimensionHudText;

pub(super) fn update_dimension_hud(
    dimension: Res<CurrentDimension>,
    dimensions: Res<DimensionRegistry>,
    language: Res<ActiveLanguage>,
    mut dimension_text: Single<&mut Text, With<DimensionHudText>>,
) {
    let dimension_name = dimensions
        .get(&dimension.id)
        .map(|definition| definition.name.text(language.get()))
        .unwrap_or(dimension.id.as_str());

    if dimension_text.0 != dimension_name {
        dimension_text.0 = dimension_name.to_string();
    }
}
