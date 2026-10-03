use std::path::Path;

use bevy::{
    camera::visibility::RenderLayers, light::NotShadowCaster, mesh::VertexAttributeValues,
    prelude::*,
};
use image::RgbaImage;

use crate::app::game_state::GameState;

use super::material::player_skin_texture_path;

const SKIN_ATLAS_SIZE: f32 = 64.0;
const FACE_NORMAL_THRESHOLD: f32 = 0.5;
const OUTER_LAYER_DEPTH_PIXELS: f32 = 0.25;
const SKIN_PIXEL_WORLD: f32 = 0.45 / 8.0;
const BODY_HALF_WIDTH_WORLD: f32 = SKIN_PIXEL_WORLD * 4.0;
const SLIM_ARM_WIDTH_WORLD: f32 = SKIN_PIXEL_WORLD * 3.0;
const SLIM_ARM_PIVOT_X: f32 = BODY_HALF_WIDTH_WORLD + SLIM_ARM_WIDTH_WORLD * 0.5;

#[derive(Component)]
struct PlayerSkinMapped;

#[derive(Component)]
struct PlayerSkinOuterLayer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayerSkinModel {
    Classic,
    Slim,
}

impl PlayerSkinModel {
    fn arm_width_pixels(self) -> f32 {
        match self {
            Self::Classic => 4.0,
            Self::Slim => 3.0,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug)]
struct PlayerSkinProfile {
    model: PlayerSkinModel,
}

impl FromWorld for PlayerSkinProfile {
    fn from_world(_world: &mut World) -> Self {
        let texture_path = player_skin_texture_path();
        let source_path = Path::new("assets").join(texture_path);
        let image = image::open(&source_path)
            .unwrap_or_else(|error| {
                panic!(
                    "failed to read player skin {}: {error}",
                    source_path.display()
                )
            })
            .to_rgba8();
        validate_skin_resolution(&image, &source_path);
        let model = infer_skin_model(&image);
        info!(
            "player skin selected path={} resolution={}x{} model={model:?}",
            texture_path,
            image.width(),
            image.height(),
        );
        Self { model }
    }
}

type PlayerSkinBaseMeshQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Mesh3d,
        &'static MeshMaterial3d<StandardMaterial>,
        &'static RenderLayers,
    ),
    (Without<PlayerSkinMapped>, Without<PlayerSkinOuterLayer>),
>;

type PlayerSkinParentLayersQuery<'w, 's> =
    Query<'w, 's, &'static RenderLayers, Without<PlayerSkinOuterLayer>>;
type PlayerSkinOuterLayerQuery<'w, 's> =
    Query<'w, 's, (&'static ChildOf, &'static mut RenderLayers), With<PlayerSkinOuterLayer>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayerSkinPart {
    Head,
    HeadLayer,
    Body,
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
}

impl PlayerSkinPart {
    fn from_node_name(name: &str) -> Option<Self> {
        match name {
            "HeadMesh" => Some(Self::Head),
            "HairLayer" => Some(Self::HeadLayer),
            "BodyMesh" => Some(Self::Body),
            "RightArmMesh" => Some(Self::RightArm),
            "LeftArmMesh" => Some(Self::LeftArm),
            "RightLegMesh" => Some(Self::RightLeg),
            "LeftLegMesh" => Some(Self::LeftLeg),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Head => "HeadMesh",
            Self::HeadLayer => "HairLayer",
            Self::Body => "BodyMesh",
            Self::RightArm => "RightArmMesh",
            Self::LeftArm => "LeftArmMesh",
            Self::RightLeg => "RightLegMesh",
            Self::LeftLeg => "LeftLegMesh",
        }
    }

