#!/usr/bin/env python3
"""One-time migration: cubical shared GLB; species PNGs from creature JSON."""
from pathlib import Path
import json
import subprocess
import struct

ROOT = Path.cwd()


def replace_once(text: str, old: str, new: str, path: Path) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f'{path}: expected one occurrence, got {count}: {old[:90]!r}')
    return text.replace(old, new, 1)


def patch_file(path: str, alterations: list[tuple[str, str]]) -> None:
    file = ROOT / path
    text = file.read_text(encoding='utf-8')
    for old, new in alterations:
        text = replace_once(text, old, new, file)
    file.write_text(text, encoding='utf-8')


script = 'assets/models/creatures/slime/generate_slime.py'
patch_file(script, [
    ("Generate Asteria's sharp-edged slime GLB and its pixel-perfect 64x64 skin.",
     "Generate Asteria's sharp-edged shared slime GLB and standalone 64x64 skins."),
    ("Uses only Python's standard library. Never rounds corners or smooths face details.",
     "Uses only Python's standard library. Never rounds corners or smooths face details.\nCreature JSON determines which PNG is loaded; GLB embeds no image data."),
    ("OUT = Path(__file__).resolve().parent\n",
     "OUT = Path(__file__).resolve().parent\nTEXTURES = OUT.parents[2] / 'textures' / 'creatures'\n"),
])
generator = ROOT / script
text = generator.read_text(encoding='utf-8')
start = text.index('# An RGBA 64x64 atlas.')
end = text.index('# Outward winding', start)
assert text.count('# An RGBA 64x64 atlas.') == text.count('# Outward winding') == 1
text = text[:start] + '''# A grayscale atlas enables independent HSI tinting of shell/core/cheeks. Each
# species keeps its own PNG, selected exclusively through its creature JSON.
# Atlas tile positions must match the UV tile coordinates below.
def png_chunk(kind: bytes, payload: bytes) -> bytes:
    content = kind + payload
    return (struct.pack('>I', len(payload)) + content
            + struct.pack('>I', zlib.crc32(content) & 0xffffffff))


def make_skin(species: str) -> bytes:
    pixels = bytearray([255, 255, 255, 255] * 64 * 64)
    for y in range(64):
        for x in range(64):
            tile = (x // 16, y // 16)
            u, v = x % 16, y % 16
            if tile == (0, 0):
                border = min(u, v, 15-u, 15-v) < 2
                # Different square patterns, never rounded or smoothed.
                if species == 'meadow_slime':
                    gray = 229 if border else (247 if (u//4 + v//4) % 2 else 255)
                else:
                    gray = 222 if border else (239 if (u//2 + v//4) % 2 else 255)
            elif tile == (1, 0):
                gray = 220 if min(u, v, 15-u, 15-v) < 2 else (246 if (u//4 + v//4) % 2 else 255)
            elif tile == (0, 1):
                gray = 245 if u < 4 or v < 4 else 255
            else:
                gray = 255
            index = (y * 64 + x) * 4
            pixels[index:index + 4] = bytes((gray, gray, gray, 255))
    scanlines = b''.join(b'\\0' + pixels[y*64*4:(y+1)*64*4] for y in range(64))
    return (b'\\x89PNG\\r\\n\\x1a\\n' + png_chunk(b'IHDR', struct.pack('>IIBBBBB', 64, 64, 8, 6, 0, 0, 0))
            + png_chunk(b'IDAT', zlib.compress(scanlines, 9)) + png_chunk(b'IEND', b''))


TEXTURES.mkdir(parents=True, exist_ok=True)
for species in ('meadow_slime', 'ember_slime'):
    destination = TEXTURES / f'{species}.png'
    # Only create missing default skins; never overwrite subsequent artist edits.
    if not destination.exists():
        destination.write_bytes(make_skin(species))

''' + text[end:]
text = replace_once(text, "'baseColorFactor': [*color, alpha], 'baseColorTexture': {'index': 0},",
                    "'baseColorFactor': [*color, alpha],", generator)
