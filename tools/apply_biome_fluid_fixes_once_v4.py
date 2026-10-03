from pathlib import Path

exec(
    compile(
        Path("tools/apply_biome_fluid_fixes_once_v3.py").read_text(encoding="utf-8"),
        "tools/apply_biome_fluid_fixes_once_v3.py",
        "exec",
    )
)

replace_exact(
    "src/world/generation/biome_map.rs",
    "    pub(crate) ocean_weight: f32,\n",
    "",
)
replace_exact(
    "src/world/generation/biome_map.rs",
    "                let ocean_weight = ocean_weight_from_surface(&surface, biome_field);\n",
    "",
)
replace_exact(
    "src/world/generation/biome_map.rs",
    "                    ocean_weight,\n",
    "",
)
replace_exact(
    "src/world/generation/biome_map.rs",
    '''fn ocean_weight_from_surface(surface: &BiomeFieldSample<'_>, biome_field: &BiomeField) -> f32 {\n    let Some(ocean_index) = biome_field.ocean_surface_index() else {\n        return 0.0;\n    };\n\n    surface\n        .influences\n        .iter()\n        .filter(|influence| influence.surface_index == ocean_index)\n        .map(|influence| influence.weight)\n        .sum::<f32>()\n        .clamp(0.0, 1.0)\n}\n''',
    "",
)

print("Applied final obsolete ocean-weight cleanup")
