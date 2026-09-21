use super::{BiomeClimate, BiomeClimateRange, BiomeDefinition, BiomeKind};

pub(super) fn validate_biome_definition(definition: &BiomeDefinition) {
    match definition.kind {
        BiomeKind::Surface => validate_surface_biome(definition),
        BiomeKind::Volume => validate_volume_biome(definition),
        BiomeKind::Hydrology => validate_hydrology_biome(definition),
    }

    if let Some(range) = definition.vertical_range {
        assert!(
            range.min >= 0.0,
            "biome {} verticalRange.min cannot be negative",
            definition.id
        );
        assert!(
            range.max >= range.min,
            "biome {} verticalRange.max must be greater than or equal to min",
            definition.id
        );
    }

    validate_climate(&definition.id, definition.climate);
    validate_distributions(definition);
    validate_creature_spawns(definition);
    definition.hydrology.validate(&definition.id);
    validate_visuals(definition);

    if let Some(terrain) = &definition.terrain {
        terrain.validate(&definition.id);
    }
    for modifier in &definition.terrain_modifiers {
        modifier.validate(&definition.id);
    }
    for carver in &definition.surface_carvers {
        carver.validate(&definition.id);
    }
    if let Some(surface_fluid) = &definition.surface_fluid {
        surface_fluid.validate(&definition.id);
    }
    if let Some(surface_margin) = &definition.surface_margin {
        surface_margin.validate(&definition.id);
    }
    if let Some(modifier) = &definition.density_modifier {
        modifier.validate(&definition.id);
    }
}

fn validate_surface_biome(definition: &BiomeDefinition) {
    assert!(
        definition.visuals.is_some(),
        "surface biome {} must define visuals",
        definition.id
    );
    assert!(
        definition.terrain.is_some(),
        "surface biome {} must define terrain",
        definition.id
    );
    assert!(
        definition.density_modifier.is_none(),
        "surface biome {} cannot define a volume densityModifier",
        definition.id
    );
    assert!(
        definition.solid_block.is_none(),
        "surface biome {} must use surfaceLayers instead of solidBlock",
        definition.id
    );
    assert_eq!(
        definition.priority, 0,
        "surface biome {} cannot define volume overlap priority",
        definition.id
    );
    assert!(
        definition.surface_carvers.is_empty(),
        "surface biome {} cannot define surfaceCarvers; use allowSurfaceCarvers to permit cavern entrances",
        definition.id
    );

    if let Some(surface_fluid) = &definition.surface_fluid {
        let Some(crate::content::biome_terrain::BiomeTerrain::Volcano {
            crater_depth,
            ..
        }) = definition.terrain
        else {
            panic!(
                "surface biome {} surfaceFluid currently requires volcano terrain",
                definition.id
            );
        };
        match surface_fluid {
            crate::content::biome_surface_fluid::BiomeSurfaceFluid::VolcanoCrater {
                level_offset,
                ..
            } => {
                assert!(
                    *level_offset < crater_depth,
                    "surface biome {} volcano crater levelOffset must stay below craterDepth",
                    definition.id
                );
            }
        }
    }

    definition.validate_surface_materials();
}

fn validate_volume_biome(definition: &BiomeDefinition) {
    assert!(
        definition.visuals.is_some(),
        "volume biome {} must define visuals",
        definition.id
    );
    assert!(
        distributions_are_regional(definition),
        "volume biome {} cannot define a surface distribution",
        definition.id
    );
    assert!(
        definition.terrain_modifiers.is_empty(),
        "volume biome {} cannot define terrainModifiers",
        definition.id
    );
    assert!(
        !definition.allow_surface_carvers,
        "volume biome {} cannot enable allowSurfaceCarvers",
        definition.id
    );
    assert!(
        definition.surface_carvers.is_empty()
            || matches!(
                definition.density_modifier,
                Some(crate::content::biome_density::BiomeDensityModifier::Cavern { .. })
            ),
        "volume biome {} can define surfaceCarvers only with a cavern densityModifier",
        definition.id
    );
    assert!(
        definition.surface_layers.is_empty(),
        "volume biome {} cannot define surfaceLayers",
        definition.id
    );
    assert!(
        definition.surface_margin.is_none(),
        "volume biome {} cannot define surfaceMargin",
        definition.id
    );
    assert!(
        definition.surface_fluid.is_none(),
        "volume biome {} cannot define surfaceFluid",
        definition.id
    );
}

fn validate_hydrology_biome(definition: &BiomeDefinition) {
    assert!(
        definition.visuals.is_some(),
        "hydrology biome {} must define visuals",
        definition.id
    );
    assert!(
        distributions_are_regional(definition),
        "hydrology biome {} cannot define a surface distribution",
        definition.id
    );
    assert!(
        definition.terrain.is_none(),
        "hydrology biome {} cannot define terrain",
        definition.id
    );
    assert!(
        definition.terrain_modifiers.is_empty(),
        "hydrology biome {} cannot define terrainModifiers",
        definition.id
    );
    assert!(
        definition.surface_carvers.is_empty(),
        "hydrology biome {} cannot define surfaceCarvers",
        definition.id
    );
    assert!(
        !definition.allow_surface_carvers,
        "hydrology biome {} cannot enable allowSurfaceCarvers",
        definition.id
    );
    assert!(
        definition.surface_layers.is_empty(),
        "hydrology biome {} cannot define surfaceLayers",
        definition.id
    );
    assert!(
        definition.surface_margin.is_none(),
        "hydrology biome {} cannot define surfaceMargin",
        definition.id
    );
    assert!(
        definition.surface_fluid.is_none(),
        "hydrology biome {} cannot define surfaceFluid",
        definition.id
    );
    assert!(
        definition.density_modifier.is_none(),
        "hydrology biome {} cannot define densityModifier",
        definition.id
    );
    assert!(
        definition.solid_block.is_none(),
        "hydrology biome {} cannot define solidBlock",
        definition.id
    );
    assert!(
        definition.vertical_range.is_none(),
        "hydrology biome {} cannot define verticalRange",
        definition.id
    );
    assert_eq!(
        definition.priority, 0,
        "hydrology biome {} cannot define volume overlap priority",
        definition.id
    );
}