text = replace_once(text, "    'images':[{'name':'slime_skin_64','mimeType':'image/png','bufferView':image_view}],\n    'samplers':[{'magFilter':9728,'minFilter':9728,'wrapS':33071,'wrapT':33071}],\n    'textures':[{'source':0,'sampler':0}],\n", "", generator)
text = replace_once(text, "'collision_source':'slime.collider.json','skin_resolution':[64,64],",
                    "'collision_source':'slime.collider.json','skin_resolution':[64,64],\n              'texture_source':'creature JSON material textures under textures/creatures/',", generator)
text = replace_once(text, "f'{len(animations)} animation clips, embedded 64x64 pixel skin')",
                    "f'{len(animations)} animation clips, JSON-selected standalone 64x64 PNG skins')", generator)
assert not any(bad in text for bad in ('image_view', "'images':", "'baseColorTexture':"))
generator.write_text(text, encoding='utf-8')

path = 'src/content/creature.rs'
patch_file(path, [
    ('    pub model: String,\n    pub collider: CreatureCollider,',
     '    pub model: String,\n    #[serde(default)]\n    pub textures: std::collections::HashMap<String, String>,\n    pub collider: CreatureCollider,'),
    ('        definition.collider.validate(&definition.id);',
     '''        for (material, texture) in &definition.textures {
            assert!(
                !material.trim().is_empty() && valid_creature_texture_path(texture),
                "creature {} material {material:?} has an invalid texture path: {texture}",
                definition.id
            );
        }
        definition.collider.validate(&definition.id);'''),
    ('#[cfg(test)]\nmod tests {',
     '''/// Creature materials may only refer to PNGs beneath the moddable creature texture root.
fn valid_creature_texture_path(path: &str) -> bool {
    if path.contains('\\\\') || path.contains(':') || path.contains('\\0') {
        return false;
    }
    let candidate = Path::new(path);
    if !candidate.components().all(|component| matches!(component, Component::Normal(_))) {
        return false;
    }
    let mut parts = path.split('/');
    if parts.next() != Some("textures") || parts.next() != Some("creatures") {
        return false;
    }
    if !parts.all(|part| !part.is_empty() && part != "." && part != "..") {
        return false;
    }
    matches!(candidate.extension().and_then(|ext| ext.to_str()), Some("png"))
}

#[cfg(test)]
mod tests {'''),
    ('    #[test]\n    fn collider_bounds_stay_independent_of_visual_animation() {',
     '''    #[test]
    fn texture_paths_remain_inside_moddable_creatures_directory() {
        assert!(valid_creature_texture_path("textures/creatures/meadow_slime.png"));
        assert!(!valid_creature_texture_path("textures/creatures/../secret.png"));
        assert!(!valid_creature_texture_path("/textures/creatures/slime.png"));
        assert!(!valid_creature_texture_path("textures/blocks/stone.png"));
        assert!(!valid_creature_texture_path("textures/creatures/slime.jpg"));
        assert!(!valid_creature_texture_path("textures/creatures\\\\secret.png"));
    }

    #[test]
    fn collider_bounds_stay_independent_of_visual_animation() {'''),
])