    fn spec(self, model: PlayerSkinModel) -> PlayerSkinMeshSpec {
        let arm_width = model.arm_width_pixels();
        match self {
            Self::Head => PlayerSkinMeshSpec {
                base: SkinBoxUv::new(0.0, 0.0, 8.0, 8.0, 8.0),
                outer: None,
            },
            Self::HeadLayer => PlayerSkinMeshSpec {
                base: SkinBoxUv::new(32.0, 0.0, 8.0, 8.0, 8.0),
                outer: None,
            },
            Self::Body => PlayerSkinMeshSpec {
                base: SkinBoxUv::new(16.0, 16.0, 8.0, 12.0, 4.0),
                outer: Some(OuterLayerSpec {
                    name: "BodyLayer",
                    layout: SkinBoxUv::new(16.0, 32.0, 8.0, 12.0, 4.0),
                }),
            },
            Self::RightArm => PlayerSkinMeshSpec {
                base: SkinBoxUv::new(40.0, 16.0, arm_width, 12.0, 4.0),
                outer: Some(OuterLayerSpec {
                    name: "RightArmLayer",
                    layout: SkinBoxUv::new(40.0, 32.0, arm_width, 12.0, 4.0),
                }),
            },
            Self::LeftArm => PlayerSkinMeshSpec {
                base: SkinBoxUv::new(32.0, 48.0, arm_width, 12.0, 4.0),
                outer: Some(OuterLayerSpec {
                    name: "LeftArmLayer",
                    layout: SkinBoxUv::new(48.0, 48.0, arm_width, 12.0, 4.0),
                }),
            },
            Self::RightLeg => PlayerSkinMeshSpec {
                base: SkinBoxUv::new(0.0, 16.0, 4.0, 12.0, 4.0),
                outer: Some(OuterLayerSpec {
                    name: "RightLegLayer",
                    layout: SkinBoxUv::new(0.0, 32.0, 4.0, 12.0, 4.0),
                }),
            },
            Self::LeftLeg => PlayerSkinMeshSpec {
                base: SkinBoxUv::new(16.0, 48.0, 4.0, 12.0, 4.0),
                outer: Some(OuterLayerSpec {
                    name: "LeftLegLayer",
                    layout: SkinBoxUv::new(0.0, 48.0, 4.0, 12.0, 4.0),
                }),
            },
        }
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
            SkinFace::Top => PixelRect::new(self.x + self.depth, self.y, self.width, self.depth),
            SkinFace::Bottom => PixelRect::new(
                self.x + self.depth + self.width,
                self.y,
                self.width,
                self.depth,
            ),
        }
    }

    fn dimensions(self) -> Vec3 {
        Vec3::new(self.width, self.height, self.depth)
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
            Self::Back => [0.5 - x, 0.5 - y],
            Self::Right => [z + 0.5, 0.5 - y],
            Self::Left => [0.5 - z, 0.5 - y],
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

#[derive(Clone, Copy)]
struct OuterLayerSpec {
    name: &'static str,
    layout: SkinBoxUv,
}

impl OuterLayerSpec {
    fn relative_scale(self) -> Vec3 {
        let dimensions = self.layout.dimensions();
        let expansion = Vec3::splat(OUTER_LAYER_DEPTH_PIXELS * 2.0);
        (dimensions + expansion) / dimensions
    }
}

#[derive(Clone, Copy)]
struct PlayerSkinMeshSpec {
    base: SkinBoxUv,
    outer: Option<OuterLayerSpec>,
}

pub(crate) struct PlayerSkinUvPlugin;

impl Plugin for PlayerSkinUvPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerSkinProfile>().add_systems(
            Update,
            (configure_player_skin_meshes, sync_outer_layer_render_layers)
                .chain()
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn configure_player_skin_meshes(
    mut commands: Commands,
    profile: Res<PlayerSkinProfile>,
    mut meshes: ResMut<Assets<Mesh>>,
    base_meshes: PlayerSkinBaseMeshQuery,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    mut transforms: Query<&mut Transform>,
) {
    for (entity, mesh_handle, material_handle, render_layers) in &base_meshes {
        let Some((node_entity, part)) = resolve_player_skin_part(entity, &names, &parents) else {
            continue;
        };
        let spec = part.spec(profile.model);

        if profile.model == PlayerSkinModel::Slim {
            apply_slim_arm_geometry(node_entity, part, &parents, &mut transforms);
        }

        let Some(source_mesh) = meshes.get(mesh_handle.id()).cloned() else {
            continue;
        };
        let mut base_mesh = source_mesh.clone();
        if !remap_mesh_uvs(&mut base_mesh, spec.base) {
            warn!(
                "player skin mesh {} skipped: source mesh has unexpected vertex data",
                part.name()
            );
            commands.entity(entity).insert(PlayerSkinMapped);
            continue;
        }

        let base_mesh = meshes.add(base_mesh);
        commands
            .entity(entity)
            .insert((Mesh3d(base_mesh), PlayerSkinMapped));

        let Some(outer) = spec.outer else {
            continue;
        };
        let mut outer_mesh = source_mesh;
        if !remap_mesh_uvs(&mut outer_mesh, outer.layout) {
            warn!(
                "player skin outer layer {} skipped: source mesh has unexpected vertex data",
                outer.name
            );
            continue;
        }
        let outer_mesh = meshes.add(outer_mesh);

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Name::new(outer.name),
                PlayerSkinOuterLayer,
                Mesh3d(outer_mesh),
                material_handle.clone(),
                Transform::from_scale(outer.relative_scale()),
                Visibility::Inherited,
                render_layers.clone(),
                NotShadowCaster,
            ));
        });
    }
}

