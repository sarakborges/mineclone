use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use crate::{
    content::block::BlockDefinition,
    rendering::block_model_material::{BlockModelMaterial, BlockModelMaterialExtension},
    voxel::mesh::BlockFace,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockModelMode {
    Display,
    World,
}

#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct BlockModel {
    block_id: Option<&'static str>,
    mode: BlockModelMode,
    opacity: f32,
}

impl BlockModel {
    pub(crate) fn display(block_id: &'static str) -> Self {
        Self {
            block_id: Some(block_id),
            mode: BlockModelMode::Display,
            opacity: 1.0,
        }
    }

    pub(crate) fn empty_display() -> Self {
        Self {
            block_id: None,
            mode: BlockModelMode::Display,
            opacity: 1.0,
        }
    }

    pub(crate) fn world(block_id: &'static str, opacity: f32) -> Self {
        Self {
            block_id: Some(block_id),
            mode: BlockModelMode::World,
            opacity: opacity.clamp(0.0, 1.0),
        }
    }

    pub(crate) fn block_id(&self) -> Option<&'static str> {
        self.block_id
    }

    pub(crate) fn set_block_id(&mut self, block_id: Option<&'static str>) -> bool {
        if self.block_id == block_id {
            return false;
        }

        self.block_id = block_id;
        true
    }

    pub(crate) fn opacity(&self) -> f32 {
        self.opacity
    }

    pub(crate) fn faces(&self) -> &'static [BlockFace] {
        match self.mode {
            BlockModelMode::Display => &DISPLAY_FACES,
            BlockModelMode::World => &WORLD_FACES,
        }
    }
}

const DISPLAY_FACES: [BlockFace; 3] = [BlockFace::Top, BlockFace::Front, BlockFace::Right];
const WORLD_FACES: [BlockFace; 6] = [
    BlockFace::Right,
    BlockFace::Left,
    BlockFace::Top,
    BlockFace::Bottom,
    BlockFace::Front,
    BlockFace::Back,
];

#[derive(Clone, Copy)]
struct BlockDisplayFaceGeometry {
    origin: Vec2,
    axis_u: Vec2,
    axis_v: Vec2,
}

impl BlockDisplayFaceGeometry {
    fn points(self) -> [Vec2; 4] {
        [
            self.origin,
            self.origin + self.axis_u,
            self.origin + self.axis_u + self.axis_v,
            self.origin + self.axis_v,
        ]
    }
}

fn block_display_face_geometry(face: BlockFace) -> BlockDisplayFaceGeometry {
    match face {
        BlockFace::Top => BlockDisplayFaceGeometry {
            origin: Vec2::new(0.10, 0.28),
            axis_u: Vec2::new(0.40, 0.20),
            axis_v: Vec2::new(0.40, -0.20),
        },
        BlockFace::Front => BlockDisplayFaceGeometry {
            origin: Vec2::new(0.10, 0.28),
            axis_u: Vec2::new(0.40, 0.20),
            axis_v: Vec2::new(0.00, 0.42),
        },
        BlockFace::Right => BlockDisplayFaceGeometry {
            origin: Vec2::new(0.50, 0.48),
            axis_u: Vec2::new(0.40, -0.20),
            axis_v: Vec2::new(0.00, 0.42),
        },
        _ => panic!("{face:?} is not part of the display block model"),
    }
}

pub(crate) fn block_display_face_basis(face: BlockFace) -> (Vec4, Vec4) {
    let geometry = block_display_face_geometry(face);
    (
        Vec4::new(
            geometry.origin.x,
            geometry.origin.y,
            geometry.axis_u.x,
            geometry.axis_u.y,
        ),
        Vec4::new(geometry.axis_v.x, geometry.axis_v.y, 0.0, 0.0),
    )
}

#[derive(Resource)]
pub(crate) struct BlockModelMeshes {
    world_right: Handle<Mesh>,
    world_left: Handle<Mesh>,
    world_top: Handle<Mesh>,
    world_bottom: Handle<Mesh>,
    world_front: Handle<Mesh>,
    world_back: Handle<Mesh>,
    display_top: Handle<Mesh>,
    display_front: Handle<Mesh>,
    display_right: Handle<Mesh>,
}

impl BlockModelMeshes {
    pub(crate) fn world_face(&self, face: BlockFace) -> Handle<Mesh> {
        match face {
            BlockFace::Right => self.world_right.clone(),
            BlockFace::Left => self.world_left.clone(),
            BlockFace::Top => self.world_top.clone(),
            BlockFace::Bottom => self.world_bottom.clone(),
            BlockFace::Front => self.world_front.clone(),
            BlockFace::Back => self.world_back.clone(),
        }
    }

    pub(crate) fn display_face(&self, face: BlockFace) -> Handle<Mesh> {
        match face {
            BlockFace::Top => self.display_top.clone(),
            BlockFace::Front => self.display_front.clone(),
            BlockFace::Right => self.display_right.clone(),
            _ => panic!("{face:?} is not part of the display block model"),
        }
    }
}

