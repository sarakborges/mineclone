use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::*,
    render::render_resource::PrimitiveTopology,
};

use crate::{content::block::BlockDefinition, voxel::mesh::BlockFace};

pub(crate) fn block_faces() -> [BlockFace; 6] {
    [
        BlockFace::Right,
        BlockFace::Left,
        BlockFace::Top,
        BlockFace::Bottom,
        BlockFace::Front,
        BlockFace::Back,
    ]
}

pub(crate) fn block_face_material(
    face: BlockFace,
    block: &BlockDefinition,
    asset_server: &AssetServer,
    materials: &mut Assets<StandardMaterial>,
    opacity: f32,
) -> Handle<StandardMaterial> {
    let texture = match face {
        BlockFace::Right => &block.textures.right,
        BlockFace::Left => &block.textures.left,
        BlockFace::Top => &block.textures.top,
        BlockFace::Bottom => &block.textures.bottom,
        BlockFace::Front => &block.textures.front,
        BlockFace::Back => &block.textures.back,
    };
    let opacity = opacity.clamp(0.0, 1.0);

    materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, opacity),
        base_color_texture: Some(asset_server.load(texture.clone())),
        perceptual_roughness: 1.0,
        alpha_mode: if opacity < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        unlit: true,
        ..default()
    })
}

pub(crate) fn block_face_mesh(face: BlockFace) -> Mesh {
    let (vertices, normal) = match face {
        BlockFace::Right => (
            [[0.5, -0.5, 0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5]],
            [1.0, 0.0, 0.0],
        ),
        BlockFace::Left => (
            [[-0.5, -0.5, -0.5], [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5]],
            [-1.0, 0.0, 0.0],
        ),
        BlockFace::Top => (
            [[-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5]],
            [0.0, 1.0, 0.0],
        ),
        BlockFace::Bottom => (
            [[-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, -0.5, 0.5], [-0.5, -0.5, 0.5]],
            [0.0, -1.0, 0.0],
        ),
        BlockFace::Front => (
            [[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]],
            [0.0, 0.0, 1.0],
        ),
        BlockFace::Back => (
            [[0.5, -0.5, -0.5], [-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5]],
            [0.0, 0.0, -1.0],
        ),
    };

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![normal; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    )
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
}
