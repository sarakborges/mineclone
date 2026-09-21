use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::{
    content::{
        block::{BlockDefinition, BlockRegistry, BlockTextureLayer, BlockTint},
        fluid::{FluidId, FluidRegistry},
        layer::{LayerDefinition, LayerRegistry},
    },
    rendering::{
        block_texture::{
            TerrainTextureTable, block_face_texture_layers, load_block_texture_layer,
            terrain_array_alpha_signature,
        },
        terrain_material::{
            TerrainLightingBuffer, TerrainMaterial, TerrainMaterialExtension,
        },
    },
    voxel::block_face::{BlockFace, BlockFaces},
};

const TEXTURE_LAYER_DEPTH_BIAS: f32 = 2.0;
const TERRAIN_TEXTURE_SIZE: u32 = 64;
const TERRAIN_TEXTURE_BYTES: usize =
    TERRAIN_TEXTURE_SIZE as usize * TERRAIN_TEXTURE_SIZE as usize * 4;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum TerrainAlphaKey {
    Opaque,
    Mask(u32),
    Blend,
}

impl TerrainAlphaKey {
    fn for_layer(definition: &BlockDefinition, layer_index: usize) -> Self {
        if layer_index > 0 || definition.alpha_blend {
            Self::Blend
        } else if let Some(cutoff) = definition.alpha_cutoff {
            Self::Mask(cutoff.to_bits())
        } else {
            Self::Opaque
        }
    }

    fn alpha_mode(self) -> AlphaMode {
        match self {
            Self::Opaque => AlphaMode::Opaque,
            Self::Mask(cutoff) => AlphaMode::Mask(f32::from_bits(cutoff)),
            Self::Blend => AlphaMode::Blend,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct TerrainMaterialKey {
    texture: Option<String>,
    overlay_texture: Option<String>,
    base_tint_enabled: bool,
    overlay_tint_enabled: bool,
    alpha: TerrainAlphaKey,
    layer_index: usize,
}


struct TerrainMaterialBuilder<'a> {
    asset_server: &'a AssetServer,
    materials: &'a mut Assets<TerrainMaterial>,
    cache: HashMap<TerrainMaterialKey, Handle<TerrainMaterial>>,
    lighting: &'a TerrainLightingBuffer,
    roughness: f32,
    metallic: f32,
    texture_array: Handle<Image>,
}

impl<'a> TerrainMaterialBuilder<'a> {
    fn new(
        asset_server: &'a AssetServer,
        materials: &'a mut Assets<TerrainMaterial>,
        lighting: &'a TerrainLightingBuffer,
        roughness: f32,
        metallic: f32,
        texture_array: Handle<Image>,
    ) -> Self {
        Self {
            asset_server,
            materials,
            cache: HashMap::new(),
            lighting,
            roughness,
            metallic,
            texture_array,
        }
    }

    fn array_material(&mut self, alpha: TerrainAlphaKey) -> Handle<TerrainMaterial> {
        self.materials.add(TerrainMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: self.roughness,
                metallic: self.metallic,
                alpha_mode: alpha.alpha_mode(),
                fog_enabled: true,
                unlit: true,
                ..default()
            },
            extension: TerrainMaterialExtension {
                lighting: self.lighting.handle(),
                fluid_animation_factor: 0.0,
                base_tint_enabled: 0.0,
                overlay_enabled: 0.0,
                overlay_tint_enabled: 0.0,
                texture_array_enabled: 1.0,
                overlay_texture: None,
                terrain_texture_array: self.texture_array.clone(),
            },
        })
    }

    fn layers_for(
        &mut self,
        definition: &BlockDefinition,
        face: BlockFace,
    ) -> Vec<Handle<TerrainMaterial>> {
        let layers = block_face_texture_layers(face, definition);
        if layers.len() <= 2 {
            return vec![self.composite_material_for(definition, layers)];
        }

        layers
            .iter()
            .enumerate()
            .map(|(layer_index, layer)| {
                self.material_for(definition, Some(layer), layer_index)
            })
            .collect()
    }

