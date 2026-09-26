use bevy::{
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    prelude::*,
};

use crate::{
    content::{builtin_ids::ARTISANS_KIT_TOOL_ID, tool::ToolRegistry},
    player::hotbar::PlayerHotbar,
};

use super::{
    animation::{PlayerViewModel, base_viewmodel_transform},
    model::VIEW_MODEL_ARM_GRIP_Y,
};

// Match the Brush viewmodel: same grip in the animated arm, material pass,
// render layer and pivot-based positioning. The Artisan's Kit uses its hotbar icon.
const VIEW_MODEL_RENDER_LAYER: usize = 1;
const ARTISANS_KIT_DISPLAY_SIZE: f32 = 0.52;
const ARTISANS_KIT_GRIP_OFFSET: Vec2 = Vec2::new(-0.36, -0.36);
const ARTISANS_KIT_DISPLAY_ANGLE: f32 = 0.30;

#[derive(Component)]
pub(super) struct HeldArtisansKitRoot;

#[derive(Resource)]
pub(super) struct HeldArtisansKitAssets {
    mesh: Handle<Mesh>,
    icon: Handle<StandardMaterial>,
}

fn artisans_kit_sprite_transform() -> Transform {
    let rotation = Quat::from_rotation_z(ARTISANS_KIT_DISPLAY_ANGLE);
    let grip = ARTISANS_KIT_GRIP_OFFSET.extend(0.0) * ARTISANS_KIT_DISPLAY_SIZE;
    Transform::from_translation(-(rotation * grip)).with_rotation(rotation)
}

pub(super) fn setup_held_artisans_kit_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    tools: Res<ToolRegistry>,
) {
    let artisans_kit = tools.get(ARTISANS_KIT_TOOL_ID).expect("Artisan's Kit tool definition is required");
    let icon = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load(artisans_kit.icon.clone())),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: true,
        double_sided: true,
        ..default()
    });
    commands.insert_resource(HeldArtisansKitAssets {
        mesh: meshes.add(Rectangle::new(ARTISANS_KIT_DISPLAY_SIZE, ARTISANS_KIT_DISPLAY_SIZE)),
        icon,
    });
}

pub(super) fn spawn_held_artisans_kit(
    mut commands: Commands,
    viewmodels: Query<Entity, Added<PlayerViewModel>>,
    hotbar: Res<PlayerHotbar>,
    assets: Res<HeldArtisansKitAssets>,
) {
    let selected = hotbar.item_at(hotbar.selected_slot()) == Some(ARTISANS_KIT_TOOL_ID);
    let rotation = base_viewmodel_transform().rotation.inverse();
    for viewmodel in &viewmodels {
        commands.entity(viewmodel).with_children(|hand| {
            hand.spawn((
                HeldArtisansKitRoot,
                Transform::from_translation(Vec3::new(-0.08, VIEW_MODEL_ARM_GRIP_Y, 0.21))
                    .with_rotation(rotation),
                if selected { Visibility::Visible } else { Visibility::Hidden },
                RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
            ))
            .with_children(|artisans_kit| {
                artisans_kit.spawn((
                    Mesh3d(assets.mesh.clone()),
                    MeshMaterial3d(assets.icon.clone()),
                    artisans_kit_sprite_transform(),
                    RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                    NotShadowCaster,
                ));
            });
        });
    }
}

pub(super) fn sync_held_artisans_kit(
    hotbar: Res<PlayerHotbar>,
    mut roots: Query<&mut Visibility, With<HeldArtisansKitRoot>>,
) {
    if !hotbar.is_changed() {
        return;
    }
    let desired = if hotbar.item_at(hotbar.selected_slot()) == Some(ARTISANS_KIT_TOOL_ID) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut roots {
        if *visibility != desired {
            *visibility = desired;
        }
    }
}
