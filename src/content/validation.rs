use super::loader::LoadedContent;

pub(super) fn validate_content(content: &LoadedContent) {
    content.player.validate_references(&content.attacks);

    for category in content.inventory_categories.iter() {
        category.validate();
    }

    for block in content.blocks.iter() {
        assert!(
            content.layers.get(&block.id).is_none(),
            "content id {} cannot be both a block and a layer",
            block.id
        );
        assert!(
            content.items.get(&block.id).is_none(),
            "content id {} cannot be both a block and an item",
            block.id
        );
        block.validate_references(
            &content.inventory_categories,
            &content.secondary_properties,
            &content.tool_categories,
            &content.tools,
        );
        block.loot_table.validate_references(
            &format!("block {}", block.id),
            |item_id| {
                content.blocks.get(item_id).is_some()
                    || content.items.get(item_id).is_some()
                    || content.layers.get(item_id).is_some()
                    || content.objects.get(item_id).is_some()
                    || content.tools.get(item_id).is_some()
            },
        );
    }

    for object in content.objects.iter() {
        assert!(
            content.blocks.get(&object.id).is_none(),
            "content id {} cannot be both an object and a block",
            object.id
        );
        assert!(
            content.layers.get(&object.id).is_none(),
            "content id {} cannot be both an object and a layer",
            object.id
        );
        assert!(
            content.items.get(&object.id).is_none(),
            "content id {} cannot be both an object and an item",
            object.id
        );
        assert!(
            content.tools.get(&object.id).is_none(),
            "content id {} cannot be both an object and a tool",
            object.id
        );
        object.validate_references(&content.inventory_categories);
    }

    for layer in content.layers.iter() {
        assert!(
            content.blocks.get(&layer.id).is_none(),
            "content id {} cannot be both a layer and a block",
            layer.id
        );
        assert!(
            content.tools.get(&layer.id).is_none(),
            "content id {} cannot be both a layer and a tool",
            layer.id
        );
        assert!(
            content.items.get(&layer.id).is_none(),
            "content id {} cannot be both a layer and an item",
            layer.id
        );
        layer.validate_references(&content.inventory_categories);
    }

    for tool in content.tools.iter() {
        assert!(
            content.blocks.get(&tool.id).is_none(),
            "content id {} cannot be both a block and a tool",
            tool.id
        );
        assert!(
            content.layers.get(&tool.id).is_none(),
            "content id {} cannot be both a tool and a layer",
            tool.id
        );
        assert!(
            content.items.get(&tool.id).is_none(),
            "content id {} cannot be both a tool and an item",
            tool.id
        );
        tool.validate_references(&content.inventory_categories, &content.tool_categories);
    }

    for item in content.items.iter() {
        assert!(
            content.blocks.get(&item.id).is_none(),
            "content id {} cannot be both an item and a block",
            item.id
        );
        assert!(
            content.layers.get(&item.id).is_none(),
            "content id {} cannot be both an item and a layer",
            item.id
        );
        assert!(
            content.tools.get(&item.id).is_none(),
            "content id {} cannot be both an item and a tool",
            item.id
        );
        item.validate_references(&content.inventory_categories);
    }

    for structure in content.structures.iter() {
        structure.validate_references(&content.blocks, &content.layers, &content.fluids);
    }

    for structure_set in content.structure_sets.iter() {
        assert!(
            !content.structures.resolves_reference(&structure_set.id),
            "content id {} cannot be both a structure/structure group and a structure set",
            structure_set.id
        );
        structure_set.validate_references(&content.structures);
    }

    for biome in content.biomes.iter() {
        biome.validate_material_references(&content.blocks);
        biome.validate_spawn_references(&content.creatures);
        biome.validate_structure_references(&content.structures, &content.structure_sets);
        biome.validate_surface_fluid_references(&content.fluids);
    }

    for dimension in content.dimensions.iter() {
        dimension.validate_biomes(&content.biomes);
        assert!(
            content.day_night_cycles.get(&dimension.day_night_cycle).is_some(),
            "dimension {} references missing day-night cycle {}",
            dimension.id,
            dimension.day_night_cycle
        );
        assert!(
            content.skies.get(&dimension.sky).is_some(),
            "dimension {} references missing sky {}",
            dimension.id,
            dimension.sky
        );
        dimension
            .hydrology
            .validate_references(&dimension.id, &content.biomes, &content.fluids);
    }
}
