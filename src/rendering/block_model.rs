use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use crate::{content::block::BlockDefinition, voxel::mesh::BlockFace};

#[derive(Resource)]
pub(crate) struct BlockModelMeshes {
    right: Handle<Mesh>,
    left: Handle<Mesh>,
    top: Handle<Mesh>,
    bottom: Handle<Mesh>,
    front: Handle<Mesh>,
    back: Handle<Mesh>,
}

impl BlockModelMeshes {
    pub(crate) fn for_face(&self, face: BlockFace) -> Handle<Mesh> {
        match face {
            BlockFace::Right => self.right.clone(),
            BlockFace::Left => self.left.clone(),
            BlockFace::Top => self.top.clone(),
            BlockFace::Bottom => self.bottom.clone(),
            BlockFace::Front => self.front.clone(),
            BlockFace::Back => self.back.clone(),
        }
    }
}

struct BlockFaceMaterialHandles {
    right: Handle<StandardMaterial>,
    left: Handle<StandardMaterial>,
    top: Handle<StandardMaterial>,
    bottom: Handle<StandardMaterial>,
    front: Handle<StandardMaterial>,
    back: Handle<StandardMaterial>,
}

impl BlockFaceMaterialHandles {
    fn new(materials: &mut Assets<StandardMaterial>, opacity: f32) -> Self {
        Self {
            right: materials.add(block_model_placeholder_material(opacity)),
            left: materials.add(block_model_placeholder_material(opacity)),
            top: materials.add(block_model_placeholder_material(opacity)),
            bottom: materials.add(block_model_placeholder_material(opacity)),
            front: materials.add(block_model_placeholder_material(opacity)),
            back: materials.add(block_model_placeholder_material(opacity)),
        }
    }

    fn for_face(&self, face: BlockFace) -> Handle<StandardMaterial> {
        match face {
            BlockFace::Right => self.right.clone(),
            BlockFace::Left => self.left.clone(),
            BlockFace::Top => self.top.clone(),
            BlockFace::Bottom => self.bottom.clone(),
            BlockFace::Front => self.front.clone(),
            BlockFace::Back => self.back.clone(),
        }
    }
}

#[derive(Resource)]
pub(crate) struct BlockModelMaterials {
    held: BlockFaceMaterialHandles,
    preview: BlockFaceMaterialHandles,
}

impl BlockModelMaterials {
    pub(crate) fn held_for_face(&self, face: BlockFace) -> Handle<StandardMaterial> {
        self.held.for_face(face)
    }

    pub(crate) fn preview_for_face(&self, face: BlockFace) -> Handle<StandardMaterial> {
        self.preview.for_face(face)
    }
}

pub(crate) fn setup_block_model_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(BlockModelMeshes {
        right: meshes.add(block_face_mesh(BlockFace::Right)),
        left: meshes.add(block_face_mesh(BlockFace::Left)),
        top: meshes.add(block_face_mesh(BlockFace::Top)),
        bottom: meshes.add(block_face_mesh(BlockFace::Bottom)),
        front: meshes.add(block_face_mesh(BlockFace::Front)),
        back: meshes.add(block_face_mesh(BlockFace::Back)),
    });
    commands.insert_resource(BlockModelMaterials {
        held: BlockFaceMaterialHandles::new(&mut materials, 1.0),
        preview: BlockFaceMaterialHandles::new(&mut materials, 0.68),
    });
}

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

pub(crate) fn block_face_texture(face: BlockFace, block: &BlockDefinition) -> Option<&str> {
    let texture = match face {
        BlockFace::Right => block.textures.right.as_str(),
        BlockFace::Left => block.textures.left.as_str(),
        BlockFace::Top => block.textures.top.as_str(),
        BlockFace::Bottom => block.textures.bottom.as_str(),
        BlockFace::Front => block.textures.front.as_str(),
        BlockFace::Back => block.textures.back.as_str(),
    };

    (!texture.is_empty()).then_some(texture)
}

pub(crate) fn block_face_material_data(
    face: BlockFace,
    block: &BlockDefinition,
    asset_server: &AssetServer,
    opacity: f32,
) -> StandardMaterial {
    let opacity = opacity.clamp(0.0, 1.0);

    StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, opacity),
        base_color_texture: block_face_texture(face, block)
            .map(|texture| asset_server.load(texture.to_owned())),
        perceptual_roughness: 1.0,
        alpha_mode: if opacity < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        unlit: true,
        ..default()
    }
}

fn block_model_placeholder_material(opacity: f32) -> StandardMaterial {
    let opacity = opacity.clamp(0.0, 1.0);

    StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, opacity),
        perceptual_roughness: 1.0,
        alpha_mode: if opacity < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        unlit: true,
        ..default()
    }
}

pub(crate) fn block_face_mesh(face: BlockFace) -> Mesh {
    let (vertices, normal) = match face {
        BlockFace::Right => (
            [
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
            ],
            [1.0, 0.0, 0.0],
        ),
        BlockFace::Left => (
            [
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ],
            [-1.0, 0.0, 0.0],
        ),
        BlockFace::Top => (
            [
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
            ],
            [0.0, 1.0, 0.0],
        ),
        BlockFace::Bottom => (
            [
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
            ],
            [0.0, -1.0, 0.0],
        ),
        BlockFace::Front => (
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            [0.0, 0.0, 1.0],
        ),
        BlockFace::Back => (
            [
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
            ],
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
