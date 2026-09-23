use bevy::{
    camera::visibility::RenderLayers,
    light::NotShadowCaster,
    prelude::*,
};

use crate::{
    content::{
        builtin_ids::{BRUSH_TOOL_ID, DYED_PROPERTY_ID},
        item::ItemRegistry,
        secondary_property::SecondaryPropertyRegistry,
        tool::ToolRegistry,
    },
    player::hotbar::PlayerHotbar,
    tools::BrushMode,
};

const HELD_SPRITE_SIZE: f32 = 0.52;
const HELD_TOOL_GRIP_OFFSET: Vec2 = Vec2::new(-0.36, -0.36);
const HELD_TOOL_DISPLAY_ANGLE: f32 = 0.30;
const HELD_TINT_DEPTH: f32 = 0.004;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HeldSpriteKind {
    Item,
    Tool,
}

struct HeldSpriteVisual<'a> {
    icon: &'a str,
    tint_icon: Option<&'a str>,
    tint: Option<Color>,
    kind: HeldSpriteKind,
}

#[derive(Resource)]
pub(crate) struct HeldSpriteMesh(pub(crate) Handle<Mesh>);

#[derive(Component)]
pub(crate) struct HeldSpriteRoot;

#[derive(Component)]
pub(crate) struct HeldSpriteBase;

#[derive(Component)]
pub(crate) struct HeldSpriteTint;

pub(crate) fn setup_held_sprite_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.insert_resource(HeldSpriteMesh(
        meshes.add(Rectangle::new(HELD_SPRITE_SIZE, HELD_SPRITE_SIZE)),
    ));
}

fn selected_visual<'a>(
    hotbar: &PlayerHotbar,
    items: &'a ItemRegistry,
    tools: &'a ToolRegistry,
    brush_mode: &BrushMode,
    properties: &SecondaryPropertyRegistry,
) -> Option<HeldSpriteVisual<'a>> {
    let item_id = hotbar.item_at(hotbar.selected_slot())?;

    if let Some(item) = items.get(item_id) {
        return Some(HeldSpriteVisual {
            icon: &item.icon,
            tint_icon: None,
            tint: None,
            kind: HeldSpriteKind::Item,
        });
    }

    let tool = tools.get(item_id)?;
    if tool.icon.is_empty() {
        return None;
    }

    let tint = if item_id == BRUSH_TOOL_ID {
        brush_mode
            .dye_id()
            .and_then(|dye| properties.get(DYED_PROPERTY_ID, dye))
            .map(|definition| definition.color.to_color())
    } else {
        None
    };

    Some(HeldSpriteVisual {
        icon: &tool.icon,
        tint_icon: tool.tint_icon.as_deref(),
        tint,
        kind: HeldSpriteKind::Tool,
    })
}

fn plane_transform(kind: HeldSpriteKind, depth: f32) -> Transform {
    match kind {
        HeldSpriteKind::Item => Transform::from_translation(Vec3::Z * depth),
        HeldSpriteKind::Tool => {
            let rotation = Quat::from_rotation_z(HELD_TOOL_DISPLAY_ANGLE);
            let grip = HELD_TOOL_GRIP_OFFSET.extend(0.0) * HELD_SPRITE_SIZE;
            Transform::from_translation(-(rotation * grip) + Vec3::Z * depth)
                .with_rotation(rotation)
        }
    }
}

fn base_material(
    visual: Option<&HeldSpriteVisual<'_>>,
    asset_server: &AssetServer,
) -> StandardMaterial {
    StandardMaterial {
        base_color_texture: visual.map(|visual| asset_server.load(visual.icon.to_owned())),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: true,
        double_sided: true,
        ..default()
    }
}

fn tint_material(
    visual: Option<&HeldSpriteVisual<'_>>,
    asset_server: &AssetServer,
) -> StandardMaterial {
    StandardMaterial {
        base_color: visual.and_then(|visual| visual.tint).unwrap_or(Color::WHITE),
        base_color_texture: visual
            .and_then(|visual| visual.tint_icon)
            .map(|path| asset_server.load(path.to_owned())),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        double_sided: true,
        ..default()
    }
}

