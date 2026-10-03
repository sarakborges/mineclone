from pathlib import Path

script = Path("tools/apply_biome_fluid_fixes_once.py").read_text(encoding="utf-8")
old_source = '''    "    pub color: Hsi,\\n    pub opacity: f32,\\n",
    "    pub color: Hsi,\\n    #[serde(default)]\\n    pub biome_tint: bool,\\n    #[serde(default)]\\n    pub biome_immersion_tint: bool,\\n    pub opacity: f32,\\n",
'''
new_source = '''    "pub struct FluidDefinition {\\n    pub id: String,\\n    pub name: LocalizedText,\\n    pub color: Hsi,\\n    pub opacity: f32,\\n",
    "pub struct FluidDefinition {\\n    pub id: String,\\n    pub name: LocalizedText,\\n    pub color: Hsi,\\n    #[serde(default)]\\n    pub biome_tint: bool,\\n    #[serde(default)]\\n    pub biome_immersion_tint: bool,\\n    pub opacity: f32,\\n",
'''
if script.count(old_source) != 1:
    raise RuntimeError("one-shot script fluid schema matcher changed unexpectedly")
script = script.replace(old_source, new_source, 1)
exec(compile(script, "tools/apply_biome_fluid_fixes_once.py", "exec"))
