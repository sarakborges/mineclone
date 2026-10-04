use bevy::prelude::*;

use crate::{
    content::dimension::DimensionRegistry,
    localization::{ActiveLanguage, Language},
    world::dimension::CurrentDimension,
};

#[derive(Component)]
pub(super) struct DimensionHudText;

pub(super) trait LocalizedHudSource<R>: Resource
where
    R: Resource,
{
    fn id(&self) -> &str;
    fn localized_name<'a>(&'a self, registry: &'a R, language: Language) -> Option<&'a str>;
}

impl LocalizedHudSource<DimensionRegistry> for CurrentDimension {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn localized_name<'a>(
        &'a self,
        registry: &'a DimensionRegistry,
        language: Language,
    ) -> Option<&'a str> {
        registry
            .get(self.id.as_str())
            .map(|definition| definition.name.text(language))
    }
}

pub(super) fn update_localized_name_hud<S, R, M>(
    source: Res<S>,
    registry: Res<R>,
    language: Res<ActiveLanguage>,
    mut text: Single<&mut Text, With<M>>,
    mut cached: Local<Option<(String, Language)>>,
) where
    S: LocalizedHudSource<R>,
    R: Resource,
    M: Component,
{
    let id = source.id();
    let language = language.get();
    let next = source.localized_name(&registry, language).unwrap_or(id);
    let cache_matches = cached.as_ref().is_some_and(|(cached_id, cached_language)| {
        cached_id == id && *cached_language == language
    });
    if cache_matches && !registry.is_changed() && text.0 == next {
        return;
    }
    *cached = Some((id.to_owned(), language));

    if text.0 != next {
        text.0 = next.to_owned();
    }
}