struct BlockFaceMaterialHandles {
    right: Handle<BlockModelMaterial>,
    left: Handle<BlockModelMaterial>,
    top: Handle<BlockModelMaterial>,
    bottom: Handle<BlockModelMaterial>,
    front: Handle<BlockModelMaterial>,
    back: Handle<BlockModelMaterial>,
}

impl BlockFaceMaterialHandles {
    fn new(materials: &mut Assets<BlockModelMaterial>, opacity: f32) -> Self {
        Self {
            right: materials.add(block_model_placeholder_material(opacity)),
            left: materials.add(block_model_placeholder_material(opacity)),
            top: materials.add(block_model_placeholder_material(opacity)),
            bottom: materials.add(block_model_placeholder_material(opacity)),
            front: materials.add(block_model_placeholder_material(opacity)),
            back: materials.add(block_model_placeholder_material(opacity)),
        }
    }

    fn for_face(&self, face: BlockFace) -> Handle<BlockModelMaterial> {
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
    pub(crate) fn held_for_face(&self, face: BlockFace) -> Handle<BlockModelMaterial> {
        self.held.for_face(face)
    }

    pub(crate) fn preview_for_face(&self, face: BlockFace) -> Handle<BlockModelMaterial> {
        self.preview.for_face(face)
    }
}

pub(crate) fn setup_block_model_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BlockModelMaterial>>,
) {
    commands.insert_resource(BlockModelMeshes {
        world_right: meshes.add(block_face_mesh(BlockFace::Right)),
        world_left: meshes.add(block_face_mesh(BlockFace::Left)),
        world_top: meshes.add(block_face_mesh(BlockFace::Top)),
        world_bottom: meshes.add(block_face_mesh(BlockFace::Bottom)),
        world_front: meshes.add(block_face_mesh(BlockFace::Front)),
        world_back: meshes.add(block_face_mesh(BlockFace::Back)),
        display_top: meshes.add(block_display_face_mesh(BlockFace::Top)),
        display_front: meshes.add(block_display_face_mesh(BlockFace::Front)),
        display_right: meshes.add(block_display_face_mesh(BlockFace::Right)),
    });
    commands.insert_resource(BlockModelMaterials {
        held: BlockFaceMaterialHandles::new(&mut materials, 1.0),
        preview: BlockFaceMaterialHandles::new(&mut materials, 0.68),
    });
}

pub(crate) fn block_display_face_shade(face: BlockFace) -> f32 {
    match face {
        BlockFace::Top => 1.0,
        BlockFace::Front => 0.86,
        BlockFace::Right => 0.74,
        _ => 1.0,
    }
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

    if !texture.is_empty() {
        Some(texture)
    } else {
        first_block_texture(block)
    }
}

fn first_block_texture(block: &BlockDefinition) -> Option<&str> {
    [
        block.textures.top.as_str(),
        block.textures.front.as_str(),
        block.textures.right.as_str(),
        block.textures.left.as_str(),
        block.textures.back.as_str(),
        block.textures.bottom.as_str(),
    ]
    .into_iter()
    .find(|texture| !texture.is_empty())
}

pub(crate) fn block_face_material_data(
    face: BlockFace,
    block: &BlockDefinition,
    asset_server: &AssetServer,
    opacity: f32,
) -> BlockModelMaterial {
    let opacity = opacity.clamp(0.0, 1.0);

    BlockModelMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, opacity),
            base_color_texture: block_face_texture(face, block)
                .map(|texture| asset_server.load(texture.to_owned())),
            perceptual_roughness: 1.0,
            alpha_mode: block.alpha_mode(opacity),
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        },
        extension: BlockModelMaterialExtension::default(),
    }
}

pub(crate) fn set_block_model_tint(material: &mut BlockModelMaterial, tint: Color) {
    material.extension.set_tint(tint);
}

pub(crate) fn apply_block_display_shading(
    material: &mut BlockModelMaterial,
    face: BlockFace,
    opacity: f32,
) {
    let shade = block_display_face_shade(face);
    material.base.base_color = Color::srgba(shade, shade, shade, opacity.clamp(0.0, 1.0));
}

fn block_model_placeholder_material(opacity: f32) -> BlockModelMaterial {
    let opacity = opacity.clamp(0.0, 1.0);

    BlockModelMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, opacity),
            perceptual_roughness: 1.0,
            alpha_mode: if opacity < 1.0 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        },
        extension: BlockModelMaterialExtension::default(),
    }
}

fn display_position(point: Vec2) -> [f32; 3] {
    [(point.x - 0.5) * 2.0, (0.5 - point.y) * 2.0, 0.0]
}

fn block_display_face_mesh(face: BlockFace) -> Mesh {
    let geometry = block_display_face_geometry(face);
    let points = geometry.points();
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        points.into_iter().map(display_position).collect::<Vec<_>>(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec())
    .with_inserted_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]))
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
