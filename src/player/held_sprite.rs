use bevy::{
    camera::visibility::RenderLayers,
    ecs::system::SystemParam,
    light::NotShadowCaster,
    prelude::*,
};

use crate::{
    content::{
        biome::BiomeRegistry,
        builtin_ids::{BIOME_TINT_METADATA_KEY, DYED_PROPERTY_ID},
        item::ItemRegistry,
        object::ObjectRegistry,
        secondary_property::SecondaryPropertyRegistry,
        tool::ToolRegistry,
        tool_behavior::BRUSH_PAINT_BEHAVIOR_ID,
    },
    player::{camera::GameplayCamera, hotbar::PlayerHotbar},
    rendering::block_tint::block_tint_at_with_override,
    tools::BrushMode,
    world::biome_field::BiomeField,
};

const HELD_SPRITE_SIZE: f32 = 0.52;
const HELD_TOOL_DISPLAY_ANGLE: f32 = 0.30;
const HELD_TINT_DEPTH: f32 = 0.004;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HeldSpriteKind {
    Item,
    Tool,
}

struct HeldSpriteVisual<'a> {
    icon: &'a str,
    base_color: Color,
    tint_icon: Option<&'a str>,
    tint: Option<Color>,
    kind: HeldSpriteKind,
}

#[derive(Resource)]
pub(crate) struct HeldSpriteMesh(pub(crate) Handle<Mesh>);

#[derive(Component)]
pub(crate) struct HeldSpriteRoot;

#[derive(Component)]
struct HeldSpriteBase;

#[derive(Component)]
struct HeldSpriteTint;

#[derive(SystemParam)]
pub(crate) struct HeldSpriteContent<'w, 's> {
    hotbar: Res<'w, PlayerHotbar>,
    items: Res<'w, ItemRegistry>,
    objects: Res<'w, ObjectRegistry>,
    biomes: Res<'w, BiomeRegistry>,
    biome_field: Res<'w, BiomeField>,
    player: Single<'w, 's, Ref<'static, Transform>, With<GameplayCamera>>,
    tools: Res<'w, ToolRegistry>,
    brush_mode: Res<'w, BrushMode>,
    properties: Res<'w, SecondaryPropertyRegistry>,
    asset_server: Res<'w, AssetServer>,
}

impl HeldSpriteContent<'_, '_> {
    fn selected_visual(&self) -> Option<HeldSpriteVisual<'_>> {
        let item_id = self.hotbar.item_at(self.hotbar.selected_slot())?;

        if let Some(item) = self.items.get(item_id) {
            return Some(HeldSpriteVisual {
                icon: &item.icon,
                base_color: Color::WHITE,
                tint_icon: None,
                tint: None,
                kind: HeldSpriteKind::Item,
            });
        }
        if let Some(object) = self.objects.get(item_id) {
            let biome_override = self
                .hotbar
                .stack_at(self.hotbar.selected_slot())
                .and_then(|stack| stack.metadata().get(BIOME_TINT_METADATA_KEY));
            let position = Vec2::new(self.player.translation.x, self.player.translation.z);
            return Some(HeldSpriteVisual {
                icon: &object.icon,
                base_color: block_tint_at_with_override(
                    object.tint,
                    position,
                    biome_override,
                    &self.biome_field,
                    &self.biomes,
                ),
                tint_icon: None,
                tint: None,
                kind: HeldSpriteKind::Item,
            });
        }

        let tool = self.tools.get(item_id)?;
        if tool.icon.is_empty() {
            return None;
        }

        let tint = if tool.uses_behavior(BRUSH_PAINT_BEHAVIOR_ID) {
            self.brush_mode
                .dye_id()
                .and_then(|dye| self.properties.get(DYED_PROPERTY_ID, dye))
                .map(|definition| definition.color.to_color())
        } else {
            None
        };

        Some(HeldSpriteVisual {
            icon: &tool.icon,
            base_color: Color::WHITE,
            tint_icon: tool.tint_icon.as_deref(),
            tint,
            kind: HeldSpriteKind::Tool,
        })
    }

    fn inputs_changed(&self) -> bool {
        self.hotbar.is_changed()
            || self.brush_mode.is_changed()
            || self.properties.is_changed()
            || self.biomes.is_changed()
            || self.biome_field.is_changed()
            || self.player.is_changed()
    }
}

