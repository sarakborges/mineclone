use bevy::{
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    mesh::VertexAttributeValues,
    prelude::*,
};

use crate::app::game_state::GameState;

const SKIN_ATLAS_SIZE: f32 = 64.0;
const OUTER_LAYER_DEPTH_PIXELS: f32 = 0.25;

#[derive(Component)]
struct PlayerSkinOuterLayer;

type PlayerSkinBaseMeshQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Name,
        &'static Mesh3d,
        &'static MeshMaterial3d<StandardMaterial>,
        &'static RenderLayers,
    ),
    Added<RenderLayers>,
>;

type PlayerSkinParentLayersQuery<'w, 's> =
    Query<'w, 's, &'static RenderLayers, Without<PlayerSkinOuterLayer>>;
type PlayerSkinOuterLayerQuery<'w, 's> = Query<
    'w,
    's,
    (&'static ChildOf, &'static mut RenderLayers),
    With<PlayerSkinOuterLayer>,
>;

#[derive(Clone, Copy, Debug, PartialEq)]
struct OuterLayerSpec {
    name: &'static str,
    uv_offset_pixels: Vec2,
    dimensions_pixels: Vec3,
}

impl OuterLayerSpec {
    fn for_base_mesh(name: &str) -> Option<Self> {
        match name {
            "BodyMesh" => Some(Self {
                name: "BodyLayer",
                uv_offset_pixels: Vec2::new(0.0, 16.0),
                dimensions_pixels: Vec3::new(8.0, 12.0, 4.0),
            }),
            "RightArmMesh" => Some(Self {
                name: "RightArmLayer",
                uv_offset_pixels: Vec2::new(0.0, 16.0),
                dimensions_pixels: Vec3::new(4.0, 12.0, 4.0),
            }),
            "LeftArmMesh" => Some(Self {
                name: "LeftArmLayer",
                uv_offset_pixels: Vec2::new(16.0, 0.0),
                dimensions_pixels: Vec3::new(4.0, 12.0, 4.0),
            }),
            "RightLegMesh" => Some(Self {
                name: "RightLegLayer",
                uv_offset_pixels: Vec2::new(0.0, 16.0),
                dimensions_pixels: Vec3::new(4.0, 12.0, 4.0),
            }),
            "LeftLegMesh" => Some(Self {
                name: "LeftLegLayer",
                uv_offset_pixels: Vec2::new(-16.0, 0.0),
                dimensions_pixels: Vec3::new(4.0, 12.0, 4.0),
            }),
            _ => None,
        }
    }

    fn uv_offset(self) -> Vec2 {
        self.uv_offset_pixels / SKIN_ATLAS_SIZE
    }

    fn relative_scale(self) -> Vec3 {
        let expansion = Vec3::splat(OUTER_LAYER_DEPTH_PIXELS * 2.0);
        (self.dimensions_pixels + expansion) / self.dimensions_pixels
    }
}

pub(crate) struct PlayerSkinUvPlugin;

impl Plugin for PlayerSkinUvPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_player_skin_outer_layers, sync_outer_layer_render_layers)
                .chain()
                .run_if(in_state(GameState::Gameplay)),
        );
    }
}

fn spawn_player_skin_outer_layers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    base_meshes: PlayerSkinBaseMeshQuery,
) {
    for (entity, name, mesh_handle, material_handle, render_layers) in &base_meshes {
        let Some(spec) = OuterLayerSpec::for_base_mesh(name.as_str()) else {
            continue;
        };

        let Some(source_mesh) = meshes.get(mesh_handle.id()) else {
            warn!(
                "player skin outer layer {} skipped: source mesh is unavailable",
                spec.name
            );
            continue;
        };
        let mut outer_mesh = source_mesh.clone();
        if !shift_mesh_uvs(&mut outer_mesh, spec.uv_offset()) {
            warn!(
                "player skin outer layer {} skipped: source mesh has unexpected UV data",
                spec.name
            );
            continue;
        }
        let outer_mesh = meshes.add(outer_mesh);

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Name::new(spec.name),
                PlayerSkinOuterLayer,
                Mesh3d(outer_mesh),
                material_handle.clone(),
                Transform::from_scale(spec.relative_scale()),
                Visibility::Inherited,
                render_layers.clone(),
                NotShadowCaster,
            ));
        });
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

fn shift_mesh_uvs(mesh: &mut Mesh, offset: Vec2) -> bool {
    let shifted = {
        let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            return false;
        };

        uvs.iter()
            .map(|uv| [uv[0] + offset.x, uv[1] + offset.y])
            .collect::<Vec<_>>()
    };

    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, shifted);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outer_layers_use_standard_minecraft_regions() {
        assert_eq!(
            OuterLayerSpec::for_base_mesh("BodyMesh")
                .expect("body layer")
                .uv_offset_pixels,
            Vec2::new(0.0, 16.0)
        );
        assert_eq!(
            OuterLayerSpec::for_base_mesh("RightArmMesh")
                .expect("right arm layer")
                .uv_offset_pixels,
            Vec2::new(0.0, 16.0)
        );
        assert_eq!(
            OuterLayerSpec::for_base_mesh("LeftArmMesh")
                .expect("left arm layer")
                .uv_offset_pixels,
            Vec2::new(16.0, 0.0)
        );
        assert_eq!(
            OuterLayerSpec::for_base_mesh("RightLegMesh")
                .expect("right leg layer")
                .uv_offset_pixels,
            Vec2::new(0.0, 16.0)
        );
        assert_eq!(
            OuterLayerSpec::for_base_mesh("LeftLegMesh")
                .expect("left leg layer")
                .uv_offset_pixels,
            Vec2::new(-16.0, 0.0)
        );
    }

    #[test]
    fn body_outer_layer_expands_by_quarter_pixel_per_side() {
        let scale = OuterLayerSpec::for_base_mesh("BodyMesh")
            .expect("body layer")
            .relative_scale();

        assert!((scale.x - 8.5 / 8.0).abs() < f32::EPSILON);
        assert!((scale.y - 12.5 / 12.0).abs() < f32::EPSILON);
        assert!((scale.z - 4.5 / 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn head_layer_is_authored_by_the_model_not_duplicated() {
        assert!(OuterLayerSpec::for_base_mesh("HeadMesh").is_none());
        assert!(OuterLayerSpec::for_base_mesh("HairLayer").is_none());
    }
}
