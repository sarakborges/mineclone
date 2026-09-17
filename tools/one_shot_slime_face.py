#!/usr/bin/env python3
"""One-time v0.20.3 migration: enlarge cubic slime core and move face into species PNGs."""
from __future__ import annotations
import hashlib
import json
import runpy
import struct
import zlib
from pathlib import Path

ROOT = Path.cwd()
GENERATOR = ROOT / 'assets/models/creatures/slime/generate_slime.py'
SKINS = ROOT / 'assets/textures/creatures'
ORIGINAL = {'meadow_slime': '07dc2960c6079008fe4917fbc4b940f7c43efa7b',
            'ember_slime': '2a18f092f353fcd08e17e9fe68bb6058fed0a091'}
assert (ROOT/'VERSION').read_text().strip() == '0.20.2', 'Unexpected version'
for species, expected in ORIGINAL.items():
    content = (SKINS/f'{species}.png').read_bytes()
    digest = hashlib.sha1(b'blob '+str(len(content)).encode()+b'\0'+content).hexdigest()
    assert digest == expected, f'Non-default skin {species}: refusing to overwrite artist edits'

source = GENERATOR.read_text(encoding='utf-8')
def replace(old: str, new: str) -> None:
    global source
    assert source.count(old) == 1, f'Unexpected generator anchor: {old[:75]}'
    source = source.replace(old, new, 1)
replace('A grayscale atlas enables independent HSI tinting of shell/core/cheeks.',
        'Grayscale external atlas: shell, larger core and a pixel face on the front tile.')
replace('Atlas tile positions must match the UV tile coordinates below.',
        'Atlas tile positions must match the UV tile coordinates below. No face mesh.')
replace('            elif tile == (1, 0):', '''            elif tile == (2, 0):
                # Only pixels form the facial features: square eyes, glints, cheeks and mouth.
                gray = 247 if (u // 4 + v // 4) % 2 else 255
                if 4 <= v <= 7 and (3 <= u <= 5 or 10 <= u <= 12):
                    gray = 15
                if v == 4 and u in (3, 10):
                    gray = 255
                if v in (9, 10) and (1 <= u <= 2 or 13 <= u <= 14):
                    gray = 170 if species == 'meadow_slime' else 152
                if (v == 10 and u in (4, 11)) or (v == 11 and u in (5, 10)) or (v == 12 and 6 <= u <= 9):
                    gray = 24
            elif tile == (1, 0):''')
replace('def cube(positions, normals, uvs, indices, extent, center=(0,0,0), tile=(0,0)):\n    half = [v/2 for v in extent]\n    u0, u1 = (tile[0]*16 + .5)/64, (tile[0]*16 + 15.5)/64\n    v0, v1 = (tile[1]*16 + .5)/64, (tile[1]*16 + 15.5)/64\n    for normal, horizontal, vertical in faces:',
'''def cube(positions, normals, uvs, indices, extent, center=(0,0,0), tile=(0,0), front_tile=None):
    half = [v/2 for v in extent]
    for normal, horizontal, vertical in faces:
        face_tile = front_tile if front_tile is not None and normal == (0,0,-1) else tile
        u0, u1 = (face_tile[0]*16 + .5)/64, (face_tile[0]*16 + 15.5)/64
        v0, v1 = (face_tile[1]*16 + .5)/64, (face_tile[1]*16 + 15.5)/64''')
replace('def make_mesh(name, cuboids, material, tile):\n    positions, normals, uvs, indices = [], [], [], []\n    for extent, center in cuboids:\n        cube(positions, normals, uvs, indices, extent, center, tile)',
'''def make_mesh(name, cuboids, material, tile, front_tile=None):
    positions, normals, uvs, indices = [], [], [], []
    for extent, center in cuboids:
        cube(positions, normals, uvs, indices, extent, center, tile, front_tile)''')