#[derive(SystemParam)]
pub(crate) struct HeldSpriteAssets<'w> {
    mesh: Res<'w, HeldSpriteMesh>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

type HeldSpriteBaseQuery<'w, 's> = Query<
    'w,
    's,
    (&'static MeshMaterial3d<StandardMaterial>, &'static mut Transform),
    (With<HeldSpriteBase>, Without<HeldSpriteTint>),
>;

type HeldSpriteTintQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static MeshMaterial3d<StandardMaterial>,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<HeldSpriteTint>, Without<HeldSpriteBase>),
>;

#[derive(SystemParam)]
pub(crate) struct HeldSpriteSyncView<'w, 's> {
    roots: Query<
        'w,
        's,
        &'static mut Visibility,
        (With<HeldSpriteRoot>, Without<HeldSpriteTint>),
    >,
    bases: HeldSpriteBaseQuery<'w, 's>,
    tints: HeldSpriteTintQuery<'w, 's>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

pub(crate) fn setup_held_sprite_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.insert_resource(HeldSpriteMesh(
        meshes.add(Rectangle::new(HELD_SPRITE_SIZE, HELD_SPRITE_SIZE)),
    ));
}

fn plane_transform(kind: HeldSpriteKind, depth: f32) -> Transform {
    match kind {
        HeldSpriteKind::Item => Transform::from_translation(Vec3::Z * depth),
        HeldSpriteKind::Tool => Transform::from_translation(Vec3::Z * depth)
            .with_rotation(Quat::from_rotation_z(HELD_TOOL_DISPLAY_ANGLE)),
    }
}

fn base_material(
    visual: Option<&HeldSpriteVisual<'_>>,
    asset_server: &AssetServer,
) -> StandardMaterial {
    StandardMaterial {
        base_color: visual.map_or(Color::WHITE, |visual| visual.base_color),
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
    content: &HeldSpriteContent<'_, '_>,
    assets: &mut HeldSpriteAssets<'_>,
) -> [Entity; 2] {
    let visual = content.selected_visual();
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
    let base_material = assets.materials.add(base_material(
        visual.as_ref(),
        &content.asset_server,
    ));
    let tint_material = assets.materials.add(tint_material(
        visual.as_ref(),
        &content.asset_server,
    ));

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
                    Mesh3d(assets.mesh.0.clone()),
                    MeshMaterial3d(base_material),
                    plane_transform(kind, 0.0),
                    render_layers.clone(),
                    NotShadowCaster,
                ))
                .id();
            renderables[1] = root
                .spawn((
                    HeldSpriteTint,
                    Mesh3d(assets.mesh.0.clone()),
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
    content: HeldSpriteContent,
    mut view: HeldSpriteSyncView,
) {
    if !content.inputs_changed() {
        return;
    }

    let visual = content.selected_visual();
    let root_visibility = if visual.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut view.roots {
        if *visibility != root_visibility {
            *visibility = root_visibility;
        }
    }

    let Some(visual) = visual else {
        for (_, _, mut visibility) in &mut view.tints {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };

    let base_transform = plane_transform(visual.kind, 0.0);
    for (material_handle, mut transform) in &mut view.bases {
        if *transform != base_transform {
            *transform = base_transform;
        }
        if let Some(mut material) = view.materials.get_mut(&material_handle.0) {
            material.base_color = visual.base_color;
            material.base_color_texture =
                Some(content.asset_server.load(visual.icon.to_owned()));
        }
    }

    let tint_transform = plane_transform(visual.kind, HELD_TINT_DEPTH);
    let tint_visibility = if visual.tint_icon.is_some() && visual.tint.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for (material_handle, mut transform, mut visibility) in &mut view.tints {
        if *transform != tint_transform {
            *transform = tint_transform;
        }
        if *visibility != tint_visibility {
            *visibility = tint_visibility;
        }
        if let Some(mut material) = view.materials.get_mut(&material_handle.0) {
            material.base_color = visual.tint.unwrap_or(Color::WHITE);
            material.base_color_texture = visual
                .tint_icon
                .map(|path| content.asset_server.load(path.to_owned()));
        }
    }
}
