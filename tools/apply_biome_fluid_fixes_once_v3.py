from pathlib import Path

exec(
    compile(
        Path("tools/apply_biome_fluid_fixes_once_v2.py").read_text(encoding="utf-8"),
        "tools/apply_biome_fluid_fixes_once_v2.py",
        "exec",
    )
)

lighting_tests = Path("src/voxel/lighting/tests.rs")
lighting_text = lighting_tests.read_text(encoding="utf-8")
if lighting_text.count("FluidDefinition {") != 1:
    raise RuntimeError("lighting tests: expected exactly one FluidDefinition fixture")
initializer = lighting_text.index("FluidDefinition {")
opacity = lighting_text.index("        opacity:", initializer)
lighting_text = (
    lighting_text[:opacity]
    + "        biome_tint: false,\n"
    + "        biome_immersion_tint: false,\n"
    + lighting_text[opacity:]
)
lighting_tests.write_text(lighting_text, encoding="utf-8")

replace_exact(
    "src/world/generation/columns.rs",
    "    pub(crate) ocean_weight: f32,\n",
    "",
)
replace_exact(
    "src/world/generation/columns.rs",
    "                ocean_weight: surface.ocean_weight,\n",
    "",
    expected=2,
)

print("Applied Rust fixture and obsolete ocean-weight cleanup")
