#!/usr/bin/env python3
"""Generate Asteria's second-generation rounded voxel slime.

This model is intentionally separate from the legacy slime assets. The body is
sampled as a voxel volume and only exposed voxel faces are emitted, producing a
continuous rounded blob silhouette with crisp pixel-art stepping in every axis.
"""
from __future__ import annotations

import json
import struct
from pathlib import Path

OUT = Path(__file__).resolve().parent
binary = bytearray()
views: list[dict] = []
accessors: list[dict] = []
meshes: list[dict] = []
nodes: list[dict] = []
animations: list[dict] = []

BODY_WIDTH = 1.20
BODY_HEIGHT = 1.00
BODY_DEPTH = 1.14
NX, NY, NZ = 24, 20, 22
DX, DY, DZ = BODY_WIDTH / NX, BODY_HEIGHT / NY, BODY_DEPTH / NZ
BODY_HALF_HEIGHT = BODY_HEIGHT * 0.5


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
    fmt = {5126: 'f', 5123: 'H'}[component]
    payload = struct.pack('<' + fmt * len(values), *values)
    entry = {
        'bufferView': store(payload, target),
        'componentType': component,
        'count': len(values) // width,
        'type': kind,
    }
    if bounds:
        rows = [values[i:i + width] for i in range(0, len(values), width)]
        entry['min'] = [float(min(row[j] for row in rows)) for j in range(width)]
        entry['max'] = [float(max(row[j] for row in rows)) for j in range(width)]
    accessors.append(entry)
    return len(accessors) - 1


def profile(t: float) -> float:
    keys = (
        (0.00, 0.66),
        (0.06, 0.79),
        (0.15, 0.91),
        (0.28, 0.995),
        (0.44, 1.00),
        (0.58, 0.965),
        (0.70, 0.89),
        (0.80, 0.76),
        (0.88, 0.60),
        (0.94, 0.36),
        (0.975, 0.14),
        (1.00, 0.02),
    )
    for (a_t, a_r), (b_t, b_r) in zip(keys, keys[1:]):
        if t <= b_t:
            u = (t - a_t) / (b_t - a_t)
            u = u * u * (3.0 - 2.0 * u)
            return a_r + (b_r - a_r) * u
    return keys[-1][1]


def occupied(ix: int, iy: int, iz: int) -> bool:
    x = (ix + 0.5) * DX - BODY_WIDTH * 0.5
    y = (iy + 0.5) * DY - BODY_HEIGHT * 0.5
    z = (iz + 0.5) * DZ - BODY_DEPTH * 0.5
    t = (y + BODY_HALF_HEIGHT) / BODY_HEIGHT
    radius = profile(t)

    crown_shift = 0.032 * max(0.0, (t - 0.62) / 0.38) ** 1.7
    x -= crown_shift
    rx = BODY_WIDTH * 0.5 * radius
    rz = BODY_DEPTH * 0.5 * radius
    if rx <= 0.0 or rz <= 0.0:
        return False

    exponent = 2.35
    return (abs(x / rx) ** exponent + abs(z / rz) ** exponent) <= 1.0


voxels = {
    (x, y, z)
    for y in range(NY)
    for z in range(NZ)
    for x in range(NX)
    if occupied(x, y, z)
}

top_y = max(y for _, y, _ in voxels)
top_count = sum(1 for _, y, _ in voxels if y == top_y)
assert top_count <= 12, f'crown became too flat: {top_count} voxels on top layer'

FACE_SPECS = [
    ((1, 0, 0), ((1, 0, 0), (1, 0, 1), (1, 1, 1), (1, 1, 0))),
    ((-1, 0, 0), ((0, 0, 1), (0, 0, 0), (0, 1, 0), (0, 1, 1))),
    ((0, 1, 0), ((0, 1, 0), (1, 1, 0), (1, 1, 1), (0, 1, 1))),
    ((0, -1, 0), ((0, 0, 1), (1, 0, 1), (1, 0, 0), (0, 0, 0))),
    ((0, 0, 1), ((1, 0, 1), (0, 0, 1), (0, 1, 1), (1, 1, 1))),
    ((0, 0, -1), ((0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0))),
]


