use bevy::{
    camera::visibility::RenderLayers,
    ecs::system::SystemParam,
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    world_serialization::{WorldAssetRoot, WorldInstanceReady},
};

use crate::{
    content::{
        builtin_ids::DYED_PROPERTY_ID,
        biome::BiomeRegistry,
        item::ItemRegistry,
        object::{ObjectRegistry, ObjectVisualDefinition},
        object_id::intern_object_id,
        secondary_property::SecondaryPropertyRegistry,
        tool::ToolRegistry,
        tool_behavior::BRUSH_PAINT_BEHAVIOR_ID,
    },
    player::{
        camera::{CameraPerspective, GameplayCamera},
        hotbar::PlayerHotbar,
    },
    rendering::block_tint::block_tint_at,
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

#[derive(Component)]
struct HeldObjectModelAnchor;

#[derive(Component)]
struct HeldObjectModelRoot {
    object_id: &'static str,
}

#[derive(Component, Clone)]
struct HeldObjectModelAppearance {
    object_id: &'static str,
    render_layers: RenderLayers,
    dynamic_third_person_layers: bool,
    unlit: bool,
    casts_shadow: bool,
    receives_shadow: bool,
}

#[derive(Component)]
struct HeldObjectModelMaterial {
    object_id: &'static str,
}

#[derive(Component)]
struct HeldObjectDynamicRenderLayer;

#[derive(SystemParam)]
pub(crate) struct HeldSpriteContent<'w> {
    hotbar: Res<'w, PlayerHotbar>,
    items: Res<'w, ItemRegistry>,
    objects: Res<'w, ObjectRegistry>,
    tools: Res<'w, ToolRegistry>,
    brush_mode: Res<'w, BrushMode>,
    properties: Res<'w, SecondaryPropertyRegistry>,
    asset_server: Res<'w, AssetServer>,
}