fn validate_distributions(definition: &BiomeDefinition) {
    assert!(
        !definition.distributions.is_empty(),
        "biome {} must define at least one distribution",
        definition.id
    );

    for distribution in &definition.distributions {
        distribution.validate(&definition.id);
    }

    let regional_count = definition
        .distributions
        .iter()
        .filter(|distribution| distribution.is_regional())
        .count();

    assert!(
        regional_count <= 1,
        "biome {} cannot define regional distribution more than once",
        definition.id
    );
    assert!(
        regional_count == 0 || definition.distributions.len() == 1,
        "biome {} cannot combine regional distribution with macro distributions",
        definition.id
    );
}

fn distributions_are_regional(definition: &BiomeDefinition) -> bool {
    definition.distributions.len() == 1 && definition.distributions[0].is_regional()
}

fn validate_visuals(definition: &BiomeDefinition) {
    let Some(visuals) = definition.visuals.as_ref() else {
        return;
    };

    assert!(
        (0.0..=1.0).contains(&visuals.stars.density),
        "biome {} stars density must be between 0 and 1",
        definition.id
    );
    assert!(
        (0.0..=1.0).contains(&visuals.clouds.density),
        "biome {} clouds density must be between 0 and 1",
        definition.id
    );
    assert!(
        (0.0..=1.0).contains(&visuals.underwater_tint.opacity),
        "biome {} underwaterTint opacity must be between 0 and 1",
        definition.id
    );

    for (field, color) in [
        ("grassColor", visuals.grass_color),
        ("leafColor", visuals.leaf_color),
        ("foliageColor", visuals.foliage_color),
        ("underwaterTint.color", visuals.underwater_tint.color),
    ] {
        assert!(
            color.is_valid(),
            "biome {} visuals.{field} HSI color is invalid",
            definition.id
        );
    }
    if let Some(color) = visuals.water_color {
        assert!(
            color.is_valid(),
            "biome {} visuals.waterColor HSI color is invalid",
            definition.id
        );
    }
    for (phase, colors) in [
        ("dawn", (visuals.sky_color.dawn, visuals.fog_color.dawn)),
        ("day", (visuals.sky_color.day, visuals.fog_color.day)),
        ("dusk", (visuals.sky_color.dusk, visuals.fog_color.dusk)),
        ("night", (visuals.sky_color.night, visuals.fog_color.night)),
    ] {
        assert!(
            colors.0.is_valid() && colors.1.is_valid(),
            "biome {} {phase} sky/fog HSI color is invalid",
            definition.id
        );
    }
    assert!(
        visuals.terrain_roughness.is_finite()
            && (0.0..=1.0).contains(&visuals.terrain_roughness),
        "biome {} terrainRoughness must be between 0 and 1",
        definition.id
    );
    assert!(
        visuals.terrain_metallic.is_finite()
            && (0.0..=1.0).contains(&visuals.terrain_metallic),
        "biome {} terrainMetallic must be between 0 and 1",
        definition.id
    );
}

fn validate_climate(biome_id: &str, climate: BiomeClimate) {
    validate_climate_range(biome_id, "temperature", climate.temperature);
    validate_climate_range(biome_id, "humidity", climate.humidity);
    validate_climate_range(biome_id, "continentalness", climate.continentalness);
    validate_climate_range(biome_id, "erosion", climate.erosion);
}

fn validate_climate_range(biome_id: &str, field: &str, range: Option<BiomeClimateRange>) {
    let Some(range) = range else {
        return;
    };

    assert!(
        (0.0..=1.0).contains(&range.min),
        "biome {biome_id} climate.{field}.min must be between 0 and 1"
    );
    assert!(
        (0.0..=1.0).contains(&range.max),
        "biome {biome_id} climate.{field}.max must be between 0 and 1"
    );
    assert!(
        range.max >= range.min,
        "biome {biome_id} climate.{field}.max must be greater than or equal to min"
    );
}

fn validate_creature_spawns(definition: &BiomeDefinition) {
    for spawn in &definition.creature_spawns {
        assert!(!spawn.creature.trim().is_empty(), "biome {} has an empty creature spawn id", definition.id);
        assert!(spawn.weight.is_finite() && spawn.weight >= 0.0, "biome {} creature {} weight must be finite and non-negative", definition.id, spawn.creature);
        assert!(spawn.light_min <= 15 && spawn.light_max <= 15 && spawn.light_max >= spawn.light_min, "biome {} creature {} light range is invalid", definition.id, spawn.creature);
        assert!(spawn.spacing.is_finite() && spawn.spacing > 0.0, "biome {} creature {} spacing must be positive and finite", definition.id, spawn.creature);
    }
}
