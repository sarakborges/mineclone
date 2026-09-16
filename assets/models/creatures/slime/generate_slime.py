#!/usr/bin/env python3
"""Generate Asteria's sharp-edged slime GLB and its pixel-perfect 64x64 skin.

Uses only Python's standard library. Never rounds corners or smooths face details.
Physics stays on SlimeRoot; named animation clips move only visual children.
"""
from __future__ import annotations

import json
import math
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent
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


# An RGBA 64x64 atlas. Each 16x16 tile maps onto a cube face without bilinear filtering.
# White/grayscale skin preserves runtime HSI colors independently for each slime species.
pixels = bytearray([255, 255, 255, 255] * 64 * 64)
for y in range(64):
    for x in range(64):
        tile_x, tile_y = x // 16, y // 16
        u, v = x % 16, y % 16
        if (tile_x, tile_y) == (0, 0):  # outer shell: square pixel bands and sharp border
            gray = 232 if min(u, v, 15-u, 15-v) < 2 else (247 if (u//4 + v//4)%2 else 255)
        elif (tile_x, tile_y) == (1, 0):  # cubical nucleus: 4x4 pixel highlights
            gray = 225 if min(u, v, 15-u, 15-v) < 2 else (242 if (u//4 + v//4)%2 else 255)
        elif (tile_x, tile_y) == (0, 1):  # cheeks
            gray = 246 if u < 4 or v < 4 else 255
        else:
            gray = 255
        idx = (y*64+x)*4
        pixels[idx:idx+4] = bytes((gray, gray, gray, 255))


def png_chunk(kind: bytes, payload: bytes) -> bytes:
    content = kind + payload
    return struct.pack('>I', len(payload)) + content + struct.pack('>I', zlib.crc32(content) & 0xffffffff)


scanlines = b''.join(b'\0' + pixels[y*64*4:(y+1)*64*4] for y in range(64))
png = (b'\x89PNG\r\n\x1a\n' + png_chunk(b'IHDR', struct.pack('>IIBBBBB', 64, 64, 8, 6, 0, 0, 0))
       + png_chunk(b'IDAT', zlib.compress(scanlines, 9)) + png_chunk(b'IEND', b''))
(OUT / 'slime_skin_64.png').write_bytes(png)
image_view = store(png)

# Outward winding is verified by the axis-aligned normals and vertex corner order.
# normal, horizontal axis, vertical axis; cross(horizontal, vertical) == normal.
faces = [
    ((1,0,0), (0,0,-1), (0,1,0)), ((-1,0,0), (0,0,1), (0,1,0)),
    ((0,1,0), (1,0,0), (0,0,-1)), ((0,-1,0), (1,0,0), (0,0,1)),
    ((0,0,1), (1,0,0), (0,1,0)), ((0,0,-1), (-1,0,0), (0,1,0)),
]


def cube(positions, normals, uvs, indices, extent, center=(0,0,0), tile=(0,0)):
    half = [v/2 for v in extent]
    u0, u1 = (tile[0]*16 + .5)/64, (tile[0]*16 + 15.5)/64
    v0, v1 = (tile[1]*16 + .5)/64, (tile[1]*16 + 15.5)/64
    for normal, horizontal, vertical in faces:
        offset = len(positions)//3
        for a,b,uv in [(-1,-1,(u0,v1)), (1,-1,(u1,v1)),
                       (1,1,(u1,v0)), (-1,1,(u0,v0))]:
            point = [center[i] + normal[i]*half[i] + a*horizontal[i]*half[i]
                     + b*vertical[i]*half[i] for i in range(3)]
            positions.extend(point)
            normals.extend(normal)
            uvs.extend(uv)
        indices.extend([offset,offset+1,offset+2,offset,offset+2,offset+3])


def make_mesh(name, cuboids, material, tile):
    positions, normals, uvs, indices = [], [], [], []
    for extent, center in cuboids:
        cube(positions, normals, uvs, indices, extent, center, tile)
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
        'baseColorFactor': [*color, alpha], 'baseColorTexture': {'index': 0},
        'metallicFactor': 0, 'roughnessFactor': rough}, 'doubleSided': False}
    if alpha < 1:
        mat['alphaMode'] = 'BLEND'
    if emission is not None:
        mat['emissiveFactor'] = emission
    return mat


materials = [
    material('SlimeShell', [.50,.91,.78], .76, .23),
    material('SlimeCore', [.18,.70,.57], 1., .32, [.025,.08,.06]),
    material('SlimeEyes', [.055,.12,.115], 1., .34),
    material('SlimeHighlights', [.97,1.,.96], 1., .16, [.10,.10,.10]),
    material('SlimeCheeks', [.28,.71,.62], 1., .35),
]
shell = make_mesh('square_translucent_shell', [([.96,.90,.96], (0,0,0))], 0, (0,0))
core = make_mesh('square_nucleus', [([.36,.40,.36], (0,0,0))], 1, (1,0))
eye = make_mesh('square_eyes', [([.102,.125,.024], (0,0,0))], 2, (2,0))
glint = make_mesh('square_eye_glints', [([.025,.025,.009], (0,0,0))], 3, (3,0))
cheek = make_mesh('square_cheeks', [([.072,.046,.018], (0,0,0))], 4, (0,1))
# Five adjoining rectangles create a pixel-step smile; NO curves or cylindrical tubes.
smile_boxes = [([.056,.027,.018], (x, y, 0)) for x,y in
               [(-.112,-.108),(-.056,-.145),(0,-.162),(.056,-.145),(.112,-.108)]]
smile = make_mesh('pixel_step_smile', smile_boxes, 2, (2,0))


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
for x,label in [(-.177,'L'),(.177,'R')]:
    body_children.append(node('Eye_'+label,mesh=eye,translation=[x,.055,-.494]))
    body_children.append(node('Glint_'+label,mesh=glint,translation=[x-.018,.092,-.513]))
    body_children.append(node('Cheek_'+label,mesh=cheek,translation=[x*1.43,-.139,-.493]))
body_children.append(node('Smile',mesh=smile,translation=[0,0,-.502]))
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
    'asset':{'version':'2.0','generator':'Asteria cubic pixel slime v2'},
    'scene':0,'scenes':[{'name':'Slime','nodes':[root]}],
    'nodes':nodes,'meshes':meshes,'materials':materials,'animations':animations,
    'images':[{'name':'slime_skin_64','mimeType':'image/png','bufferView':image_view}],
    'samplers':[{'magFilter':9728,'minFilter':9728,'wrapS':33071,'wrapT':33071}],
    'textures':[{'source':0,'sampler':0}],
    'bufferViews':views,'accessors':accessors,'buffers':[{'byteLength':len(binary)}],
    'extras':{'asset_id':'asteria:slime_base','color_materials':['SlimeShell','SlimeCore','SlimeCheeks'],
              'collision_source':'slime.collider.json','skin_resolution':[64,64],
              'notes':'Axis-aligned cube geometry; visual animation only, physics belongs to SlimeRoot'},
}
json_chunk = json.dumps(scene,separators=(',',':'),ensure_ascii=False).encode('utf-8')
json_chunk += b' ' * (-len(json_chunk)%4)
bin_chunk = bytes(binary) + b'\0' * (-len(binary)%4)
glb = (struct.pack('<4sII',b'glTF',2,12+8+len(json_chunk)+8+len(bin_chunk))
       + struct.pack('<I4s',len(json_chunk),b'JSON')+json_chunk
       + struct.pack('<I4s',len(bin_chunk),b'BIN\0')+bin_chunk)
(OUT/'slime.glb').write_bytes(glb)
print(f'Generated {OUT/"slime.glb"}: {len(meshes)} sharp box meshes, '
      f'{len(animations)} animation clips, embedded 64x64 pixel skin')
