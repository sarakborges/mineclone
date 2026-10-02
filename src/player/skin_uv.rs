use bevy::{mesh::VertexAttributeValues, prelude::*};

use crate::app::game_state::GameState;

const SKIN_ATLAS_SIZE: f32 = 64.0;
const FACE_NORMAL_THRESHOLD: f32 = 0.5;

pub(crate) struct PlayerSkinUvPlugin;

impl Plugin for PlayerSkinUvPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            remap_player_skin_meshes.run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn remap_player_skin_meshes(
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_entities: Query<(Entity, &Mesh3d), Added<Mesh3d>>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
) {
    for (entity, mesh_handle) in &mesh_entities {
        let Some(layout) = player_skin_layout_for_entity(entity, &names, &parents) else {
            continue;
        };
        let Some(mut mesh) = meshes.get_mut(mesh_handle.id()) else {
            continue;
        };

        if !remap_mesh_uvs(&mut mesh, layout) {
            warn!(
                "player skin UV remap skipped for entity {entity:?}: unexpected mesh vertex data"
            );
        }
    }
}

fn player_skin_layout_for_entity(
    mut entity: Entity,
    names: &Query<&Name>,
    parents: &Query<&ChildOf>,
) -> Option<SkinBoxUv> {
    let mut layout = None;

    loop {
        if let Ok(name) = names.get(entity) {
            if layout.is_none() {
                layout = SkinBoxUv::for_node(name.as_str());
            }
            if name.as_str() == "PlayerRoot" {
                return layout;
            }
        }

        let Ok(parent) = parents.get(entity) else {
            return None;
        };
        entity = parent.parent();
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
    const RIGHT_ARM: Self = Self::new(40.0, 16.0, 4.0, 12.0, 4.0);
    const LEFT_ARM: Self = Self::new(32.0, 48.0, 4.0, 12.0, 4.0);
    const RIGHT_LEG: Self = Self::new(0.0, 16.0, 4.0, 12.0, 4.0);
    const LEFT_LEG: Self = Self::new(16.0, 48.0, 4.0, 12.0, 4.0);

    const fn new(x: f32, y: f32, width: f32, height: f32, depth: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            depth,
        }
    }

    fn for_node(name: &str) -> Option<Self> {
        match name {
            "HeadMesh" => Some(Self::HEAD),
            "HairLayer" => Some(Self::HEAD_LAYER),
            "BodyMesh" => Some(Self::BODY),
            "RightArmMesh" => Some(Self::RIGHT_ARM),
            "LeftArmMesh" => Some(Self::LEFT_ARM),
            "RightLegMesh" => Some(Self::RIGHT_LEG),
            "LeftLegMesh" => Some(Self::LEFT_LEG),
            _ => None,
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
    fn head_front_uses_standard_minecraft_face() {
        assert_uv(
            SkinBoxUv::HEAD
                .uv([-0.5, 0.5, 0.5], [0.0, 0.0, 1.0])
                .unwrap(),
            [8.0, 8.0],
        );
        assert_uv(
            SkinBoxUv::HEAD
                .uv([0.5, -0.5, 0.5], [0.0, 0.0, 1.0])
                .unwrap(),
            [16.0, 16.0],
        );
    }

    #[test]
    fn torso_faces_use_standard_minecraft_regions() {
        assert_eq!(
            SkinBoxUv::BODY.rect(SkinFace::Right),
            PixelRect::new(16.0, 20.0, 4.0, 12.0)
        );
        assert_eq!(
            SkinBoxUv::BODY.rect(SkinFace::Front),
            PixelRect::new(20.0, 20.0, 8.0, 12.0)
        );
        assert_eq!(
            SkinBoxUv::BODY.rect(SkinFace::Left),
            PixelRect::new(28.0, 20.0, 4.0, 12.0)
        );
        assert_eq!(
            SkinBoxUv::BODY.rect(SkinFace::Back),
            PixelRect::new(32.0, 20.0, 8.0, 12.0)
        );
        assert_eq!(
            SkinBoxUv::BODY.rect(SkinFace::Top),
            PixelRect::new(20.0, 16.0, 8.0, 4.0)
        );
        assert_eq!(
            SkinBoxUv::BODY.rect(SkinFace::Bottom),
            PixelRect::new(28.0, 16.0, 8.0, 4.0)
        );
    }

    #[test]
    fn all_authored_player_meshes_have_skin_layouts() {
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
                SkinBoxUv::for_node(name).is_some(),
                "missing layout for {name}"
            );
        }
    }
}
