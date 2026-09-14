use bevy::prelude::*;

#[derive(Resource)]
pub(super) struct StarAssets {
    pub(super) mesh: Handle<Mesh>,
    pub(super) material: Handle<StandardMaterial>,
}

#[derive(Resource)]
pub(super) struct CloudAssets {
    pub(super) mesh: Handle<Mesh>,
    pub(super) material: Handle<StandardMaterial>,
}

pub(super) fn setup_sky_layer_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let star_mesh = meshes.add(Rectangle::new(1.0, 1.0));
    let star_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        fog_enabled: false,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.insert_resource(StarAssets {
        mesh: star_mesh,
        material: star_material,
    });

    let cloud_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cloud_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.78),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        fog_enabled: false,
        ..default()
    });
    commands.insert_resource(CloudAssets {
        mesh: cloud_mesh,
        material: cloud_material,
    });
}
