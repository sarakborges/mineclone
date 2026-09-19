use bevy::{
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    prelude::*,
};

use crate::{
    content::{builtin_ids::CHISEL_TOOL_ID, tool::ToolRegistry},
    player::hotbar::PlayerHotbar,
};

use super::animation::{PlayerViewModel, base_viewmodel_transform};

// Match the Brush viewmodel: same grip in the animated arm, material pass,
// render layer and pivot-based positioning. The Chisel uses its hotbar icon.
const VIEW_MODEL_RENDER_LAYER: usize = 1;
const CHISEL_DISPLAY_SIZE: f32 = 0.52;
const CHISEL_GRIP_OFFSET: Vec2 = Vec2::new(-0.36, -0.36);
const CHISEL_DISPLAY_ANGLE: f32 = 0.30;

#[derive(Component)]
pub(super) struct HeldChiselRoot;

#[derive(Resource)]
pub(super) struct HeldChiselAssets {
    mesh: Handle<Mesh>,
    icon: Handle<StandardMaterial>,
}

fn chisel_sprite_transform() -> Transform {
    let rotation = Quat::from_rotation_z(CHISEL_DISPLAY_ANGLE);
    let grip = CHISEL_GRIP_OFFSET.extend(0.0) * CHISEL_DISPLAY_SIZE;
    Transform::from_translation(-(rotation * grip)).with_rotation(rotation)
}

pub(super) fn setup_held_chisel_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    tools: Res<ToolRegistry>,
) {
    let chisel = tools.get(CHISEL_TOOL_ID).expect("Chisel tool definition is required");
    let icon = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load(chisel.icon.clone())),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: true,
        double_sided: true,
        ..default()
    });
    commands.insert_resource(HeldChiselAssets {
        mesh: meshes.add(Rectangle::new(CHISEL_DISPLAY_SIZE, CHISEL_DISPLAY_SIZE)),
        icon,
    });
}

pub(super) fn spawn_held_chisel(
    mut commands: Commands,
    viewmodels: Query<Entity, Added<PlayerViewModel>>,
    hotbar: Res<PlayerHotbar>,
    assets: Res<HeldChiselAssets>,
) {
    let selected = hotbar.item_at(hotbar.selected_slot()) == Some(CHISEL_TOOL_ID);
    let rotation = base_viewmodel_transform().rotation.inverse();
    for viewmodel in &viewmodels {
        commands.entity(viewmodel).with_children(|hand| {
            hand.spawn((
                HeldChiselRoot,
                Transform::from_translation(Vec3::new(-0.08, 0.46, 0.21))
                    .with_rotation(rotation),
                if selected { Visibility::Visible } else { Visibility::Hidden },
                RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
            ))
            .with_children(|chisel| {
                chisel.spawn((
                    Mesh3d(assets.mesh.clone()),
                    MeshMaterial3d(assets.icon.clone()),
                    chisel_sprite_transform(),
                    RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                    NotShadowCaster,
                ));
            });
        });
    }
}

pub(super) fn sync_held_chisel(
    hotbar: Res<PlayerHotbar>,
    mut roots: Query<&mut Visibility, With<HeldChiselRoot>>,
) {
    if !hotbar.is_changed() {
        return;
    }
    let desired = if hotbar.item_at(hotbar.selected_slot()) == Some(CHISEL_TOOL_ID) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut roots {
        if *visibility != desired {
            *visibility = desired;
        }
    }
}
