from pathlib import Path
import json


def replace_exact(path, old, new, expected=1):
    path = Path(path)
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != expected:
        raise RuntimeError(f"{path}: expected {expected} exact matches, found {count}")
    path.write_text(text.replace(old, new, expected), encoding="utf-8")


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def write_json(path, value):
    Path(path).write_text(
        json.dumps(value, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


# Fluids own whether biome color affects their surface and immersion.
replace_exact(
    "src/content/fluid.rs",
    "    pub color: Hsi,\n    pub opacity: f32,\n",
    "    pub color: Hsi,\n    #[serde(default)]\n    pub biome_tint: bool,\n    #[serde(default)]\n    pub biome_immersion_tint: bool,\n    pub opacity: f32,\n",
)

replace_exact(
    "src/world/chunk_rendering/spawn.rs",
    """                context
                    .biome_field
                    .water_color(position, context.biomes, fluid.color)
                    .to_srgb()
""",
    """                if fluid.biome_tint {
                    context
                        .biome_field
                        .water_color(position, context.biomes, fluid.color)
                        .to_srgb()
                } else {
                    fluid.color.to_srgb()
                }
""",
)

replace_exact(
    "src/hud/fluid_immersion.rs",
    """    let resolved_tint = if let Some(tint) = definition.immersion_tint {
        Some((tint.color, tint.opacity))
    } else if definition.id == \"asteria:water\" {
        Some((
            biome_visuals.blend_hsi(|biome| biome.visuals().underwater_tint.color),
            biome_visuals
                .weighted_scalar(|biome| biome.visuals().underwater_tint.opacity)
                .clamp(0.0, 1.0),
        ))
    } else {
        None
    };
""",
    """    let resolved_tint = definition
        .immersion_tint
        .map(|tint| (tint.color, tint.opacity))
        .or_else(|| {
            definition.biome_immersion_tint.then(|| {
                (
                    biome_visuals.blend_hsi(|biome| biome.visuals().underwater_tint.color),
                    biome_visuals
                        .weighted_scalar(|biome| biome.visuals().underwater_tint.opacity)
                        .clamp(0.0, 1.0),
                )
            })
        });
""",
)

# Restore only the generic mutual-exclusion authoring contract.
replace_exact(
    "src/content/dimension/types.rs",
    "    #[serde(default)]\n    pub size: Option<DimensionBiomeSize>,\n}\n",
    "    #[serde(default)]\n    pub size: Option<DimensionBiomeSize>,\n    #[serde(default)]\n    pub exclusive_neighbor_group: Option<String>,\n}\n",
)

replace_exact(
    "src/world/biome_field.rs",
    "    pub weight: f32,\n    pub climate: BiomeClimate,\n",
    "    pub weight: f32,\n    pub exclusive_neighbor_group: Option<String>,\n    pub climate: BiomeClimate,\n",
)
replace_exact(
    "src/world/biome_field.rs",
    "                weight: dimension_biome.weight,\n                climate: biome.climate,\n",
    "                weight: dimension_biome.weight,\n                exclusive_neighbor_group: dimension_biome.exclusive_neighbor_group.clone(),\n                climate: biome.climate,\n",
)

replace_exact(
    "src/world/biome_field/surface_field.rs",
    """    fn land_biome_for_cell(&self, cell: IVec2) -> usize {
        let total_weight = self.surface_field_config.land_total_weight;
        if total_weight <= f32::EPSILON {
            return self.surface_field_config.first_land_index;
        }

        let target = hash_unit(cell_hash(cell, self.seed ^ LAND_HASH_SALT)) * total_weight;
        let mut cumulative = 0.0_f32;
        let mut fallback = self.surface_field_config.first_land_index;
        for (index, biome) in self.surface_biomes.iter().enumerate() {
            if Some(index) == self.ocean_surface_index || biome.weight <= 0.0 {
                continue;
            }
            fallback = index;
            cumulative += biome.weight;
            if target < cumulative {
                return index;
            }
        }

        fallback
    }
""",
    """    fn land_biome_for_cell(&self, cell: IVec2) -> usize {
        let raw = self
            .weighted_land_biome_for_cell(cell, &[])
            .unwrap_or(self.surface_field_config.first_land_index);
        let mut excluded_groups = Vec::<&str>::new();

        loop {
            let candidate = self
                .weighted_land_biome_for_cell(cell, &excluded_groups)
                .unwrap_or(raw);
            let Some(group) = self.surface_biomes[candidate]
                .exclusive_neighbor_group
                .as_deref()
            else {
                return candidate;
            };

            if !self.exclusive_neighbor_conflict(cell, candidate, group) {
                return candidate;
            }

            if excluded_groups.contains(&group) {
                return raw;
            }
            excluded_groups.push(group);
        }
    }

    fn weighted_land_biome_for_cell(
        &self,
        cell: IVec2,
        excluded_groups: &[&str],
    ) -> Option<usize> {
        let allowed = |index: usize, biome: &BiomeFieldEntry| {
            Some(index) != self.ocean_surface_index
                && biome.weight > 0.0
                && biome
                    .exclusive_neighbor_group
                    .as_deref()
                    .is_none_or(|group| !excluded_groups.contains(&group))
        };
        let total_weight = if excluded_groups.is_empty() {
            self.surface_field_config.land_total_weight
        } else {
            self.surface_biomes
                .iter()
                .enumerate()
                .filter(|(index, biome)| allowed(*index, biome))
                .map(|(_, biome)| biome.weight)
                .sum::<f32>()
        };
        if total_weight <= f32::EPSILON {
            return None;
        }

        let target = hash_unit(cell_hash(cell, self.seed ^ LAND_HASH_SALT)) * total_weight;
        let mut cumulative = 0.0_f32;
        let mut fallback = None;
        for (index, biome) in self.surface_biomes.iter().enumerate() {
            if !allowed(index, biome) {
                continue;
            }
            fallback = Some(index);
            cumulative += biome.weight;
            if target < cumulative {
                return Some(index);
            }
        }

        fallback
    }

    fn exclusive_neighbor_conflict(&self, cell: IVec2, candidate: usize, group: &str) -> bool {
        for z in -1..=1 {
            for x in -1..=1 {
                if x == 0 && z == 0 {
                    continue;
                }
                let Some(neighbor) =
                    self.weighted_land_biome_for_cell(cell + IVec2::new(x, z), &[])
                else {
                    continue;
                };
                if neighbor == candidate {
                    continue;
                }
                if self.surface_biomes[neighbor]
                    .exclusive_neighbor_group
                    .as_deref()
                    == Some(group)
                {
                    return true;
                }
            }
        }
        false
    }
""",
)
replace_exact(
    "src/world/biome_field/surface_field.rs",
    "            weight,\n            climate: BiomeClimate::default(),\n",
    "            weight,\n            exclusive_neighbor_group: None,\n            climate: BiomeClimate::default(),\n",
)

surface_field = Path("src/world/biome_field/surface_field.rs")
surface_text = surface_field.read_text(encoding="utf-8")
surface_test = r'''

    #[test]
    fn exclusive_neighbor_groups_do_not_touch() {
        let plains = entry("test:plains", 1.0, size(120.0, 420.0));
        let mut alps = entry("test:alps", 1.0, size(120.0, 320.0));
        alps.exclusive_neighbor_group = Some("test:extreme_peaks".to_owned());
        let mut volcano = entry("test:volcano", 1.0, size(120.0, 320.0));
        volcano.exclusive_neighbor_group = Some("test:extreme_peaks".to_owned());
        let surface_biomes = Arc::new(vec![plains, alps, volcano]);
        let config = SurfaceFieldConfig::from_biomes(&surface_biomes, None);
        let field = BiomeField {
            surface_biomes,
            volume_biomes: Arc::new(Vec::new()),
            surface_field_config: config,
            volume_site_spacing: None,
            climate: MacroClimateField::new(91),
            seed: 91,
            single_surface_biome: None,
            ocean_surface_index: None,
            spawn_oceans: true,
            spawn_target_surface_biome: None,
        };
        let mut saw_alps = false;
        let mut saw_volcano = false;

        for z in -32..=32 {
            for x in -32..=32 {
                let cell = IVec2::new(x, z);
                let biome = field.land_biome_for_cell(cell);
                saw_alps |= biome == 1;
                saw_volcano |= biome == 2;
                if biome != 1 && biome != 2 {
                    continue;
                }
                for dz in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dz == 0 {
                            continue;
                        }
                        let neighbor = field.land_biome_for_cell(cell + IVec2::new(dx, dz));
                        assert!(
                            neighbor == biome || (neighbor != 1 && neighbor != 2),
                            "exclusive biomes touched at {cell:?}"
                        );
                    }
                }
            }
        }

        assert!(saw_alps && saw_volcano);
    }
'''
insert_at = surface_text.rfind("\n}")
if insert_at < 0:
    raise RuntimeError("surface_field.rs: test module closing brace not found")
surface_field.write_text(
    surface_text[:insert_at] + surface_test + surface_text[insert_at:],
    encoding="utf-8",
)

# Static sea belongs to actual ocean-owned columns, not every blend with ocean.
replace_exact(
    "src/world/generation/fluids.rs",
    """    if column.ocean_weight > f32::EPSILON {
        return true;
    }
""",
    """    if pass.biome_field.ocean_surface_index() == Some(column.identity_surface_index) {
        return true;
    }
""",
)

# Swamps use winding low channels instead of sinking the entire biome into one pond.
replace_exact(
    "src/world/terrain.rs",
    """        } => {
            let strength = smoothstep(distribution_strength.clamp(0.0, 1.0));
            let broad = fractal_noise(position * scale, seed);
            let detail = fractal_noise(position * detail_scale, seed.rotate_left(23));
            sea_level
                + base_height
                - depth * strength
                + broad * amplitude
                + detail * detail_amplitude
        }
        BiomeTerrain::Mountains {
""",
    """        } => {
            let strength = smoothstep(distribution_strength.clamp(0.0, 1.0));
            let broad = fractal_noise(position * scale, seed);
            let detail = fractal_noise(position * detail_scale, seed.rotate_left(23));
            let channel_broad =
                fractal_noise(position * (scale * 0.72), seed.rotate_left(7));
            let channel_detail =
                fractal_noise(position * (detail_scale * 0.55), seed.rotate_left(47));
            let channel_distance = (channel_broad + channel_detail * 0.35).abs();
            let channel_strength =
                smoothstep(((0.28 - channel_distance) / 0.28).clamp(0.0, 1.0));
            sea_level
                + base_height
                - depth * strength * channel_strength
                + broad * amplitude
                + detail * detail_amplitude
        }
        BiomeTerrain::Mountains {
""",
)

terrain = Path("src/world/terrain.rs")
terrain_text = terrain.read_text(encoding="utf-8")
terrain_test = r'''

    #[test]
    fn swamp_terrain_contains_both_channels_and_dry_ground() {
        let terrain = BiomeTerrain::Swamp {
            base_height: 1.4,
            depth: 3.2,
            amplitude: 0.85,
            scale: 0.0065,
            detail_amplitude: 0.45,
            detail_scale: 0.045,
        };
        let mut wet = false;
        let mut dry = false;

        for z in 0..64 {
            for x in 0..64 {
                let height = biome_surface_height(
                    Vec2::new(x as f32 * 7.0, z as f32 * 7.0),
                    90,
                    42,
                    terrain,
                    &[],
                    1.0,
                );
                wet |= height < 90.0;
                dry |= height > 90.5;
            }
        }

        assert!(wet, "swamp terrain should carve channels below sea level");
        assert!(dry, "swamp terrain should retain dry ground between channels");
    }
'''
insert_at = terrain_text.rfind("\n}")
if insert_at < 0:
    raise RuntimeError("terrain.rs: test module closing brace not found")
terrain.write_text(
    terrain_text[:insert_at] + terrain_test + terrain_text[insert_at:],
    encoding="utf-8",
)

water = read_json("data/fluids/water.json")
water["biomeTint"] = True
water["biomeImmersionTint"] = True
write_json("data/fluids/water.json", water)

lava = read_json("data/fluids/lava.json")
lava["biomeTint"] = False
lava["biomeImmersionTint"] = False
lava["immersionTint"] = {
    "color": {"hue": 16, "saturation": 0.98, "intensity": 0.3},
    "opacity": 0.72,
}
write_json("data/fluids/lava.json", lava)

volcano = read_json("data/dimensions/overworld/biomes/volcano.json")
volcano["surfaceLayers"] = [{"block": "asteria:basalt"}]
write_json("data/dimensions/overworld/biomes/volcano.json", volcano)

gorge = read_json("data/dimensions/overworld/biomes/gorge.json")
gorge["surfaceLayers"] = [{"block": "asteria:stone"}]
write_json("data/dimensions/overworld/biomes/gorge.json", gorge)

withered = read_json("data/dimensions/umbral/biomes/withered_waste.json")
if not withered.get("surfaceLayers") or withered["surfaceLayers"][0].get("block") != "asteria:clay":
    raise RuntimeError("withered_waste: expected clay surface before migration")
withered["surfaceLayers"][0]["block"] = "asteria:dirt"
write_json("data/dimensions/umbral/biomes/withered_waste.json", withered)

swamp = read_json("data/dimensions/overworld/biomes/swamp.json")
swamp["terrain"].update(
    {
        "baseHeight": 1.4,
        "depth": 3.2,
        "amplitude": 0.85,
        "detailAmplitude": 0.45,
    }
)
swamp["surfaceLayers"] = [
    {"block": "asteria:dirt", "depth": 2},
    {"block": "asteria:mud", "depth": 2},
    {"block": "asteria:stone"},
]
for spawn in swamp.get("objectSpawns", []):
    if spawn.get("object") == "asteria:mushroom_brown":
        spawn["groundBlocks"] = ["asteria:dirt", "asteria:mud"]
write_json("data/dimensions/overworld/biomes/swamp.json", swamp)

dimension = read_json("data/dimensions/overworld/dimension.json")
grouped = {"asteria:overworld/alps", "asteria:overworld/volcano"}
found = set()
for biome in dimension["biomes"]:
    if biome["id"] in grouped:
        biome["exclusiveNeighborGroup"] = "asteria:overworld/extreme_peaks"
        found.add(biome["id"])
if found != grouped:
    raise RuntimeError(f"dimension: missing exclusive biomes: {grouped - found}")
write_json("data/dimensions/overworld/dimension.json", dimension)

grass = read_json("data/objects/grass.json")
grass["tint"] = "leaf"
write_json("data/objects/grass.json", grass)

mushrooms = sorted(Path("data/objects").glob("mushroom_*.json"))
if len(mushrooms) != 7:
    raise RuntimeError(f"expected 7 mushroom object definitions, found {len(mushrooms)}")
for path in mushrooms:
    mushroom = read_json(path)
    if mushroom.get("visual", {}).get("type") != "crossedSprite":
        raise RuntimeError(f"{path}: expected crossedSprite visual")
    mushroom["visual"]["planes"] = 2
    write_json(path, mushroom)

desert = read_json("data/ambient_particles/desert_sand.json")
desert.update(
    {
        "opacity": 0.4,
        "size": {"min": 0.018, "max": 0.05},
        "lifetime": {"min": 2.8, "max": 5.0},
        "spawnRate": 72.0,
        "velocity": [0.06, 0.015, 0.0],
        "velocityJitter": [0.28, 0.045, 0.28],
        "acceleration": [0.0, -0.018, 0.0],
        "wanderStrength": 0.13,
        "windInfluence": 3.4,
        "spawnRadius": 30.0,
        "verticalRange": 5.0,
    }
)
write_json("data/ambient_particles/desert_sand.json", desert)

print("Applied biome/fluid/render package")