    fn composite_material_for(
        &mut self,
        definition: &BlockDefinition,
        layers: &[BlockTextureLayer],
    ) -> Handle<TerrainMaterial> {
        let base = layers.first();
        let overlay = layers.get(1);
        let alpha = TerrainAlphaKey::for_layer(definition, 0);
        let key = TerrainMaterialKey {
            texture: base.map(|layer| layer.texture.clone()),
            overlay_texture: overlay.map(|layer| layer.texture.clone()),
            base_tint_enabled: base.is_some_and(|layer| layer.dyable),
            overlay_tint_enabled: overlay.is_some_and(|layer| layer.dyable),
            alpha,
            layer_index: 0,
        };
        if let Some(existing) = self.cache.get(&key) {
            return existing.clone();
        }

        let material = self.materials.add(TerrainMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: base.map(|layer| load_block_texture_layer(self.asset_server, layer)),
                perceptual_roughness: self.roughness,
                metallic: self.metallic,
                alpha_mode: alpha.alpha_mode(),
                fog_enabled: true,
                unlit: true,
                ..default()
            },
            extension: TerrainMaterialExtension {
                lighting: self.lighting.handle(),
                fluid_animation_factor: 0.0,
                base_tint_enabled: key.base_tint_enabled as u8 as f32,
                overlay_enabled: overlay.is_some() as u8 as f32,
                overlay_tint_enabled: key.overlay_tint_enabled as u8 as f32,
                texture_array_enabled: 0.0,
                overlay_texture: overlay.map(|layer| load_block_texture_layer(self.asset_server, layer)),
                terrain_texture_array: self.texture_array.clone(),
            },
        });
        self.cache.insert(key, material.clone());
        material
    }

    fn material_for_layer(
        &mut self,
        definition: &LayerDefinition,
    ) -> Handle<TerrainMaterial> {
        let alpha = if definition.alpha_blend {
            TerrainAlphaKey::Blend
        } else if let Some(cutoff) = definition.alpha_cutoff {
            TerrainAlphaKey::Mask(cutoff.to_bits())
        } else {
            TerrainAlphaKey::Opaque
        };
        let key = TerrainMaterialKey {
            texture: Some(definition.texture.clone()),
            base_tint_enabled: definition.tint != BlockTint::None,
            overlay_texture: None,
            overlay_tint_enabled: false,
            alpha,
            layer_index: 0,
        };
        if let Some(existing) = self.cache.get(&key) {
            return existing.clone();
        }

        let material = self.materials.add(TerrainMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(self.asset_server.load(definition.texture.clone())),
                perceptual_roughness: self.roughness,
                metallic: self.metallic,
                alpha_mode: alpha.alpha_mode(),
                fog_enabled: true,
                unlit: true,
                ..default()
            },
            extension: TerrainMaterialExtension {
                lighting: self.lighting.handle(),
                fluid_animation_factor: 0.0,
                base_tint_enabled: key.base_tint_enabled as u8 as f32,
                overlay_enabled: 0.0,
                overlay_tint_enabled: 0.0,
                texture_array_enabled: 0.0,
                overlay_texture: None,
                terrain_texture_array: self.texture_array.clone(),
            },
        });
        self.cache.insert(key, material.clone());
        material
    }

    fn material_for(
        &mut self,
        definition: &BlockDefinition,
        layer: Option<&BlockTextureLayer>,
        layer_index: usize,
    ) -> Handle<TerrainMaterial> {
        let alpha = TerrainAlphaKey::for_layer(definition, layer_index);
        let key = TerrainMaterialKey {
            texture: layer.map(|layer| layer.texture.clone()),
            base_tint_enabled: layer.is_some_and(|layer| layer.dyable),
            overlay_texture: None,
            overlay_tint_enabled: false,
            alpha,
            layer_index,
        };
        if let Some(existing) = self.cache.get(&key) {
            return existing.clone();
        }

        let base_color_texture =
            layer.map(|layer| load_block_texture_layer(self.asset_server, layer));
        let base_tint_enabled = key.base_tint_enabled as u8 as f32;
        let material = self.materials.add(TerrainMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture,
                perceptual_roughness: self.roughness,
                metallic: self.metallic,
                alpha_mode: alpha.alpha_mode(),
                depth_bias: layer_index as f32 * TEXTURE_LAYER_DEPTH_BIAS,
                fog_enabled: true,
                unlit: true,
                ..default()
            },
            extension: TerrainMaterialExtension {
                lighting: self.lighting.handle(),
                fluid_animation_factor: 0.0,
                base_tint_enabled,
                overlay_enabled: 0.0,
                overlay_tint_enabled: 0.0,
                texture_array_enabled: 0.0,
                overlay_texture: None,
                terrain_texture_array: self.texture_array.clone(),
            },
        });
        self.cache.insert(key, material.clone());
        material
    }
}

