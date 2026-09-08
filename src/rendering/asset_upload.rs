use bevy::{prelude::*, render::render_asset::RenderAssetBytesPerFrame};

const RENDER_UPLOAD_BYTES_PER_FRAME: usize = 4 * 1024 * 1024;

pub struct AssetUploadPlugin;

impl Plugin for AssetUploadPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RenderAssetBytesPerFrame::new(
            RENDER_UPLOAD_BYTES_PER_FRAME,
        ));
    }
}
