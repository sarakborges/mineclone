use bevy::{
    prelude::*, reflect::TypePath, render::render_resource::AsBindGroup, shader::ShaderRef,
};

use crate::{
    content::{block::BlockDefinition, block_orientation::BlockOrientation},
    rendering::{
        block_display::{block_display_face_basis, block_display_face_shade},
        block_texture::{block_face_texture_layers, load_block_texture_layer},
        color::color_to_linear_vec4,
    },
    voxel::{
        block_face::BlockFace,
        orientation::source_face_for_oriented_face,
    },
};

const BLOCK_ICON_SHADER_PATH: &str = "shaders/block_icon_material.wgsl";

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub(crate) struct BlockIconMaterial {
    #[texture(0)]
    #[sampler(1)]
    top_texture: Handle<Image>,
    #[texture(2)]
    #[sampler(3)]
    front_texture: Handle<Image>,
    #[texture(4)]
    #[sampler(5)]
    right_texture: Handle<Image>,
    #[texture(6)]
    #[sampler(7)]
    top_overlay_texture: Handle<Image>,
    #[texture(8)]
    #[sampler(9)]
    front_overlay_texture: Handle<Image>,
    #[texture(10)]
    #[sampler(11)]
    right_overlay_texture: Handle<Image>,
    #[uniform(12)]
    tint: Vec4,
    #[uniform(13)]
    face_shades: Vec4,
    #[uniform(14)]
    base_tint_flags: Vec4,
    #[uniform(15)]
    overlay_tint_flags: Vec4,
    #[uniform(16)]
    overlay_present_flags: Vec4,
    #[uniform(17)]
    top_origin_axis_u: Vec4,
    #[uniform(18)]
    top_axis_v: Vec4,
    #[uniform(19)]
    front_origin_axis_u: Vec4,
    #[uniform(20)]
    front_axis_v: Vec4,
    #[uniform(21)]
    right_origin_axis_u: Vec4,
    #[uniform(22)]
    right_axis_v: Vec4,
}

impl UiMaterial for BlockIconMaterial {
    fn fragment_shader() -> ShaderRef {
        BLOCK_ICON_SHADER_PATH.into()
    }
}

impl BlockIconMaterial {
    pub(crate) fn empty() -> Self {
        let (top_origin_axis_u, top_axis_v) = block_display_face_basis(BlockFace::Top);
        let (front_origin_axis_u, front_axis_v) = block_display_face_basis(BlockFace::Front);
        let (right_origin_axis_u, right_axis_v) = block_display_face_basis(BlockFace::Right);

        Self {
            top_texture: Handle::default(),
            front_texture: Handle::default(),
            right_texture: Handle::default(),
            top_overlay_texture: Handle::default(),
            front_overlay_texture: Handle::default(),
            right_overlay_texture: Handle::default(),
            tint: Vec4::ONE,
            face_shades: block_face_shades(),
            base_tint_flags: Vec4::ZERO,
            overlay_tint_flags: Vec4::ZERO,
            overlay_present_flags: Vec4::ZERO,
            top_origin_axis_u,
            top_axis_v,
            front_origin_axis_u,
            front_axis_v,
            right_origin_axis_u,
            right_axis_v,
        }
    }

    pub(crate) fn from_block(
        block: &BlockDefinition,
        asset_server: &AssetServer,
        tint: Color,
    ) -> Self {
        let mut material = Self::empty();
        material.set_block(block, asset_server);
        material.set_tint(tint);
        material
    }

    pub(crate) fn set_block(&mut self, block: &BlockDefinition, asset_server: &AssetServer) {
        self.set_block_orientation(block, block.default_orientation(), asset_server);
    }

    pub(crate) fn set_block_orientation(
        &mut self,
        block: &BlockDefinition,
        orientation: BlockOrientation,
        asset_server: &AssetServer,
    ) {
        let top = load_oriented_face_layers(asset_server, BlockFace::Top, orientation, block);
        let front = load_oriented_face_layers(asset_server, BlockFace::Front, orientation, block);
        let right = load_oriented_face_layers(asset_server, BlockFace::Right, orientation, block);

        self.top_texture = top.base;
        self.front_texture = front.base;
        self.right_texture = right.base;
        self.top_overlay_texture = top.overlay;
        self.front_overlay_texture = front.overlay;
        self.right_overlay_texture = right.overlay;
        self.base_tint_flags = Vec4::new(top.base_dyable, front.base_dyable, right.base_dyable, 0.0);
        self.overlay_tint_flags = Vec4::new(
            top.overlay_dyable,
            front.overlay_dyable,
            right.overlay_dyable,
            0.0,
        );
        self.overlay_present_flags = Vec4::new(
            top.overlay_present,
            front.overlay_present,
            right.overlay_present,
            0.0,
        );
    }

    pub(crate) fn set_tint(&mut self, tint: Color) {
        self.tint = color_to_linear_vec4(tint);
    }
}

struct IconFaceLayers {
    base: Handle<Image>,
    overlay: Handle<Image>,
    base_dyable: f32,
    overlay_dyable: f32,
    overlay_present: f32,
}

fn load_oriented_face_layers(
    asset_server: &AssetServer,
    face: BlockFace,
    orientation: BlockOrientation,
    block: &BlockDefinition,
) -> IconFaceLayers {
    let source_face = source_face_for_oriented_face(face, orientation);
    let layers = block_face_texture_layers(source_face, block);
    let base = layers
        .first()
        .map(|layer| load_block_texture_layer(asset_server, layer))
        .unwrap_or_default();
    let overlay_layer = layers.get(1);
    let overlay = overlay_layer
        .map(|layer| load_block_texture_layer(asset_server, layer))
        .unwrap_or_default();

    IconFaceLayers {
        base,
        overlay,
        base_dyable: layers.first().is_some_and(|layer| layer.dyable) as u8 as f32,
        overlay_dyable: overlay_layer.is_some_and(|layer| layer.dyable) as u8 as f32,
        overlay_present: overlay_layer.is_some() as u8 as f32,
    }
}

fn block_face_shades() -> Vec4 {
    Vec4::new(
        block_display_face_shade(BlockFace::Top),
        block_display_face_shade(BlockFace::Front),
        block_display_face_shade(BlockFace::Right),
        1.0,
    )
}