#[derive(Resource, Clone)]
pub struct TerrainMaterials {
    blocks: HashMap<String, BlockFaces<Vec<Handle<TerrainMaterial>>>>,
    layers: HashMap<String, Handle<TerrainMaterial>>,
    array_materials: HashMap<TerrainAlphaKey, Handle<TerrainMaterial>>,
    texture_table: TerrainTextureTable,
    texture_array: Handle<Image>,
    texture_sources: Arc<Mutex<Option<Vec<Handle<Image>>>>>,
    texture_array_ready: Arc<AtomicBool>,
}

impl TerrainMaterials {
    pub fn from_registry(
        blocks: &BlockRegistry,
        layers: &LayerRegistry,
        asset_server: &AssetServer,
        images: &mut Assets<Image>,
        materials: &mut Assets<TerrainMaterial>,
        lighting: &TerrainLightingBuffer,
        roughness: f32,
        metallic: f32,
    ) -> Self {
        let texture_table = TerrainTextureTable::from_blocks(blocks);
        let texture_sources = texture_table
            .paths()
            .iter()
            .map(|path| asset_server.load(path.clone()))
            .collect::<Vec<_>>();
        let texture_array =
            create_terrain_texture_array(texture_table.layer_count(), images);

        let mut builder = TerrainMaterialBuilder::new(
            asset_server,
            materials,
            lighting,
            roughness,
            metallic,
            texture_array.clone(),
        );

        let mut array_materials = HashMap::new();
        for definition in blocks.iter() {
            for face in BlockFace::ALL {
                if block_face_texture_layers(face, definition).len() > 2 {
                    continue;
                }
                let Some((alpha_blend, alpha_cutoff)) =
                    terrain_array_alpha_signature(definition)
                else {
                    continue;
                };
                let alpha = if alpha_blend {
                    TerrainAlphaKey::Blend
                } else {
                    TerrainAlphaKey::Mask(
                        alpha_cutoff.expect("shared solid terrain uses a mask cutoff"),
                    )
                };
                if !array_materials.contains_key(&alpha) {
                    array_materials.insert(alpha, builder.array_material(alpha));
                }
            }
        }

        let blocks = blocks
            .iter()
            .map(|definition| {
                let block_materials = BlockFaces::from_fn(|face| {
                    let uses_array = block_face_texture_layers(face, definition).len() <= 2
                        && terrain_array_alpha_signature(definition).is_some();
                    if uses_array {
                        Vec::new()
                    } else {
                        builder.layers_for(definition, face)
                    }
                });
                (definition.id.clone(), block_materials)
            })
            .collect();

        let layers = layers
            .iter()
            .map(|definition| {
                (
                    definition.id.clone(),
                    builder.material_for_layer(definition),
                )
            })
            .collect();

        Self {
            blocks,
            layers,
            array_materials,
            texture_table,
            texture_array,
            texture_sources: Arc::new(Mutex::new(Some(texture_sources))),
            texture_array_ready: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn texture_table(&self) -> &TerrainTextureTable {
        &self.texture_table
    }

    pub(crate) fn texture_array_handle(&self) -> Handle<Image> {
        self.texture_array.clone()
    }

    pub(crate) fn ensure_texture_array_ready(&self, images: &mut Assets<Image>) -> bool {
        if self.texture_array_ready.load(Ordering::Acquire) {
            return true;
        }

        let mut texture_sources = self
            .texture_sources
            .lock()
            .expect("terrain texture source lock cannot be poisoned");
        let Some(source_handles) = texture_sources.as_ref() else {
            self.texture_array_ready.store(true, Ordering::Release);
            return true;
        };

        let mut layers = Vec::with_capacity(source_handles.len());
        for (path, handle) in self.texture_table.paths().iter().zip(source_handles) {
            let Some(image) = images.get(handle) else {
                return false;
            };
            assert_eq!(
                image.width(),
                TERRAIN_TEXTURE_SIZE,
                "terrain texture must be {TERRAIN_TEXTURE_SIZE}x{TERRAIN_TEXTURE_SIZE}: {path}"
            );
            assert_eq!(
                image.height(),
                TERRAIN_TEXTURE_SIZE,
                "terrain texture must be {TERRAIN_TEXTURE_SIZE}x{TERRAIN_TEXTURE_SIZE}: {path}"
            );
            assert_eq!(
                image.texture_descriptor.format,
                TextureFormat::Rgba8UnormSrgb,
                "terrain texture must load as RGBA8 sRGB: {path}"
            );
            let data = image
                .data
                .as_ref()
                .unwrap_or_else(|| panic!("terrain texture CPU data is unavailable: {path}"));
            assert_eq!(
                data.len(),
                TERRAIN_TEXTURE_BYTES,
                "terrain texture has unexpected byte size: {path}"
            );
            layers.push(data.clone());
        }

        let target = images
            .get_mut(&self.texture_array)
            .expect("terrain texture array must remain resident while a world is active");
        let target_data = target
            .data
            .as_mut()
            .expect("terrain texture array must retain CPU data until populated");
        assert_eq!(
            target_data.len(),
            TERRAIN_TEXTURE_BYTES * self.texture_table.layer_count() as usize,
            "terrain texture array byte size must match its layer count"
        );

        for (index, layer) in layers.into_iter().enumerate() {
            let start = (index + 1) * TERRAIN_TEXTURE_BYTES;
            target_data[start..start + TERRAIN_TEXTURE_BYTES].copy_from_slice(&layer);
        }

        // Once the array owns the texels, its source handles are redundant.
        // Legacy 3+ layer materials keep their own texture handles alive.
        texture_sources.take();
        self.texture_array_ready.store(true, Ordering::Release);
        true
    }

    pub(super) fn for_array(
        &self,
        alpha_blend: bool,
        alpha_cutoff: Option<u32>,
    ) -> &Handle<TerrainMaterial> {
        let alpha = if alpha_blend {
            TerrainAlphaKey::Blend
        } else if let Some(cutoff) = alpha_cutoff {
            TerrainAlphaKey::Mask(cutoff)
        } else {
            TerrainAlphaKey::Opaque
        };
        self.array_materials
            .get(&alpha)
            .unwrap_or_else(|| panic!("missing shared terrain array material for {alpha:?}"))
    }

    pub(super) fn for_face(
        &self,
        block_id: &str,
        face: BlockFace,
    ) -> &[Handle<TerrainMaterial>] {
        self.blocks
            .get(block_id)
            .unwrap_or_else(|| panic!("missing terrain materials for block: {block_id}"))
            .get(face)
            .as_slice()
    }

    pub(super) fn for_layer(&self, layer_id: &str) -> &Handle<TerrainMaterial> {
        self.layers
            .get(layer_id)
            .unwrap_or_else(|| panic!("missing terrain material for layer: {layer_id}"))
    }
}

fn create_terrain_texture_array(
    layer_count: u32,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let stacked_height = TERRAIN_TEXTURE_SIZE
        .checked_mul(layer_count)
        .expect("terrain texture array height cannot overflow u32");
    let mut image = Image::new_fill(
        Extent3d {
            width: TERRAIN_TEXTURE_SIZE,
            height: stacked_height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image
        .reinterpret_stacked_2d_as_array(layer_count)
        .expect("terrain texture stack must reinterpret as a 2D array");
    images.add(image)
}

#[derive(Resource, Clone)]
pub struct FluidMaterials {
    materials: HashMap<FluidId, Handle<TerrainMaterial>>,
}

impl FluidMaterials {
    pub fn from_registry(
        fluids: &FluidRegistry,
        materials: &mut Assets<TerrainMaterial>,
        lighting: &TerrainLightingBuffer,
        texture_array: Handle<Image>,
    ) -> Self {
        let materials = fluids
            .iter()
            .map(|(fluid_id, definition)| {
                let material = materials.add(TerrainMaterial {
                    base: StandardMaterial {
                        base_color: Color::srgba(1.0, 1.0, 1.0, definition.opacity),
                        perceptual_roughness: definition.roughness,
                        metallic: definition.metallic,
                        alpha_mode: if definition.opacity >= 1.0 {
                            AlphaMode::Opaque
                        } else {
                            AlphaMode::Blend
                        },
                        double_sided: true,
                        cull_mode: None,
                        fog_enabled: true,
                        unlit: true,
                        ..default()
                    },
                    extension: TerrainMaterialExtension {
                        lighting: lighting.handle(),
                        fluid_animation_factor: 1.0,
                        base_tint_enabled: 1.0,
                        overlay_enabled: 0.0,
                        overlay_tint_enabled: 0.0,
                        texture_array_enabled: 0.0,
                        overlay_texture: None,
                        terrain_texture_array: texture_array.clone(),
                    },
                });

                (fluid_id, material)
            })
            .collect();

        Self { materials }
    }

    pub(super) fn get(&self, fluid_id: FluidId) -> &Handle<TerrainMaterial> {
        self.materials
            .get(&fluid_id)
            .unwrap_or_else(|| panic!("missing material for fluid id {fluid_id}"))
    }
}