fn resolve_player_skin_part(
    mut entity: Entity,
    names: &Query<&Name>,
    parents: &Query<&ChildOf>,
) -> Option<(Entity, PlayerSkinPart)> {
    loop {
        if let Ok(name) = names.get(entity) {
            if let Some(part) = PlayerSkinPart::from_node_name(name.as_str()) {
                return Some((entity, part));
            }
            if name.as_str() == "PlayerRoot" {
                return None;
            }
        }

        let Ok(parent) = parents.get(entity) else {
            return None;
        };
        entity = parent.parent();
    }
}

fn apply_slim_arm_geometry(
    node_entity: Entity,
    part: PlayerSkinPart,
    parents: &Query<&ChildOf>,
    transforms: &mut Query<&mut Transform>,
) {
    let pivot_x = match part {
        PlayerSkinPart::RightArm => -SLIM_ARM_PIVOT_X,
        PlayerSkinPart::LeftArm => SLIM_ARM_PIVOT_X,
        _ => return,
    };

    if let Ok(mut arm_transform) = transforms.get_mut(node_entity) {
        arm_transform.scale.x = SLIM_ARM_WIDTH_WORLD;
    }
    let Ok(parent) = parents.get(node_entity) else {
        return;
    };
    if let Ok(mut pivot_transform) = transforms.get_mut(parent.parent()) {
        pivot_transform.translation.x = pivot_x;
    }
}

