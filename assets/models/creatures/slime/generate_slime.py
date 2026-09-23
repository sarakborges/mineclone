#!/usr/bin/env python3
"""Generate Asteria's pixel-rounded shared slime GLB and standalone 64x64 skins.

Uses only Python's standard library. The silhouette stays deliberately blocky: stepped
cuboids carve the upper corners into a pixel-art dome without smooth geometry.
Creature JSON determines which PNG is loaded; GLB embeds no image data.
Physics stays on SlimeRoot; named animation clips move only visual children.
"""
from __future__ import annotations

import json
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent
TEXTURES = OUT.parents[2] / 'textures' / 'creatures' / 'slime'
binary = bytearray()
views: list[dict] = []
accessors: list[dict] = []
meshes: list[dict] = []
nodes: list[dict] = []
animations: list[dict] = []


def store(data: bytes, target: int | None = None) -> int:
    binary.extend(b'\0' * (-len(binary) % 4))
    offset = len(binary)
    binary.extend(data)
    entry = {'buffer': 0, 'byteOffset': offset, 'byteLength': len(data)}
    if target is not None:
        entry['target'] = target
    views.append(entry)
    return len(views) - 1


def accessor(values, kind='VEC3', component=5126, target=None, bounds=False):
    width = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}[kind]
    assert len(values) % width == 0
    fmt = {5126: 'f', 5123: 'H'}[component]
    payload = struct.pack('<' + fmt * len(values), *values)
    entry = {'bufferView': store(payload, target), 'componentType': component,
             'count': len(values) // width, 'type': kind}
    if bounds:
        rows = [values[i:i + width] for i in range(0, len(values), width)]
        entry['min'] = [float(min(row[j] for row in rows)) for j in range(width)]
        entry['max'] = [float(max(row[j] for row in rows)) for j in range(width)]
    accessors.append(entry)
    return len(accessors) - 1


# Standalone grayscale skins for the shell and face. Each species keeps its own
# PNG selection through creature JSON; the GLB embeds no image data.
def png_chunk(kind: bytes, payload: bytes) -> bytes:
    content = kind + payload
    return (struct.pack('>I', len(payload)) + content
            + struct.pack('>I', zlib.crc32(content) & 0xffffffff))


def make_skin(kind: str) -> bytes:
    """Create a standalone 64x64 grayscale skin for one slime material."""
    pixels = bytearray([255, 255, 255, 255] * 64 * 64)
    for y in range(64):
        for x in range(64):
            u, v = x % 64, y % 64
            if kind == 'shell':
                border = min(u, v, 63-u, 63-v) < 4
                gray = 229 if border else (247 if (u//8 + v//8) % 2 else 255)
            elif kind == 'face':
                gray = 255
                alpha = 0
                if 20 <= v <= 29 and (16 <= u <= 23 or 40 <= u <= 47):
                    gray, alpha = 15, 255
                if v == 20 and u in (16, 40):
                    gray, alpha = 255, 255
                if 29 <= v <= 31 and (8 <= u <= 13 or 50 <= u <= 55):
                    gray, alpha = 170, 255
                if (v == 38 and 24 <= u <= 39) or (v == 39 and 27 <= u <= 36):
                    gray, alpha = 24, 255
            else:
                raise ValueError(f'unknown slime skin kind: {kind}')
            index = (y * 64 + x) * 4
            if kind == 'face':
                pixels[index:index + 4] = bytes((gray, gray, gray, alpha))
            else:
                pixels[index:index + 4] = bytes((gray, gray, gray, 255))
    scanlines = b''.join(b'\0' + pixels[y*64*4:(y+1)*64*4] for y in range(64))
    return (b'\x89PNG\r\n\x1a\n' + png_chunk(b'IHDR', struct.pack('>IIBBBBB', 64, 64, 8, 6, 0, 0, 0))
            + png_chunk(b'IDAT', zlib.compress(scanlines, 9)) + png_chunk(b'IEND', b''))


TEXTURES.mkdir(parents=True, exist_ok=True)
for kind in ('shell', 'face'):
    destination = TEXTURES / f'{kind}.png'
    # Only create missing default skins; never overwrite subsequent artist edits.
    if not destination.exists():
        destination.write_bytes(make_skin(kind))

# Outward winding is verified by the axis-aligned normals and vertex corner order.
# normal, horizontal axis, vertical axis; cross(horizontal, vertical) == normal.
faces = [
    ((1,0,0), (0,0,-1), (0,1,0)), ((-1,0,0), (0,0,1), (0,1,0)),
    ((0,1,0), (1,0,0), (0,0,-1)), ((0,-1,0), (1,0,0), (0,0,1)),
    ((0,0,1), (1,0,0), (0,1,0)), ((0,0,-1), (-1,0,0), (0,1,0)),
]


def cube(positions, normals, uvs, indices, extent, center=(0,0,0), tile=(0,0), front_tile=None):
    half = [v/2 for v in extent]
    for normal, horizontal, vertical in faces:
        face_tile = front_tile if front_tile is not None and normal == (0,0,-1) else tile
        u0, u1 = (face_tile[0]*16 + .5)/64, (face_tile[0]*16 + 15.5)/64
        v0, v1 = (face_tile[1]*16 + .5)/64, (face_tile[1]*16 + 15.5)/64
        offset = len(positions)//3
        for a,b,uv in [(-1,-1,(u0,v1)), (1,-1,(u1,v1)),
                       (1,1,(u1,v0)), (-1,1,(u0,v0))]:
            point = [center[i] + normal[i]*half[i] + a*horizontal[i]*half[i]
                     + b*vertical[i]*half[i] for i in range(3)]
            positions.extend(point)
            normals.extend(normal)
            uvs.extend(uv)
        indices.extend([offset,offset+1,offset+2,offset,offset+2,offset+3])


def make_mesh(name, cuboids, material, tile, front_tile=None):
    positions, normals, uvs, indices = [], [], [], []
    for extent, center in cuboids:
        cube(positions, normals, uvs, indices, extent, center, tile, front_tile)
    assert len(positions)//3 < 65536
    attrs = {'POSITION': accessor(positions, bounds=True, target=34962),
             'NORMAL': accessor(normals, target=34962),
             'TEXCOORD_0': accessor(uvs, kind='VEC2', target=34962)}
    mesh = {'name': name, 'primitives': [{
        'attributes': attrs, 'indices': accessor(indices, kind='SCALAR', component=5123, target=34963),
        'material': material, 'mode': 4}]}
    meshes.append(mesh)
    return len(meshes)-1


def make_front_quad(name, width, height, z, material):
    half_w, half_h = width / 2, height / 2
    positions = [
        half_w, -half_h, z,
        -half_w, -half_h, z,
        -half_w, half_h, z,
        half_w, half_h, z,
    ]
    normals = [0, 0, -1] * 4
    uvs = [0, 1, 1, 1, 1, 0, 0, 0]
    indices = [0, 1, 2, 0, 2, 3]
    attrs = {'POSITION': accessor(positions, bounds=True, target=34962),
             'NORMAL': accessor(normals, target=34962),
             'TEXCOORD_0': accessor(uvs, kind='VEC2', target=34962)}
    mesh = {'name': name, 'primitives': [{
        'attributes': attrs,
        'indices': accessor(indices, kind='SCALAR', component=5123, target=34963),
        'material': material,
        'mode': 4,
    }]}
    meshes.append(mesh)
    return len(meshes)-1


def material(name, color, alpha=1., rough=.36, emission=None, alpha_mode=None):
    mat = {'name': name, 'pbrMetallicRoughness': {
        'baseColorFactor': [*color, alpha],
        'metallicFactor': 0, 'roughnessFactor': rough}, 'doubleSided': False,
        'extensions': {'KHR_materials_unlit': {}}}
    if alpha_mode is not None:
        mat['alphaMode'] = alpha_mode
    elif alpha < 1:
        mat['alphaMode'] = 'BLEND'
    if emission is not None:
        mat['emissiveFactor'] = emission
    return mat


materials = [
    # The slime is a single shell plus a transparent face decal. Both source
    # materials are fully rough/unlit so orientation never changes brightness.
    material('SlimeShell', [.50,.91,.78], 1., 1.0),
    material('SlimeFace', [1.,1.,1.], 1., 1.0, alpha_mode='BLEND'),
]
# Higher-resolution voxel dome based on the approved reference. The model is
# intentionally a little larger than before so the silhouette can use more,
# smaller steps instead of ending in a broad flat cap. Height is 1.00 m and the
# widest tier is 1.20 m; only the visible mesh is affected.
shell_profile = [
    ([.78, .04, .78], (0, -.480, 0)),
    ([.90, .06, .90], (0, -.430, 0)),
    ([1.00, .08, 1.00], (0, -.360, 0)),
    ([1.10, .10, 1.10], (0, -.270, 0)),
    ([1.16, .12, 1.16], (0, -.160, 0)),
    ([1.20, .14, 1.20], (0, -.030, 0)),
    ([1.20, .14, 1.20], (0,  .110, 0)),
    ([1.16, .12, 1.16], (0,  .240, 0)),
    ([1.08, .08, 1.08], (0,  .340, 0)),
    ([.96, .05, .96], (0,  .405, 0)),
    ([.82, .03, .82], (0,  .445, 0)),
    ([.64, .02, .64], (0,  .470, 0)),
    ([.44, .015, .44], (0, .4875, 0)),
    ([.22, .005, .22], (0, .4975, 0)),
]
shell = make_mesh('pixel_rounded_shell', shell_profile, 0, (0,0))
face = make_front_quad('square_pixel_face', .98, .60, -.606, 1)


def node(name, mesh=None, children=None, translation=None, scale=None, extras=None):
    entry = {'name': name}
    if mesh is not None: entry['mesh'] = mesh
    if children is not None: entry['children'] = children
    if translation is not None: entry['translation'] = translation
    if scale is not None: entry['scale'] = scale
    if extras is not None: entry['extras'] = extras
    nodes.append(entry)
    return len(nodes)-1


root = node('SlimeRoot', children=[], extras={
    'asteria_asset': 'creature/slime', 'unit': 'meters', 'forward': '-Z',
    'collider': {'shape':'aabb','size':[.78,.84,.78],'offset':[0,.42,0]},
    'collider_is_animated': False})
visual = node('Visual', children=[])
body = node('BodyPivot', children=[], translation=[0,.5,0])
body_children = [node('Shell',mesh=shell), node('Face',mesh=face,translation=[0,-.10,0])]
nodes[body]['children'] = body_children
nodes[visual]['children'] = [body]
collider_node = node('Hitbox_AABB', translation=[0,.42,0], extras={
    'asteria_collider': {'shape':'aabb','size':[.78,.84,.78],'solid':True,'targetable':True},
    'debug_display': False})
nodes[root]['children'] = [visual,collider_node]


def tracks(clip, times, body_scale, center_y=None):
    assert len(times) == len(body_scale)
    center_y = center_y if center_y is not None else [.5*s[1] for s in body_scale]
    t = accessor(times, kind='SCALAR', bounds=True)
    channels, samplers = [], []
    def add(target, path, values, kind):
        out = accessor([v for item in values for v in item], kind=kind)
        samplers.append({'input':t,'output':out,'interpolation':'LINEAR'})
        channels.append({'sampler':len(samplers)-1,'target':{'node':target,'path':path}})
    add(body,'scale',body_scale,'VEC3')
    add(body,'translation',[[0,y,0] for y in center_y],'VEC3')
    animations.append({'name':clip,'channels':channels,'samplers':samplers,
                       'extras':{'loop_recommended':clip in ('Idle','Airborne')}})


tracks('Idle',[0,.5,1,1.5,2],
       [[1,1,1],[1.018,.974,1.018],[1,1,1],[.988,1.021,.988],[1,1,1]])
tracks('Anticipate',[0,.07,.17,.24],
       [[1,1,1],[1.09,.845,1.09],[1.125,.76,1.125],[1.09,.83,1.09]])
tracks('Airborne',[0,.12,.35,.55,.72],
       [[1.09,.83,1.09],[.91,1.15,.91],[.96,1.085,.96],[.97,1.06,.97],[1,1,1]])
tracks('Land',[0,.045,.12,.20,.34],
       [[.97,1.055,.97],[1.17,.74,1.17],[1.12,.805,1.12],[.975,1.047,.975],[1,1,1]])
tracks('Hurt',[0,.085,.15,.24,.38],
       [[1,1,1],[1.1,.88,1.1],[.94,1.08,.94],[1.025,.968,1.025],[1,1,1]])
tracks('Death',[0,.12,.31,.55,.75],
       [[1,1,1],[1.13,.8,1.13],[1.2,.60,1.2],[1.12,.19,1.12],[.001,.001,.001]],
       center_y=[.5,.4,.3,.095,.0005])

scene = {
    'asset':{'version':'2.0','generator':'Asteria high-resolution rounded slime v7'},
    'scene':0,'scenes':[{'name':'Slime','nodes':[root]}],
    'extensionsUsed':['KHR_materials_unlit'],
    'nodes':nodes,'meshes':meshes,'materials':materials,'animations':animations,
    'bufferViews':views,'accessors':accessors,'buffers':[{'byteLength':len(binary)}],
    'extras':{'asset_id':'asteria:slime_base','color_materials':['SlimeShell','SlimeFace'],
              'collision_source':'slime.collider.json','skin_resolution':[64,64],
              'texture_source':'creature JSON material textures under textures/creatures/',
              'notes':'Single high-resolution pixel-rounded shell with transparent face decal; no core; unlit materials keep brightness direction-independent; collider does not animate'},
}
json_chunk = json.dumps(scene,separators=(',',':'),ensure_ascii=False).encode('utf-8')
json_chunk += b' ' * (-len(json_chunk)%4)
bin_chunk = bytes(binary) + b'\0' * (-len(binary)%4)
glb = (struct.pack('<4sII',b'glTF',2,12+8+len(json_chunk)+8+len(bin_chunk))
       + struct.pack('<I4s',len(json_chunk),b'JSON')+json_chunk
       + struct.pack('<I4s',len(bin_chunk),b'BIN\0')+bin_chunk)
(OUT/'slime.glb').write_bytes(glb)
print(f'Generated {OUT/"slime.glb"}: {len(meshes)} sharp box meshes, '
      f'{len(animations)} animation clips, JSON-selected standalone 64x64 PNG skins')