def make_voxel_surface_mesh() -> int:
    positions, normals, uvs, indices = [], [], [], []
    for ix, iy, iz in sorted(voxels, key=lambda value: (value[1], value[2], value[0])):
        for normal, corners in FACE_SPECS:
            neighbor = (ix + normal[0], iy + normal[1], iz + normal[2])
            if neighbor in voxels:
                continue
            offset = len(positions) // 3
            if normal[1] > 0:
                tile_x = 2
            elif normal[1] < 0:
                tile_x = 3
            elif normal[0] != 0:
                tile_x = 1
            else:
                tile_x = 0
            u0, u1 = (tile_x * 16 + .5) / 64, (tile_x * 16 + 15.5) / 64
            v0, v1 = .5 / 64, 15.5 / 64
            face_uvs = ((u0, v1), (u1, v1), (u1, v0), (u0, v0))
            for (cx, cy, cz), uv in zip(corners, face_uvs):
                positions.extend((
                    -BODY_WIDTH * 0.5 + (ix + cx) * DX,
                    -BODY_HEIGHT * 0.5 + (iy + cy) * DY,
                    -BODY_DEPTH * 0.5 + (iz + cz) * DZ,
                ))
                normals.extend(normal)
                # Every exposed voxel face gets the full 16x16 soft-shade tile.
                # The 1px low-contrast border makes individual voxels readable
                # without reintroducing direction-dependent PBR lighting.
                uvs.extend(uv)
            indices.extend((offset, offset + 1, offset + 2, offset, offset + 2, offset + 3))

    assert len(positions) // 3 < 65536
    attrs = {
        'POSITION': accessor(positions, bounds=True, target=34962),
        'NORMAL': accessor(normals, target=34962),
        'TEXCOORD_0': accessor(uvs, kind='VEC2', target=34962),
    }
    meshes.append({
        'name': 'rounded_voxel_blob',
        'primitives': [{
            'attributes': attrs,
            'indices': accessor(indices, kind='SCALAR', component=5123, target=34963),
            'material': 0,
            'mode': 4,
        }],
    })
    return len(meshes) - 1


def make_front_quad() -> int:
    width, height = 0.86, 0.48
    z = -BODY_DEPTH * 0.5 - 0.006
    half_w, half_h = width / 2, height / 2
    y_center = -0.075
    positions = [
        half_w, y_center - half_h, z,
        -half_w, y_center - half_h, z,
        -half_w, y_center + half_h, z,
        half_w, y_center + half_h, z,
    ]
    normals = [0, 0, -1] * 4
    uvs = [0, 1, 1, 1, 1, 0, 0, 0]
    indices = [0, 1, 2, 0, 2, 3]
    attrs = {
        'POSITION': accessor(positions, bounds=True, target=34962),
        'NORMAL': accessor(normals, target=34962),
        'TEXCOORD_0': accessor(uvs, kind='VEC2', target=34962),
    }
    meshes.append({
        'name': 'pixel_face_decal',
        'primitives': [{
            'attributes': attrs,
            'indices': accessor(indices, kind='SCALAR', component=5123, target=34963),
            'material': 1,
            'mode': 4,
        }],
    })
    return len(meshes) - 1


def material(name: str, color, *, alpha_mode=None, unlit=False):
    result = {
        'name': name,
        'pbrMetallicRoughness': {
            'baseColorFactor': [*color, 1.0],
            'metallicFactor': 0.0,
            'roughnessFactor': 1.0,
        },
        'doubleSided': False,
    }
    if unlit:
        result['extensions'] = {'KHR_materials_unlit': {}}
    if alpha_mode:
        result['alphaMode'] = alpha_mode
    return result


materials = [
    # Both materials stay unlit so facing direction never produces harsh face-by-face
    # lighting. A subtle vertical shade is baked into shell_soft.png instead.
    material('SlimeShell', [.50, .91, .78], unlit=True),
    material('SlimeFace', [1.0, 1.0, 1.0], alpha_mode='BLEND', unlit=True),
]
shell_mesh = make_voxel_surface_mesh()
face_mesh = make_front_quad()


def node(name, mesh=None, children=None, translation=None, extras=None):
    entry = {'name': name}
    if mesh is not None:
        entry['mesh'] = mesh
    if children is not None:
        entry['children'] = children
    if translation is not None:
        entry['translation'] = translation
    if extras is not None:
        entry['extras'] = extras
    nodes.append(entry)
    return len(nodes) - 1