impl HeldSpriteContent<'_> {
    fn selected_visual(&self) -> Option<HeldSpriteVisual<'_>> {
        let item_id = self.hotbar.item_at(self.hotbar.selected_slot())?;

        if let Some(item) = self.items.get(item_id) {
            return Some(HeldSpriteVisual {
                icon: &item.icon,
                tint_icon: None,
                tint: None,
                kind: HeldSpriteKind::Item,
            });
        }
        if let Some(object) = self.objects.get(item_id) {
            return match &object.visual {
                ObjectVisualDefinition::Model { .. } => None,
                ObjectVisualDefinition::SpritePrism { .. } => Some(HeldSpriteVisual {
                    icon: &object.icon,
                    tint_icon: None,
                    tint: None,
                    kind: HeldSpriteKind::Item,
                }),
            };
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
            tint_icon: tool.tint_icon.as_deref(),
            tint,
            kind: HeldSpriteKind::Tool,
        })
    }

    fn inputs_changed(&self) -> bool {
        self.hotbar.is_changed()
            || self.objects.is_changed()
            || self.brush_mode.is_changed()
            || self.properties.is_changed()
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

#[derive(SystemParam)]
pub(crate) struct HeldObjectModelSyncView<'w, 's> {
    roots: Query<'w, 's, (&'static HeldObjectModelRoot, &'static mut Visibility)>,
    model_materials: Query<
        'w,
        's,
        (
            &'static HeldObjectModelMaterial,
            &'static MeshMaterial3d<StandardMaterial>,
        ),
    >,
    biome_field: Res<'w, BiomeField>,
    biomes: Res<'w, BiomeRegistry>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

#[derive(SystemParam)]
pub(crate) struct HeldObjectDynamicRenderView<'w, 's> {
    renderables: Query<
        'w,
        's,
        &'static mut RenderLayers,
        With<HeldObjectDynamicRenderLayer>,
    >,
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
    dynamic_third_person_layers: bool,
    content: &HeldSpriteContent<'_>,
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
    spawn_held_object_models(
        parent,
        root_transform,
        render_layers.clone(),
        dynamic_third_person_layers,
        content,
    );

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

fn spawn_held_object_models(
    parent: &mut ChildSpawnerCommands,
    root_transform: Transform,
    render_layers: RenderLayers,
    dynamic_third_person_layers: bool,
    content: &HeldSpriteContent<'_>,
) {
    let selected_item = content.hotbar.item_at(content.hotbar.selected_slot());

    parent
        .spawn((
            HeldObjectModelAnchor,
            root_transform,
            Visibility::Inherited,
        ))
        .with_children(|anchor| {
            for object in content.objects.iter() {
                let ObjectVisualDefinition::Model { path } = &object.visual else {
                    continue;
                };
                let object_id = intern_object_id(&object.id);
                let scene = content
                    .asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset(path.clone()));
                let target_size = Vec3::from_array(object.target.size);
                let maximum_extent = target_size.max_element().max(0.001);
                let scale = HELD_SPRITE_SIZE / maximum_extent;
                let target_center = Vec3::from_array(object.target.center_offset);
                let transform = Transform::from_translation(-target_center * scale)
                    .with_scale(Vec3::splat(scale));
                let visibility = if selected_item == Some(object_id) {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };

                anchor
                    .spawn((
                        HeldObjectModelRoot { object_id },
                        WorldAssetRoot(scene),
                        transform,
                        visibility,
                        HeldObjectModelAppearance {
                            object_id,
                            render_layers: render_layers.clone(),
                            dynamic_third_person_layers,
                            unlit: object.unlit,
                            casts_shadow: object.casts_shadow,
                            receives_shadow: object.receives_shadow,
                        },
                    ))
                    .observe(configure_loaded_held_object_scene);
            }
        });
}

fn configure_loaded_held_object_scene(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    appearances: Query<&HeldObjectModelAppearance>,
    mesh_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(appearance) = appearances.get(ready.entity) else {
        return;
    };

    for descendant in children.iter_descendants(ready.entity) {
        if appearance.dynamic_third_person_layers {
            commands.entity(descendant).insert((
                HeldObjectDynamicRenderLayer,
                RenderLayers::from_layers(&[]),
            ));
        } else {
            commands
                .entity(descendant)
                .insert(appearance.render_layers.clone());
        }

        if !appearance.casts_shadow {
            commands.entity(descendant).insert(NotShadowCaster);
        }
        if !appearance.receives_shadow {
            commands.entity(descendant).insert(NotShadowReceiver);
        }

        let Ok(original) = mesh_materials.get(descendant) else {
            continue;
        };
        let Some(mut material) = materials.get(original.id()).cloned() else {
            continue;
        };
        material.unlit = appearance.unlit;
        let material = materials.add(material);
        commands.entity(descendant).insert((
            MeshMaterial3d(material),
            HeldObjectModelMaterial {
                object_id: appearance.object_id,
            },
        ));
    }
}

pub(crate) fn sync_held_object_models(
    content: HeldSpriteContent,
    player: Single<&Transform, With<GameplayCamera>>,
    mut view: HeldObjectModelSyncView,
    mut tint_cell: Local<Option<IVec2>>,
) {
    let selected = content.hotbar.item_at(content.hotbar.selected_slot());
    for (root, mut visibility) in &mut view.roots {
        let next = if selected == Some(root.object_id) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != next {
            *visibility = next;
        }
    }

    let current_cell = player.translation.xz().floor().as_ivec2();
    let inputs_changed = content.hotbar.is_changed()
        || content.objects.is_changed()
        || view.biome_field.is_changed()
        || view.biomes.is_changed()
        || *tint_cell != Some(current_cell);
    if !inputs_changed {
        return;
    }
    *tint_cell = Some(current_cell);

    let Some(object_id) = selected else {
        return;
    };
    let Some(object) = content.objects.get(object_id) else {
        return;
    };
    if !matches!(&object.visual, ObjectVisualDefinition::Model { .. }) {
        return;
    }

    let tint = block_tint_at(
        object.tint,
        current_cell.as_vec2() + Vec2::splat(0.5),
        &view.biome_field,
        &view.biomes,
    );
    for (marker, handle) in &view.model_materials {
        if marker.object_id != object_id {
            continue;
        }
        if let Some(mut material) = view.materials.get_mut(&handle.0) {
            material.base_color = tint;
            material.unlit = object.unlit;
        }
    }
}

pub(crate) fn sync_held_object_dynamic_render_layers(
    perspective: Res<CameraPerspective>,
    mut view: HeldObjectDynamicRenderView,
) {
    let layers = if perspective.is_third_person() {
        RenderLayers::layer(0)
    } else {
        RenderLayers::from_layers(&[])
    };
    for mut render_layers in &mut view.renderables {
        if *render_layers != layers {
            *render_layers = layers.clone();
        }
    }
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
            material.base_color = Color::WHITE;
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