pub(crate) fn spawn_held_sprite(
    parent: &mut ChildSpawnerCommands,
    root_transform: Transform,
    render_layers: RenderLayers,
    hotbar: &PlayerHotbar,
    items: &ItemRegistry,
    tools: &ToolRegistry,
    brush_mode: &BrushMode,
    properties: &SecondaryPropertyRegistry,
    asset_server: &AssetServer,
    mesh: &Handle<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> [Entity; 2] {
    let visual = selected_visual(hotbar, items, tools, brush_mode, properties);
    let kind = visual
        .as_ref()
        .map_or(HeldSpriteKind::Item, |visual| visual.kind);
    let root_visibility = if visual.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let tint_visibility = if visual
        .as_ref()
        .is_some_and(|visual| visual.tint_icon.is_some() && visual.tint.is_some())
    {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let base_material = materials.add(base_material(visual.as_ref(), asset_server));
    let tint_material = materials.add(tint_material(visual.as_ref(), asset_server));

    let mut renderables = [Entity::PLACEHOLDER; 2];
    parent
        .spawn((
            HeldSpriteRoot,
            root_transform,
            root_visibility,
            render_layers.clone(),
        ))
        .with_children(|root| {
            renderables[0] = root
                .spawn((
                    HeldSpriteBase,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(base_material),
                    plane_transform(kind, 0.0),
                    render_layers.clone(),
                    NotShadowCaster,
                ))
                .id();
            renderables[1] = root
                .spawn((
                    HeldSpriteTint,
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(tint_material),
                    plane_transform(kind, HELD_TINT_DEPTH),
                    tint_visibility,
                    render_layers,
                    NotShadowCaster,
                ))
                .id();
        });
    renderables
}

pub(crate) fn sync_held_sprites(
    hotbar: Res<PlayerHotbar>,
    items: Res<ItemRegistry>,
    tools: Res<ToolRegistry>,
    brush_mode: Res<BrushMode>,
    properties: Res<SecondaryPropertyRegistry>,
    asset_server: Res<AssetServer>,
    mut roots: Query<&mut Visibility, With<HeldSpriteRoot>>,
    mut bases: Query<
        (&MeshMaterial3d<StandardMaterial>, &mut Transform),
        (With<HeldSpriteBase>, Without<HeldSpriteTint>),
    >,
    mut tints: Query<
        (
            &MeshMaterial3d<StandardMaterial>,
            &mut Transform,
            &mut Visibility,
        ),
        (With<HeldSpriteTint>, Without<HeldSpriteBase>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !hotbar.is_changed() && !brush_mode.is_changed() && !properties.is_changed() {
        return;
    }

    let visual = selected_visual(&hotbar, &items, &tools, &brush_mode, &properties);
    let root_visibility = if visual.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut roots {
        if *visibility != root_visibility {
            *visibility = root_visibility;
        }
    }

    let Some(visual) = visual else {
        for (_, _, mut visibility) in &mut tints {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };

    let base_transform = plane_transform(visual.kind, 0.0);
    for (material_handle, mut transform) in &mut bases {
        if *transform != base_transform {
            *transform = base_transform;
        }
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.base_color = Color::WHITE;
            material.base_color_texture = Some(asset_server.load(visual.icon.to_owned()));
        }
    }

    let tint_transform = plane_transform(visual.kind, HELD_TINT_DEPTH);
    let tint_visibility = if visual.tint_icon.is_some() && visual.tint.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (material_handle, mut transform, mut visibility) in &mut tints {
        if *transform != tint_transform {
            *transform = tint_transform;
        }
        if *visibility != tint_visibility {
            *visibility = tint_visibility;
        }
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.base_color = visual.tint.unwrap_or(Color::WHITE);
            material.base_color_texture = visual
                .tint_icon
                .map(|path| asset_server.load(path.to_owned()));
        }
    }
}