root = node('SlimeRoot', children=[], extras={
    'asteria_asset': 'creature/slime_blob',
    'unit': 'meters',
    'forward': '-Z',
    'collider': {'shape': 'aabb', 'size': [.78, .84, .78], 'offset': [0, .42, 0]},
    'collider_is_animated': False,
})
visual = node('Visual', children=[])
body = node('BodyPivot', children=[], translation=[0, BODY_HALF_HEIGHT, 0])
shell = node('Shell', mesh=shell_mesh)
face = node('Face', mesh=face_mesh)
nodes[body]['children'] = [shell, face]
nodes[visual]['children'] = [body]
collider_node = node('Hitbox_AABB', translation=[0, .42, 0], extras={
    'asteria_collider': {
        'shape': 'aabb',
        'size': [.78, .84, .78],
        'solid': True,
        'targetable': True,
    },
    'debug_display': False,
})
nodes[root]['children'] = [visual, collider_node]


def tracks(clip, times, body_scale, center_y=None):
    center_y = center_y if center_y is not None else [BODY_HALF_HEIGHT * scale[1] for scale in body_scale]
    time_accessor = accessor(times, kind='SCALAR', bounds=True)
    channels, samplers = [], []

    def add(target, path, values, kind):
        output = accessor([value for row in values for value in row], kind=kind)
        samplers.append({'input': time_accessor, 'output': output, 'interpolation': 'LINEAR'})
        channels.append({'sampler': len(samplers) - 1, 'target': {'node': target, 'path': path}})

    add(body, 'scale', body_scale, 'VEC3')
    add(body, 'translation', [[0, y, 0] for y in center_y], 'VEC3')
    animations.append({
        'name': clip,
        'channels': channels,
        'samplers': samplers,
        'extras': {'loop_recommended': clip in ('Idle', 'Airborne')},
    })


tracks('Idle', [0, .5, 1, 1.5, 2],
       [[1,1,1], [1.018,.974,1.018], [1,1,1], [.988,1.021,.988], [1,1,1]])
tracks('Anticipate', [0,.07,.17,.24],
       [[1,1,1], [1.09,.845,1.09], [1.125,.76,1.125], [1.09,.83,1.09]])
tracks('Airborne', [0,.12,.35,.55,.72],
       [[1.09,.83,1.09], [.91,1.15,.91], [.96,1.085,.96], [.97,1.06,.97], [1,1,1]])
tracks('Land', [0,.045,.12,.20,.34],
       [[.97,1.055,.97], [1.17,.74,1.17], [1.12,.805,1.12], [.975,1.047,.975], [1,1,1]])
tracks('Hurt', [0,.085,.15,.24,.38],
       [[1,1,1], [1.1,.88,1.1], [.94,1.08,.94], [1.025,.968,1.025], [1,1,1]])
tracks('Death', [0,.12,.31,.55,.75],
       [[1,1,1], [1.13,.8,1.13], [1.2,.60,1.2], [1.12,.19,1.12], [.001,.001,.001]],
       center_y=[BODY_HALF_HEIGHT, .4, .3, .095, .0005])

scene = {
    'asset': {'version': '2.0', 'generator': 'Asteria sampled voxel blob slime v1'},
    'scene': 0,
    'scenes': [{'name': 'SlimeBlob', 'nodes': [root]}],
    'extensionsUsed': ['KHR_materials_unlit'],
    'nodes': nodes,
    'meshes': meshes,
    'materials': materials,
    'animations': animations,
    'bufferViews': views,
    'accessors': accessors,
    'buffers': [{'byteLength': len(binary)}],
    'extras': {
        'asset_id': 'asteria:slime_blob',
        'color_materials': ['SlimeShell', 'SlimeFace'],
        'collision_source': 'slime_blob.collider.json',
        'texture_source': 'creature JSON material textures under textures/creatures/',
        'voxel_resolution': [NX, NY, NZ],
        'occupied_voxels': len(voxels),
        'top_layer_voxels': top_count,
        'notes': 'Second-generation sampled voxel blob. Each exposed voxel maps a low-contrast shell_soft tile so blocks remain readable without directional lighting. Legacy slime assets remain untouched.',
    },
}
json_chunk = json.dumps(scene, separators=(',', ':'), ensure_ascii=False).encode('utf-8')
json_chunk += b' ' * (-len(json_chunk) % 4)
bin_chunk = bytes(binary) + b'\0' * (-len(binary) % 4)
glb = (
    struct.pack('<4sII', b'glTF', 2, 12 + 8 + len(json_chunk) + 8 + len(bin_chunk))
    + struct.pack('<I4s', len(json_chunk), b'JSON') + json_chunk
    + struct.pack('<I4s', len(bin_chunk), b'BIN\0') + bin_chunk
)
(OUT / 'slime_blob.glb').write_bytes(glb)
print(f'Generated {OUT / "slime_blob.glb"}: {len(voxels)} voxels, {top_count} top-layer voxels')