path = 'src/creatures/visual.rs'
patch_file(path, [
    ('    material_tints: HashMap<String, Hsi>,\n    graph:',
     '    material_tints: HashMap<String, Hsi>,\n    material_textures: HashMap<String, Handle<Image>>,\n    graph:'),
    ('    HashMap<(AssetId<StandardMaterial>, [u32; 3]), Handle<StandardMaterial>>,',
     '    HashMap<(AssetId<StandardMaterial>, Option<[u32; 3]>, Option<AssetId<Image>>), Handle<StandardMaterial>>,'),
    ('    definitions: Res<CreatureRegistry>,\n    gltfs:',
     '    definitions: Res<CreatureRegistry>,\n    asset_server: Res<AssetServer>,\n    gltfs:'),
    ('        let tints = definition.material_tints.clone();',
     '''        let tints = definition.material_tints.clone();
        let textures = definition
            .textures
            .iter()
            .map(|(material, path)| (material.clone(), asset_server.load::<Image>(path.clone())))
            .collect();'''),
    ('                        material_tints: tints,\n                        graph,',
     '                        material_tints: tints,\n                        material_textures: textures,\n                        graph,'),
])
visual = ROOT / path
text = visual.read_text(encoding='utf-8')
a = text.index('        if let Ok((original, material_name)) = mesh_materials.get(descendant)')
b = text.index('        if let Ok((player_entity, mut player)) = players.get_mut(descendant)', a)
assert text.count('        if let Ok((original, material_name)) = mesh_materials.get(descendant)') == 1
assert text.count('        if let Ok((player_entity, mut player)) = players.get_mut(descendant)') == 1
text = text[:a] + '''        if let Ok((original, material_name)) = mesh_materials.get(descendant) {
            let name = material_name.0.as_str();
            let tint = appearance.material_tints.get(name);
            let texture = appearance.material_textures.get(name);
            if tint.is_some() || texture.is_some() {
                let rgb = tint.map(|color| color.to_srgb());
                let cache_key = (
                    original.id(),
                    rgb.map(|color| color.map(f32::to_bits)),
                    texture.map(|image| image.id()),
                );
                let replacement = if let Some(existing) = tint_assets.cache.0.get(&cache_key) {
                    Some(existing.clone())
                } else {
                    tint_assets
                        .materials
                        .get(original.id())
                        .cloned()
                        .map(|mut material| {
                            if let Some(color) = rgb {
                                let alpha = material.base_color.to_srgba().alpha;
                                material.base_color = Color::srgba(color[0], color[1], color[2], alpha);
                            }
                            if let Some(image) = texture {
                                material.base_color_texture = Some(image.clone());
                            }
                            let handle = tint_assets.materials.add(material);
                            tint_assets.cache.0.insert(cache_key, handle.clone());
                            handle
                        })
                };
                if let Some(material) = replacement {
                    commands.entity(descendant).insert(MeshMaterial3d(material));
                }
            }
        }

''' + text[b:]
visual.write_text(text, encoding='utf-8')

materials = ('SlimeShell', 'SlimeCore', 'SlimeEyes', 'SlimeHighlights', 'SlimeCheeks')
for species in ('meadow', 'ember'):
    file = ROOT / f'data/creatures/slimes/{species}.json'
    original = json.loads(file.read_text(encoding='utf-8'))
    assert 'textures' not in original
    texture = f'textures/creatures/{species}_slime.png'
    original['textures'] = {material: texture for material in materials}
    file.write_text(json.dumps(original, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')

version = ROOT / 'VERSION'
assert version.read_text(encoding='utf-8') == '0.18.0\n', 'Unexpected version: migration aborted'
version.write_text('0.19.0\n', encoding='utf-8')
subprocess.run(['python3', script], cwd=ROOT, check=True)

asset = (ROOT / 'assets/models/creatures/slime/slime.glb').read_bytes()
magic, version, length = struct.unpack_from('<4sII', asset)
assert magic == b'glTF' and version == 2 and length == len(asset)
json_length, chunk_type = struct.unpack_from('<I4s', asset, 12)
assert chunk_type == b'JSON'
scene = json.loads(asset[20:20+json_length])
assert len(scene['meshes']) == 6 and len(scene['animations']) == 6
assert not any(key in scene for key in ('images','textures','samplers'))
assert all('TEXCOORD_0' in mesh['primitives'][0]['attributes'] for mesh in scene['meshes'])
for species in ('meadow', 'ember'):
    texture = ROOT / f'assets/textures/creatures/{species}_slime.png'
    png = texture.read_bytes()
    assert png.startswith(b'\x89PNG\r\n\x1a\n')
    assert struct.unpack_from('>II', png, 16) == (64, 64)
    definition = json.loads((ROOT / f'data/creatures/slimes/{species}.json').read_text())
    assert all(ROOT / ('assets/' + path) == texture for path in definition['textures'].values())
print('Verified: cubical GLB without images; 6 clips; per-species 64x64 PNGs and JSON slots.')