start = source.index('materials = [\n')
end = source.index('\n\ndef node(', start)
source = source[:start] + '''materials = [
    material('SlimeShell', [.50,.91,.78], .76, .23),
    material('SlimeCore', [.18,.70,.57], 1., .32, [.025,.08,.06]),
]
# Only shell and core have geometry; the face is a front-facing tile of the
# external 64x64 skin selected by each creature JSON.
shell = make_mesh('square_translucent_shell', [([.96,.90,.96], (0,0,0))], 0, (0,0), (2,0))
core = make_mesh('square_nucleus', [([.58,.62,.58], (0,0,0))], 1, (1,0))
''' + source[end:]
start = source.index("body_children = [node('Shell',mesh=shell),inner]\n")
end = source.index("nodes[body]['children'] = body_children", start)
source = source[:start] + "body_children = [node('Shell',mesh=shell),inner]\n" + source[end:]
replace("'Asteria cubic pixel slime v2'", "'Asteria cubic pixel slime v3'")
replace("'color_materials':['SlimeShell','SlimeCore','SlimeCheeks']", "'color_materials':['SlimeShell','SlimeCore']")
replace("'notes':'Axis-aligned cube geometry; visual animation only, physics belongs to SlimeRoot'", "'notes':'Only cubic shell and enlarged core; face in species PNG front tile; collider does not animate'")
assert all(s not in source for s in ('square_eyes', 'square_eye_glints', 'square_cheeks', 'pixel_step_smile', "node('Eye_", "node('Smile'"))
GENERATOR.write_text(source, encoding='utf-8')

ns = runpy.run_path(str(GENERATOR), run_name='__main__')
for species in ORIGINAL:
    (SKINS/f'{species}.png').write_bytes(ns['make_skin'](species))
for name in ('meadow', 'ember'):
    p = ROOT/f'data/creatures/slimes/{name}.json'
    data = json.loads(p.read_text(encoding='utf-8'))
    for obsolete in ('SlimeEyes','SlimeHighlights','SlimeCheeks'):
        data['textures'].pop(obsolete, None)
        data['materialTints'].pop(obsolete, None)
    assert set(data['textures']) == {'SlimeShell','SlimeCore'}
    p.write_text(json.dumps(data, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')

glb = (GENERATOR.parent/'slime.glb').read_bytes()
magic, version, total = struct.unpack_from('<4sII', glb)
assert (magic, version, total) == (b'glTF', 2, len(glb))
chunk_length, chunk_type = struct.unpack_from('<I4s', glb, 12)
assert chunk_type == b'JSON'
gltf = json.loads(glb[20:20+chunk_length])
assert len(gltf['meshes']) == len(gltf['materials']) == 2
assert len(gltf['animations']) == 6
assert {m['name'] for m in gltf['materials']} == {'SlimeShell','SlimeCore'}
assert not any(k in gltf for k in ('images','textures','samplers'))
assert not any(any(term in n['name'].lower() for term in ('eye','glint','cheek','smile','mouth')) for n in gltf['nodes'])
assert gltf['meshes'][1]['name'] == 'square_nucleus'
assert gltf['accessors'][gltf['meshes'][1]['primitives'][0]['attributes']['POSITION']]['max'] == [.29,.31,.29]
for species in ORIGINAL:
    png = (SKINS/f'{species}.png').read_bytes()
    assert png[:8] == b'\x89PNG\r\n\x1a\n' and struct.unpack_from('>II',png,16) == (64,64)
    length = struct.unpack_from('>I',png,33)[0]
    assert png[37:41] == b'IDAT'
    raw = zlib.decompress(png[41:41+length])
    assert len(raw) == 64 * 257
    pixel = lambda u,v: raw[v*257+1+(u+32)*4]
    assert pixel(4,4) < 40 and pixel(3,4) == 255 and pixel(8,12) < 40
(ROOT/'VERSION').write_text('0.20.3\n',encoding='utf-8')
print('Validated: core 0.58x0.62x0.58, face in external 64x64 PNG, 2 meshes, 6 clips, no embedded images')
