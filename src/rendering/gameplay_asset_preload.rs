use std::collections::BTreeSet;

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState, UntypedHandle},
    prelude::*,
};

use crate::content::{
    creature::CreatureRegistry,
    object::{ObjectRegistry, ObjectVisualDefinition},
    player::PlayerDefinition,
};

struct GameplayAssetPreload {
    path: String,
    handle: UntypedHandle,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GameplayAssetLoadProgress {
    pub(crate) loaded: usize,
    pub(crate) total: usize,
}

impl GameplayAssetLoadProgress {
    pub(crate) fn is_complete(self) -> bool {
        self.loaded == self.total
    }
}

#[derive(Resource, Default)]
pub(crate) struct GameplayAssetPreloads {
    assets: Vec<GameplayAssetPreload>,
}

impl GameplayAssetPreloads {
    pub(crate) fn from_content(
        asset_server: &AssetServer,
        player: &PlayerDefinition,
        creatures: &CreatureRegistry,
        objects: &ObjectRegistry,
    ) -> Self {
        let mut model_paths = BTreeSet::new();
        let mut image_paths = BTreeSet::new();

        if let Some(path) = player.model.as_ref() {
            model_paths.insert(path.clone());
        }

        for creature in creatures.iter() {
            model_paths.insert(creature.model.clone());
            image_paths.extend(creature.textures.values().cloned());
        }

        for object in objects.iter() {
            match &object.visual {
                ObjectVisualDefinition::Model { path } => {
                    model_paths.insert(path.clone());
                }
                ObjectVisualDefinition::SpritePrism { texture, .. } => {
                    image_paths.insert(texture.clone());
                }
            }
        }

        let mut assets = Vec::with_capacity(model_paths.len() + image_paths.len());
        for path in model_paths {
            assets.push(GameplayAssetPreload {
                handle: asset_server.load::<Gltf>(path.clone()).untyped(),
                path,
            });
        }
        for path in image_paths {
            assets.push(GameplayAssetPreload {
                handle: asset_server.load::<Image>(path.clone()).untyped(),
                path,
            });
        }

        Self { assets }
    }

    pub(crate) fn total(&self) -> usize {
        self.assets.len()
    }

    pub(crate) fn load_progress(
        &self,
        asset_server: &AssetServer,
    ) -> GameplayAssetLoadProgress {
        let mut loaded = 0;

        for asset in &self.assets {
            match asset_server.load_state(asset.handle.id()) {
                LoadState::Failed(error) => {
                    panic!("failed to load gameplay asset {}: {error:?}", asset.path)
                }
                LoadState::NotLoaded | LoadState::Loading | LoadState::Loaded => {}
            }

            match asset_server.recursive_dependency_load_state(asset.handle.id()) {
                RecursiveDependencyLoadState::Failed(error) => {
                    panic!(
                        "failed to load a dependency of gameplay asset {}: {error:?}",
                        asset.path
                    )
                }
                RecursiveDependencyLoadState::NotLoaded
                | RecursiveDependencyLoadState::Loading
                | RecursiveDependencyLoadState::Loaded => {}
            }

            if asset_server.is_loaded_with_dependencies(asset.handle.id()) {
                loaded += 1;
            }
        }

        GameplayAssetLoadProgress {
            loaded,
            total: self.assets.len(),
        }
    }
}
