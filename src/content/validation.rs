use super::loader::LoadedContent;

pub(super) fn validate_content(content: &LoadedContent) {
    for attack in content.attacks.iter() {
        attack.validate();
    }
    assert!(content.attacks.get(&content.player.attack).is_some(), "player references missing attack {}", content.player.attack);

    for category in content.inventory_categories.iter() {
        category.validate_references(&content.blocks);
    }

    for block in content.blocks.iter() {
        assert!(
            content.inventory_categories.get(&block.category).is_some(),
            "block {} references missing inventory category {}",
            block.id,
            block.category
        );
        for property in &block.secondary_properties {
            assert!(
                content.secondary_properties.contains_property(property),
                "block {} references missing secondary property {}",
                block.id,
                property
            );
        }
    }

    for tool in content.tools.iter() {
        assert!(
            content.blocks.get(&tool.id).is_none(),
            "content id {} cannot be both a block and a tool",
            tool.id
        );
        assert!(
            content.inventory_categories.get(&tool.category).is_some(),
            "tool {} references missing inventory category {}",
            tool.id,
            tool.category
        );
    }

    for structure in content.structures.iter() {
        structure.validate_references(&content.blocks, &content.fluids);
    }

    for biome in content.biomes.iter() {
        biome.validate_material_references(&content.blocks);
        for spawn in &biome.creature_spawns {
            assert!(
                content.creatures.get(&spawn.creature).is_some(),
                "biome {} references missing creature spawn: {}",
                biome.id,
                spawn.creature
            );
        }
        biome.validate_structure_references(&content.structures);
        if let Some(surface_fluid) = &biome.surface_fluid {
            assert!(
                content.fluids.id_of(surface_fluid.fluid_id()).is_some(),
                "biome {} surfaceFluid references missing fluid {}",
                biome.id,
                surface_fluid.fluid_id()
            );
        }
    }

    for dimension in content.dimensions.iter() {
        dimension.validate_biomes(&content.biomes);
        dimension
            .hydrology
            .validate_references(&dimension.id, &content.biomes, &content.fluids);
    }
}
