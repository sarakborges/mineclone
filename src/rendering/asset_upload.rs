use bevy::{prelude::*, render::render_asset::RenderAssetBytesPerFrame};

// Keep GPU uploads bounded without letting continuously streamed chunk meshes
// starve small image assets. Bevy's limit is soft: a single large asset may
// overshoot it and exhaust the shared budget for the rest of that frame.
const RENDER_UPLOAD_BYTES_PER_FRAME: usize = 32 * 1024 * 1024;

pub struct AssetUploadPlugin;

impl Plugin for AssetUploadPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RenderAssetBytesPerFrame::new(RENDER_UPLOAD_BYTES_PER_FRAME));
    }
}
