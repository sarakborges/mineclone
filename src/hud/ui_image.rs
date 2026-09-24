use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};

pub(crate) fn load_smooth_image(asset_server: &AssetServer, path: String) -> Handle<Image> {
    asset_server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::linear();
        })
        .load(path)
}