fn sync_outer_layer_render_layers(
    parent_layers: PlayerSkinParentLayersQuery,
    mut outer_layers: PlayerSkinOuterLayerQuery,
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

fn validate_skin_resolution(image: &RgbaImage, path: &Path) {
    let width = image.width();
    let height = image.height();
    assert!(
        width == height && matches!(width, 64 | 128 | 256 | 512),
        "player skin {} must be one of 64x64, 128x128, 256x256, or 512x512; got {width}x{height}",
        path.display()
    );
}

fn infer_skin_model(image: &RgbaImage) -> PlayerSkinModel {
    debug_assert_eq!(image.width(), image.height());
    let scale = image.width() / 64;
    let unused = [
        (50, 16, 2, 4),
        (54, 20, 2, 12),
        (42, 48, 2, 4),
        (46, 52, 2, 12),
    ];

    if unused
        .iter()
        .copied()
        .any(|rect| area_has_transparency(image, rect, scale))
        || unused
            .iter()
            .copied()
            .all(|rect| area_is_rgba(image, rect, scale, [0, 0, 0, 255]))
        || unused
            .iter()
            .copied()
            .all(|rect| area_is_rgba(image, rect, scale, [255, 255, 255, 255]))
    {
        PlayerSkinModel::Slim
    } else {
        PlayerSkinModel::Classic
    }
}

fn area_has_transparency(
    image: &RgbaImage,
    (x, y, width, height): (u32, u32, u32, u32),
    scale: u32,
) -> bool {
    pixels_in_area(image, (x, y, width, height), scale).any(|pixel| pixel[3] != 255)
}

fn area_is_rgba(
    image: &RgbaImage,
    (x, y, width, height): (u32, u32, u32, u32),
    scale: u32,
    expected: [u8; 4],
) -> bool {
    pixels_in_area(image, (x, y, width, height), scale).all(|pixel| pixel.0 == expected)
}

fn pixels_in_area(
    image: &RgbaImage,
    (x, y, width, height): (u32, u32, u32, u32),
    scale: u32,
) -> impl Iterator<Item = &image::Rgba<u8>> {
    let x_start = x * scale;
    let y_start = y * scale;
    let x_end = (x + width) * scale;
    let y_end = (y + height) * scale;

    (y_start..y_end).flat_map(move |py| (x_start..x_end).map(move |px| image.get_pixel(px, py)))
}

#[cfg(test)]
mod tests {
    use image::Rgba;

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
    fn authored_node_names_map_to_skin_parts() {
        for name in [
            "HeadMesh",
            "HairLayer",
            "BodyMesh",
            "RightArmMesh",
            "LeftArmMesh",
            "RightLegMesh",
            "LeftLegMesh",
        ] {
            assert!(PlayerSkinPart::from_node_name(name).is_some(), "{name}");
        }
    }

    #[test]
    fn primitive_display_names_do_not_masquerade_as_node_names() {
        assert_eq!(
            PlayerSkinPart::from_node_name("HeadMesh (PlayerSkin)"),
            None
        );
    }

    #[test]
    fn canonical_head_uvs_preserve_side_orientation() {
        let head = PlayerSkinPart::Head.spec(PlayerSkinModel::Classic).base;

        assert_uv(
            head.uv([-0.5, 0.5, 0.5], [0.0, 0.0, 1.0])
                .expect("front uv"),
            [8.0, 8.0],
        );
        assert_uv(
            head.uv([0.5, 0.5, -0.5], [0.0, 0.0, -1.0])
                .expect("back uv"),
            [24.0, 8.0],
        );
        assert_uv(
            head.uv([-0.5, 0.5, -0.5], [-1.0, 0.0, 0.0])
                .expect("right uv"),
            [0.0, 8.0],
        );
        assert_uv(
            head.uv([0.5, 0.5, 0.5], [1.0, 0.0, 0.0]).expect("left uv"),
            [16.0, 8.0],
        );
    }

    #[test]
    fn classic_and_slim_arms_use_expected_widths() {
        assert_eq!(
            PlayerSkinPart::RightArm
                .spec(PlayerSkinModel::Classic)
                .base
                .width,
            4.0
        );
        assert_eq!(
            PlayerSkinPart::RightArm
                .spec(PlayerSkinModel::Slim)
                .base
                .width,
            3.0
        );
    }

    #[test]
    fn transparent_unused_arm_pixels_identify_slim_skin() {
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([40, 50, 60, 255]));
        image.put_pixel(50, 16, Rgba([0, 0, 0, 0]));

        assert_eq!(infer_skin_model(&image), PlayerSkinModel::Slim);
    }

    #[test]
    fn populated_arm_pixels_identify_classic_skin() {
        let image = RgbaImage::from_pixel(64, 64, Rgba([40, 50, 60, 255]));

        assert_eq!(infer_skin_model(&image), PlayerSkinModel::Classic);
    }

    #[test]
    fn body_outer_layer_expands_by_quarter_pixel_per_side() {
        let spec = PlayerSkinPart::Body
            .spec(PlayerSkinModel::Classic)
            .outer
            .expect("body outer layer");
        let scale = spec.relative_scale();

        assert!((scale.x - 8.5 / 8.0).abs() < f32::EPSILON);
        assert!((scale.y - 12.5 / 12.0).abs() < f32::EPSILON);
        assert!((scale.z - 4.5 / 4.0).abs() < f32::EPSILON);
    }
}
