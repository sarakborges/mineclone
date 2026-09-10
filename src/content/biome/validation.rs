use super::{BiomeClimate, BiomeClimateRange, BiomeDefinition, BiomeKind, BiomeSizeAxis};

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
    definition.hydrology.validate(&definition.id);
    validate_visuals(definition);

    if let Some(terrain) = &definition.terrain {
        terrain.validate(&definition.id);
    }
    if let Some(modifier) = &definition.density_modifier {
        modifier.validate(&definition.id);
    }
}

fn validate_surface_biome(definition: &BiomeDefinition) {
    validate_size_axis(&definition.id, "x", definition.size.x);
    validate_size_axis(&definition.id, "z", definition.size.z);
    if let Some(vertical_size) = definition.size.y {
        validate_size_axis(&definition.id, "y", vertical_size);
    }

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

    definition.validate_surface_materials();
}

fn validate_volume_biome(definition: &BiomeDefinition) {
    validate_size_axis(&definition.id, "x", definition.size.x);
    validate_size_axis(&definition.id, "z", definition.size.z);
    let vertical_size = definition
        .size
        .y
        .unwrap_or_else(|| panic!("volume biome {} must define size.y", definition.id));
    validate_size_axis(&definition.id, "y", vertical_size);

    assert!(
        definition.surface_layers.is_empty(),
        "volume biome {} cannot define surfaceLayers",
        definition.id
    );
}

fn validate_hydrology_biome(definition: &BiomeDefinition) {
    assert!(
        definition.terrain.is_none(),
        "hydrology biome {} cannot define terrain",
        definition.id
    );
    assert!(
        definition.surface_layers.is_empty(),
        "hydrology biome {} cannot define surfaceLayers",
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

fn validate_visuals(definition: &BiomeDefinition) {
    assert!(
        (0.0..=1.0).contains(&definition.visuals.stars.density),
        "biome {} stars density must be between 0 and 1",
        definition.id
    );
    assert!(
        (0.0..=1.0).contains(&definition.visuals.clouds.density),
        "biome {} clouds density must be between 0 and 1",
        definition.id
    );
    assert!(
        (0.0..=1.0).contains(&definition.visuals.underwater_tint.opacity),
        "biome {} underwaterTint opacity must be between 0 and 1",
        definition.id
    );
}

fn validate_size_axis(biome_id: &str, axis: &str, size: BiomeSizeAxis) {
    assert!(
        size.min > 0.0,
        "biome {biome_id} size.{axis}.min must be positive"
    );
    assert!(
        size.max >= size.min,
        "biome {biome_id} size.{axis}.max must be greater than or equal to min"
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
