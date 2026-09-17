#!/usr/bin/env python3
"""Generate Asteria's sharp-edged shared slime GLB and standalone 64x64 skins.

Uses only Python's standard library. Never rounds corners or smooths face details.
Creature JSON determines which PNG is loaded; GLB embeds no image data.
Physics stays on SlimeRoot; named animation clips move only visual children.
"""
from __future__ import annotations

import json
import math
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent
TEXTURES = OUT.parents[2] / 'textures' / 'creatures'
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


# Grayscale external atlas: shell, larger core and a pixel face on the front tile. Each
# species keeps its own PNG, selected exclusively through its creature JSON.
# Atlas tile positions must match the UV tile coordinates below. No face mesh.
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
            elif tile == (2, 0):
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
            elif tile == (1, 0):
                gray = 220 if min(u, v, 15-u, 15-v) < 2 else (246 if (u//4 + v//4) % 2 else 255)
            elif tile == (0, 1):
                gray = 245 if u < 4 or v < 4 else 255
            else:
                gray = 255
            index = (y * 64 + x) * 4
            pixels[index:index + 4] = bytes((gray, gray, gray, 255))
    scanlines = b''.join(b'\0' + pixels[y*64*4:(y+1)*64*4] for y in range(64))
    return (b'\x89PNG\r\n\x1a\n' + png_chunk(b'IHDR', struct.pack('>IIBBBBB', 64, 64, 8, 6, 0, 0, 0))
            + png_chunk(b'IDAT', zlib.compress(scanlines, 9)) + png_chunk(b'IEND', b''))


TEXTURES.mkdir(parents=True, exist_ok=True)
for species in ('meadow_slime', 'ember_slime'):
    destination = TEXTURES / f'{species}.png'
    # Only create missing default skins; never overwrite subsequent artist edits.
    if not destination.exists():
        destination.write_bytes(make_skin(species))

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


def material(name, color, alpha=1., rough=.36, emission=None):
    mat = {'name': name, 'pbrMetallicRoughness': {
        'baseColorFactor': [*color, alpha],
        'metallicFactor': 0, 'roughnessFactor': rough}, 'doubleSided': False}
    if alpha < 1:
        mat['alphaMode'] = 'BLEND'
    if emission is not None:
        mat['emissiveFactor'] = emission
    return mat


materials = [
    material('SlimeShell', [.50,.91,.78], .76, .23),
    material('SlimeCore', [.18,.70,.57], 1., .32, [.025,.08,.06]),
]
# Only shell and core have geometry; the face is a front-facing tile of the
# external 64x64 skin selected by each creature JSON.
shell = make_mesh('square_translucent_shell', [([.96,.90,.96], (0,0,0))], 0, (0,0), (2,0))
core = make_mesh('square_nucleus', [([.58,.62,.58], (0,0,0))], 1, (1,0))


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
inner = node('InnerCore', mesh=core, translation=[0,-.025,0], scale=[1,1,1])
body_children = [node('Shell',mesh=shell),inner]
nodes[body]['children'] = body_children
nodes[visual]['children'] = [body]
collider_node = node('Hitbox_AABB', translation=[0,.42,0], extras={
    'asteria_collider': {'shape':'aabb','size':[.78,.84,.78],'solid':True,'targetable':True},
    'debug_display': False})
nodes[root]['children'] = [visual,collider_node]


def tracks(clip, times, body_scale, center_y=None, core_scale=None, core_angle=None):
    assert len(times) == len(body_scale)
    center_y = center_y if center_y is not None else [.5*s[1] for s in body_scale]
    core_scale = core_scale if core_scale is not None else [[1,1,1]]*len(times)
    core_angle = core_angle if core_angle is not None else [0]*len(times)
    t = accessor(times, kind='SCALAR', bounds=True)
    channels, samplers = [], []
    def add(target, path, values, kind):
        out = accessor([v for item in values for v in item], kind=kind)
        samplers.append({'input':t,'output':out,'interpolation':'LINEAR'})
        channels.append({'sampler':len(samplers)-1,'target':{'node':target,'path':path}})
    add(body,'scale',body_scale,'VEC3')
    add(body,'translation',[[0,y,0] for y in center_y],'VEC3')
    add(inner,'scale',core_scale,'VEC3')
    add(inner,'rotation',[[0,math.sin(a/2),0,math.cos(a/2)] for a in core_angle],'VEC4')
    animations.append({'name':clip,'channels':channels,'samplers':samplers,
                       'extras':{'loop_recommended':clip in ('Idle','Airborne')}})


tracks('Idle',[0,.5,1,1.5,2],
       [[1,1,1],[1.018,.974,1.018],[1,1,1],[.988,1.021,.988],[1,1,1]],
       core_scale=[[1,1,1],[1.065,.97,1.065],[1,1,1],[.96,1.04,.96],[1,1,1]],
       core_angle=[0,.04,0,-.04,0])
tracks('Anticipate',[0,.07,.17,.24],
       [[1,1,1],[1.09,.845,1.09],[1.125,.76,1.125],[1.09,.83,1.09]],
       core_scale=[[1,1,1],[1.05,.9,1.05],[1.10,.84,1.10],[1.05,.92,1.05]],
       core_angle=[0,-.07,-.10,-.04])
tracks('Airborne',[0,.12,.35,.55,.72],
       [[1.09,.83,1.09],[.91,1.15,.91],[.96,1.085,.96],[.97,1.06,.97],[1,1,1]],
       core_scale=[[1,1,1],[.95,1.08,.95],[.98,1.035,.98],[1,1,1],[1,1,1]],
       core_angle=[0,.13,.23,.11,0])
tracks('Land',[0,.045,.12,.20,.34],
       [[.97,1.055,.97],[1.17,.74,1.17],[1.12,.805,1.12],[.975,1.047,.975],[1,1,1]],
       core_scale=[[1,1,1],[1.12,.85,1.12],[1.05,.94,1.05],[.98,1.025,.98],[1,1,1]])
tracks('Hurt',[0,.085,.15,.24,.38],
       [[1,1,1],[1.1,.88,1.1],[.94,1.08,.94],[1.025,.968,1.025],[1,1,1]],
       core_angle=[0,-.22,.19,-.08,0])
tracks('Death',[0,.12,.31,.55,.75],
       [[1,1,1],[1.13,.8,1.13],[1.2,.60,1.2],[1.12,.19,1.12],[.001,.001,.001]],
       center_y=[.5,.4,.3,.095,.0005],
       core_scale=[[1,1,1],[1.1,.88,1.1],[1.2,.7,1.2],[.95,.3,.95],[.001,.001,.001]],
       core_angle=[0,.2,.5,.9,1.2])

scene = {
    'asset':{'version':'2.0','generator':'Asteria cubic pixel slime v3'},
    'scene':0,'scenes':[{'name':'Slime','nodes':[root]}],
    'nodes':nodes,'meshes':meshes,'materials':materials,'animations':animations,
    'bufferViews':views,'accessors':accessors,'buffers':[{'byteLength':len(binary)}],
    'extras':{'asset_id':'asteria:slime_base','color_materials':['SlimeShell','SlimeCore'],
              'collision_source':'slime.collider.json','skin_resolution':[64,64],
              'texture_source':'creature JSON material textures under textures/creatures/',
              'notes':'Only cubic shell and enlarged core; face in species PNG front tile; collider does not animate'},
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
