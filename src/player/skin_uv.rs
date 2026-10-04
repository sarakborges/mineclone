use bevy::{
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    mesh::VertexAttributeValues,
    prelude::*,
};

use crate::app::game_state::GameState;

const SKIN_ATLAS_SIZE: f32 = 64.0;
const FACE_NORMAL_THRESHOLD: f32 = 0.5;
const OUTER_LAYER_DEPTH_PIXELS: f32 = 0.25;

#[derive(Component)]
struct PlayerSkinOuterLayer;

type PlayerSkinMesh<'a> = (
    Entity,
    &'a Name,
    &'a Mesh3d,
    &'a MeshMaterial3d<StandardMaterial>,
    &'a RenderLayers,
);

#[derive(Clone, Copy, Debug, PartialEq)]
struct SkinPartSpec {
    base_layout: SkinBoxUv,
    outer_layer: Option<OuterLayerSpec>,
}

impl SkinPartSpec {
    fn for_mesh_name(name: &str) -> Option<Self> {
        match name {
            "HeadMesh" => Some(Self {
                base_layout: SkinBoxUv::HEAD,
                outer_layer: None,
            }),
            "HairLayer" => Some(Self {
                base_layout: SkinBoxUv::HEAD_LAYER,
                outer_layer: None,
            }),
            "BodyMesh" => Some(Self {
                base_layout: SkinBoxUv::BODY,
                outer_layer: Some(OuterLayerSpec {
                    name: "BodyLayer",
                    layout: SkinBoxUv::BODY_LAYER,
                    dimensions_pixels: Vec3::new(8.0, 12.0, 4.0),
                }),
            }),
            "RightArmMesh" => Some(Self {
                base_layout: SkinBoxUv::RIGHT_ARM,
                outer_layer: Some(OuterLayerSpec {
                    name: "RightArmLayer",
                    layout: SkinBoxUv::RIGHT_ARM_LAYER,
                    dimensions_pixels: Vec3::new(4.0, 12.0, 4.0),
                }),
            }),
            "LeftArmMesh" => Some(Self {
                base_layout: SkinBoxUv::LEFT_ARM,
                outer_layer: Some(OuterLayerSpec {
                    name: "LeftArmLayer",
                    layout: SkinBoxUv::LEFT_ARM_LAYER,
                    dimensions_pixels: Vec3::new(4.0, 12.0, 4.0),
                }),
            }),
            "RightLegMesh" => Some(Self {
                base_layout: SkinBoxUv::RIGHT_LEG,
                outer_layer: Some(OuterLayerSpec {
                    name: "RightLegLayer",
                    layout: SkinBoxUv::RIGHT_LEG_LAYER,
                    dimensions_pixels: Vec3::new(4.0, 12.0, 4.0),
                }),
            }),
            "LeftLegMesh" => Some(Self {
                base_layout: SkinBoxUv::LEFT_LEG,
                outer_layer: Some(OuterLayerSpec {
                    name: "LeftLegLayer",
                    layout: SkinBoxUv::LEFT_LEG_LAYER,
                    dimensions_pixels: Vec3::new(4.0, 12.0, 4.0),
                }),
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OuterLayerSpec {
    name: &'static str,
    layout: SkinBoxUv,
    dimensions_pixels: Vec3,
}

impl OuterLayerSpec {
    fn relative_scale(self) -> Vec3 {
        let expansion = Vec3::splat(OUTER_LAYER_DEPTH_PIXELS * 2.0);
        (self.dimensions_pixels + expansion) / self.dimensions_pixels
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SkinBoxUv {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    depth: f32,
}

impl SkinBoxUv {
    const HEAD: Self = Self::new(0.0, 0.0, 8.0, 8.0, 8.0);
    const HEAD_LAYER: Self = Self::new(32.0, 0.0, 8.0, 8.0, 8.0);

    const BODY: Self = Self::new(16.0, 16.0, 8.0, 12.0, 4.0);
    const BODY_LAYER: Self = Self::new(16.0, 32.0, 8.0, 12.0, 4.0);

    const RIGHT_ARM: Self = Self::new(40.0, 16.0, 4.0, 12.0, 4.0);
    const RIGHT_ARM_LAYER: Self = Self::new(40.0, 32.0, 4.0, 12.0, 4.0);

    const LEFT_ARM: Self = Self::new(32.0, 48.0, 4.0, 12.0, 4.0);
    const LEFT_ARM_LAYER: Self = Self::new(48.0, 48.0, 4.0, 12.0, 4.0);

    const RIGHT_LEG: Self = Self::new(0.0, 16.0, 4.0, 12.0, 4.0);
    const RIGHT_LEG_LAYER: Self = Self::new(0.0, 32.0, 4.0, 12.0, 4.0);

    const LEFT_LEG: Self = Self::new(16.0, 48.0, 4.0, 12.0, 4.0);
    const LEFT_LEG_LAYER: Self = Self::new(0.0, 48.0, 4.0, 12.0, 4.0);

    const fn new(x: f32, y: f32, width: f32, height: f32, depth: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            depth,
        }
    }

    fn uv(self, position: [f32; 3], normal: [f32; 3]) -> Option<[f32; 2]> {
        let face = SkinFace::from_normal(normal)?;
        let rect = self.rect(face);
        let [s, t] = face.face_coordinates(position);
        Some([
            (rect.x + s * rect.width) / SKIN_ATLAS_SIZE,
            (rect.y + t * rect.height) / SKIN_ATLAS_SIZE,
        ])
    }

    fn rect(self, face: SkinFace) -> PixelRect {
        match face {
            SkinFace::Right => PixelRect::new(self.x, self.y + self.depth, self.depth, self.height),
            SkinFace::Front => PixelRect::new(
                self.x + self.depth,
                self.y + self.depth,
                self.width,
                self.height,
            ),
            SkinFace::Left => PixelRect::new(
                self.x + self.depth + self.width,
                self.y + self.depth,
                self.depth,
                self.height,
            ),
            SkinFace::Back => PixelRect::new(
                self.x + self.depth + self.width + self.depth,
                self.y + self.depth,
                self.width,
                self.height,
            ),
            SkinFace::Top => PixelRect::new(
                self.x + self.depth,
                self.y,
                self.width,
                self.depth,
            ),
            SkinFace::Bottom => PixelRect::new(
                self.x + self.depth + self.width,
                self.y,
                self.width,
                self.depth,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkinFace {
    Right,
    Front,
    Left,
    Back,
    Top,
    Bottom,
}

impl SkinFace {
    fn from_normal(normal: [f32; 3]) -> Option<Self> {
        if normal[2] > FACE_NORMAL_THRESHOLD {
            Some(Self::Front)
        } else if normal[2] < -FACE_NORMAL_THRESHOLD {
            Some(Self::Back)
        } else if normal[0] < -FACE_NORMAL_THRESHOLD {
            Some(Self::Right)
        } else if normal[0] > FACE_NORMAL_THRESHOLD {
            Some(Self::Left)
        } else if normal[1] > FACE_NORMAL_THRESHOLD {
            Some(Self::Top)
        } else if normal[1] < -FACE_NORMAL_THRESHOLD {
            Some(Self::Bottom)
        } else {
            None
        }
    }

    fn face_coordinates(self, position: [f32; 3]) -> [f32; 2] {
        let [x, y, z] = position;
        match self {
            Self::Front => [x + 0.5, 0.5 - y],
            Self::Back => [x + 0.5, 0.5 - y],
            Self::Right => [0.5 - z, 0.5 - y],
            Self::Left => [z + 0.5, 0.5 - y],
            Self::Top => [x + 0.5, z + 0.5],
            Self::Bottom => [x + 0.5, 0.5 - z],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PixelRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl PixelRect {
    const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub(crate) struct PlayerSkinUvPlugin;

impl Plugin for PlayerSkinUvPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (map_player_skin_meshes, sync_outer_layer_render_layers)
                .chain()
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn map_player_skin_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    player_meshes: Query<PlayerSkinMesh<'_>, Added<RenderLayers>>,
) {
    for (entity, name, mesh_handle, material_handle, render_layers) in &player_meshes {
        let Some(spec) = SkinPartSpec::for_mesh_name(name.as_str()) else {
            continue;
        };

        let Some(mut source_mesh) = meshes.get_mut(mesh_handle.id()) else {
            warn!(
                "player skin UV mapping skipped for {}: source mesh is unavailable",
                name.as_str()
            );
            continue;
        };
        if !remap_mesh_uvs(&mut source_mesh, spec.base_layout) {
            warn!(
                "player skin UV mapping skipped for {}: source mesh has unexpected vertex data",
                name.as_str()
            );
            continue;
        }

        let Some(outer_spec) = spec.outer_layer else {
            continue;
        };

        let mut outer_mesh = (*source_mesh).clone();
        drop(source_mesh);
        if !remap_mesh_uvs(&mut outer_mesh, outer_spec.layout) {
            warn!(
                "player skin outer layer {} skipped: source mesh has unexpected vertex data",
                outer_spec.name
            );
            continue;
        }
        let outer_mesh = meshes.add(outer_mesh);

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Name::new(outer_spec.name),
                PlayerSkinOuterLayer,
                Mesh3d(outer_mesh),
                material_handle.clone(),
                Transform::from_scale(outer_spec.relative_scale()),
                Visibility::Inherited,
                render_layers.clone(),
                NotShadowCaster,
            ));
        });
    }
}

fn sync_outer_layer_render_layers(
    parent_layers: Query<&RenderLayers, Without<PlayerSkinOuterLayer>>,
    mut outer_layers: Query<(&ChildOf, &mut RenderLayers), With<PlayerSkinOuterLayer>>,
) {
    for (parent, mut render_layers) in &mut outer_layers {
        let Ok(expected) = parent_layers.get(parent.parent()) else {
            continue;
        };
        if *render_layers != *expected {
            *render_layers = expected.clone();
        }
    }
}

fn remap_mesh_uvs(mesh: &mut Mesh, layout: SkinBoxUv) -> bool {
    let uvs = {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            return false;
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            return false;
        };
        if positions.len() != normals.len() {
            return false;
        }

        let mut uvs = Vec::with_capacity(positions.len());
        for (&position, &normal) in positions.iter().zip(normals.iter()) {
            let Some(uv) = layout.uv(position, normal) else {
                return false;
            };
            uvs.push(uv);
        }
        uvs
    };

    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_uv(actual: [f32; 2], expected_pixels: [f32; 2]) {
        let expected = [
            expected_pixels[0] / SKIN_ATLAS_SIZE,
            expected_pixels[1] / SKIN_ATLAS_SIZE,
        ];
        assert!((actual[0] - expected[0]).abs() < f32::EPSILON);
        assert!((actual[1] - expected[1]).abs() < f32::EPSILON);
    }

    #[test]
    fn head_front_uses_standard_skin_region() {
        assert_uv(
            SkinBoxUv::HEAD
                .uv([-0.5, 0.5, 0.5], [0.0, 0.0, 1.0])
                .expect("head front top-left UV"),
            [8.0, 8.0],
        );
        assert_uv(
            SkinBoxUv::HEAD
                .uv([0.5, -0.5, 0.5], [0.0, 0.0, 1.0])
                .expect("head front bottom-right UV"),
            [16.0, 16.0],
        );
    }

    #[test]
    fn body_front_uses_standard_skin_region() {
        assert_eq!(
            SkinBoxUv::BODY.rect(SkinFace::Front),
            PixelRect::new(20.0, 20.0, 8.0, 12.0)
        );
        assert_eq!(
            SkinBoxUv::BODY_LAYER.rect(SkinFace::Front),
            PixelRect::new(20.0, 36.0, 8.0, 12.0)
        );
    }

    #[test]
    fn every_player_mesh_has_an_explicit_layout() {
        for name in [
            "HeadMesh",
            "HairLayer",
            "BodyMesh",
            "RightArmMesh",
            "LeftArmMesh",
            "RightLegMesh",
            "LeftLegMesh",
        ] {
            assert!(
                SkinPartSpec::for_mesh_name(name).is_some(),
                "missing skin layout for {name}"
            );
        }
    }

    #[test]
    fn limb_outer_layers_use_explicit_standard_regions() {
        assert_eq!(
            SkinBoxUv::RIGHT_ARM_LAYER.rect(SkinFace::Front),
            PixelRect::new(44.0, 36.0, 4.0, 12.0)
        );
        assert_eq!(
            SkinBoxUv::LEFT_ARM_LAYER.rect(SkinFace::Front),
            PixelRect::new(52.0, 52.0, 4.0, 12.0)
        );
        assert_eq!(
            SkinBoxUv::RIGHT_LEG_LAYER.rect(SkinFace::Front),
            PixelRect::new(4.0, 36.0, 4.0, 12.0)
        );
        assert_eq!(
            SkinBoxUv::LEFT_LEG_LAYER.rect(SkinFace::Front),
            PixelRect::new(4.0, 52.0, 4.0, 12.0)
        );
    }

    #[test]
    fn normalized_uvs_scale_to_every_supported_texture_size() {
        let uv = SkinBoxUv::BODY
            .uv([-0.5, 0.5, 0.5], [0.0, 0.0, 1.0])
            .expect("body front top-left UV");

        for size in [64.0, 128.0, 256.0, 512.0] {
            let scale = size / SKIN_ATLAS_SIZE;
            assert!((uv[0] * size - 20.0 * scale).abs() < f32::EPSILON);
            assert!((uv[1] * size - 20.0 * scale).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn body_outer_layer_expands_by_quarter_pixel_per_side() {
        let scale = SkinPartSpec::for_mesh_name("BodyMesh")
            .expect("body spec")
            .outer_layer
            .expect("body layer")
            .relative_scale();

        assert!((scale.x - 8.5 / 8.0).abs() < f32::EPSILON);
        assert!((scale.y - 12.5 / 12.0).abs() < f32::EPSILON);
        assert!((scale.z - 4.5 / 4.0).abs() < f32::EPSILON);
    }
}
