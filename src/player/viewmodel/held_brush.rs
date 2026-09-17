use bevy::{
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    prelude::*,
};

use crate::{
    content::{
        builtin_ids::{BRUSH_TOOL_ID, DYED_PROPERTY_ID},
        secondary_property::SecondaryPropertyRegistry,
        tool::ToolRegistry,
    },
    player::hotbar::PlayerHotbar,
    tools::BrushMode,
};

use super::animation::{PlayerViewModel, base_viewmodel_transform};

const VIEW_MODEL_RENDER_LAYER: usize = 1;
// The artwork contains the entire handle and bristles. Keep it close to the
// held-block footprint so the handle meets the hand rather than covering it.
const BRUSH_DISPLAY_SIZE: f32 = 0.30;
// A small camera-facing separation prevents coincident surfaces without
// noticeably changing the registration of the two 64x64 textures.
const BRUSH_TINT_DEPTH: f32 = 0.004;

#[derive(Component)]
pub(super) struct HeldBrushRoot;

#[derive(Component)]
pub(super) struct HeldBrushTint;

#[derive(Resource)]
pub(super) struct HeldBrushAssets {
    mesh: Handle<Mesh>,
    icon: Handle<StandardMaterial>,
    tint_icon: Option<Handle<StandardMaterial>>,
}

type HeldBrushOverlays<'w, 's> = Query<
    'w,
    's,
    (&'static MeshMaterial3d<StandardMaterial>, &'static mut Visibility),
    (With<HeldBrushTint>, Without<HeldBrushRoot>),
>;

fn selected_tint(mode: &BrushMode, properties: &SecondaryPropertyRegistry) -> Option<Color> {
    mode.dye_id()
        .and_then(|dye| properties.get(DYED_PROPERTY_ID, dye))
        .map(|definition| definition.color.to_color())
}

/// Load the same two images used by the hotbar; the overlay is hidden for an
/// unpainted Brush and its color is updated when the palette changes.
pub(super) fn setup_held_brush_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    tools: Res<ToolRegistry>,
    mode: Res<BrushMode>,
    properties: Res<SecondaryPropertyRegistry>,
) {
    let brush = tools.get(BRUSH_TOOL_ID).expect("Brush tool definition is required");
    let icon = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load(brush.icon.clone())),
        // The 64x64 base is pixel art: alpha masking preserves its silhouette
        // and draws it in the opaque pass, before the transparent dye overlay.
        // Two Blend materials can be depth-sorted in the opposite order to UI.
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: true,
        double_sided: true,
        ..default()
    });
    let tint_icon = brush.tint_icon.as_ref().map(|path| {
        materials.add(StandardMaterial {
            base_color: selected_tint(&mode, &properties).unwrap_or(Color::WHITE),
            base_color_texture: Some(asset_server.load(path.clone())),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            double_sided: true,
            ..default()
        })
    });
    commands.insert_resource(HeldBrushAssets {
        mesh: meshes.add(Rectangle::new(BRUSH_DISPLAY_SIZE, BRUSH_DISPLAY_SIZE)),
        icon,
        tint_icon,
    });
}

/// Attach the Brush to the same animated hand root as held blocks. A textured
/// flat mesh is intentional: the Brush artwork is a 64x64 item sprite.
pub(super) fn spawn_held_brush(
    mut commands: Commands,
    viewmodels: Query<Entity, Added<PlayerViewModel>>,
    hotbar: Res<PlayerHotbar>,
    mode: Res<BrushMode>,
    assets: Res<HeldBrushAssets>,
) {
    let selected = hotbar.item_at(hotbar.selected_slot()) == Some(BRUSH_TOOL_ID);
    let rotation = base_viewmodel_transform().rotation.inverse();
    for viewmodel in &viewmodels {
        commands.entity(viewmodel).with_children(|hand| {
            hand.spawn((
                HeldBrushRoot,
                Transform::from_translation(Vec3::new(-0.02, 0.72, 0.21))
                    .with_rotation(rotation),
                if selected { Visibility::Visible } else { Visibility::Hidden },
                RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
            ))
            .with_children(|brush| {
                brush.spawn((
                    Mesh3d(assets.mesh.clone()),
                    MeshMaterial3d(assets.icon.clone()),
                    Transform::IDENTITY,
                    RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                    NotShadowCaster,
                ));
                if let Some(material) = &assets.tint_icon {
                    brush.spawn((
                        HeldBrushTint,
                        Mesh3d(assets.mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        // Positive Z is towards the viewmodel camera; keep the
                        // dye on top of the base, as in the hotbar icon.
                        Transform::from_translation(Vec3::new(0.0, 0.0, BRUSH_TINT_DEPTH)),
                        if mode.dye_id().is_some() {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        },
                        RenderLayers::layer(VIEW_MODEL_RENDER_LAYER),
                        NotShadowCaster,
                    ));
                }
            });
        });
    }
}

pub(super) fn sync_held_brush(
    hotbar: Res<PlayerHotbar>,
    mode: Res<BrushMode>,
    properties: Res<SecondaryPropertyRegistry>,
    mut roots: Query<&mut Visibility, (With<HeldBrushRoot>, Without<HeldBrushTint>)>,
    mut overlays: HeldBrushOverlays,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !hotbar.is_changed() && !mode.is_changed() && !properties.is_changed() {
        return;
    }
    let selected = hotbar.item_at(hotbar.selected_slot()) == Some(BRUSH_TOOL_ID);
    let visibility = if selected { Visibility::Visible } else { Visibility::Hidden };
    for mut current in &mut roots {
        if *current != visibility {
            *current = visibility;
        }
    }

    let tint = selected_tint(&mode, &properties);
    for (material_handle, mut current) in &mut overlays {
        let desired = if tint.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *current != desired {
            *current = desired;
        }
        if let Some(color) = tint
            && let Some(mut material) = materials.get_mut(&material_handle.0)
            && material.base_color != color
        {
            material.base_color = color;
        }
    }
}
