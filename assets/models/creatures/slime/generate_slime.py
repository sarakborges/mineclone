#!/usr/bin/env python3
"""Generate an original, color-customizable animated glTF 2.0 slime asset.

Self-contained (Python 3 + numpy + trimesh); no Blender or texture files needed.
Animates only visual child nodes. The collider is a separate, static definition.
"""
from __future__ import annotations
import json
import math
import struct
from pathlib import Path
import numpy as np
import trimesh

OUT = Path(__file__).resolve().parent
binary = bytearray()
buffer_views = []
accessors = []
meshes = []
nodes = []
animations = []


def append_bytes(data: bytes, target=None) -> int:
    while len(binary) % 4:
        binary.append(0)
    offset = len(binary)
    binary.extend(data)
    view = {'buffer': 0, 'byteOffset': offset, 'byteLength': len(data)}
    if target is not None:
        view['target'] = target
    buffer_views.append(view)
    return len(buffer_views) - 1


def accessor(values, component=5126, kind='VEC3', target=None, bounds=False):
    dtype = {5126: '<f4', 5123: '<u2', 5125: '<u4'}[component]
    ar = np.asarray(values, dtype=dtype)
    if kind == 'SCALAR':
        ar = ar.reshape(-1)
    else:
        ar = ar.reshape(-1, {'VEC3': 3, 'VEC4': 4}[kind])
    view = append_bytes(ar.tobytes(order='C'), target=target)
    a = {'bufferView': view, 'componentType': component, 'count': len(ar), 'type': kind}
    if bounds:
        if ar.ndim == 1:
            a['min'], a['max'] = [float(ar.min())], [float(ar.max())]
        else:
            a['min'] = [float(x) for x in ar.min(axis=0)]
            a['max'] = [float(x) for x in ar.max(axis=0)]
    accessors.append(a)
    return len(accessors) - 1


def add_mesh(name, vertices, faces, material, normals=None):
    v = np.asarray(vertices, dtype=np.float32).reshape(-1, 3)
    f = np.asarray(faces, dtype=np.uint32).reshape(-1, 3)
    if normals is None:
        mesh = trimesh.Trimesh(vertices=v, faces=f, process=False)
        n = np.asarray(mesh.vertex_normals, dtype=np.float32)
    else:
        n = np.asarray(normals, dtype=np.float32).reshape(-1, 3)
    p_idx = accessor(v, target=34962, bounds=True)
    n_idx = accessor(n, target=34962)
    index_type = 5123 if len(v) < 65536 else 5125
    i_idx = accessor(f.flatten(), component=index_type, kind='SCALAR', target=34963)
    meshes.append({'name': name, 'primitives': [{
        'attributes': {'POSITION': p_idx, 'NORMAL': n_idx},
        'indices': i_idx, 'material': material, 'mode': 4,
    }]})
    return len(meshes) - 1


def rounded_box(extents, radius=.13, segments=12):
    half = np.asarray(extents, dtype=float) / 2
    inner = half - radius
    verts, norms, faces = [], [], []
    for axis, ua, va in [(0, 1, 2), (1, 2, 0), (2, 0, 1)]:
        for sign in [-1, 1]:
            origin = len(verts)
            for i in range(segments + 1):
                u = -half[ua] + 2 * half[ua] * i / segments
                for j in range(segments + 1):
                    v = -half[va] + 2 * half[va] * j / segments
                    p = np.zeros(3)
                    p[axis], p[ua], p[va] = sign * half[axis], u, v
                    q = np.clip(p, -inner, inner)
                    direction = p - q
                    length = np.linalg.norm(direction)
                    direction = direction / length if length > 1e-9 else np.eye(3)[axis] * sign
                    verts.append((q + radius * direction).tolist())
                    norms.append(direction.tolist())
            for i in range(segments):
                for j in range(segments):
                    a = origin + i * (segments + 1) + j
                    b = origin + (i + 1) * (segments + 1) + j
                    c, d = a + 1, b + 1
                    if sign > 0:
                        faces.extend([[a, b, c], [b, d, c]])
                    else:
                        faces.extend([[a, c, b], [b, c, d]])
    return verts, faces, norms


def ellipsoid(extents, center, subdivisions=2):
    shape = trimesh.creation.icosphere(subdivisions=subdivisions, radius=1.0)
    v = np.asarray(shape.vertices) * np.asarray(extents) + np.asarray(center)
    normal = np.asarray(shape.vertices) / np.asarray(extents)
    normal /= np.linalg.norm(normal, axis=1, keepdims=True)
    return v, shape.faces, normal


def mouth_curve():
    # Small low-poly smile on the front (-Z), designed to be readable at game distance.
    points = []
    for i in range(13):
        x = -0.152 + .304 * i / 12
        y = -.145 + .063 * (x / .152) ** 2
        points.append(np.array([x, y, -.529], dtype=float))
    v, n, f = [], [], []
    sides = 7
    for j, p in enumerate(points):
        tangent = points[min(j+1, len(points)-1)] - points[max(j-1, 0)]
        tangent /= np.linalg.norm(tangent)
        axis = np.array([0., 0., 1.])
        binormal = np.cross(tangent, axis)
        binormal /= np.linalg.norm(binormal)
        for k in range(sides):
            angle = k * math.tau / sides
            norm = binormal * math.cos(angle) + axis * math.sin(angle)
            v.append((p + .014 * norm).tolist())
            n.append(norm.tolist())
    for j in range(len(points)-1):
        for k in range(sides):
            a = j*sides + k
            b = j*sides + (k+1)%sides
            c = (j+1)*sides + k
            d = (j+1)*sides + (k+1)%sides
            f.extend([[a, b, c], [b, d, c]])
    return v, f, n


def mat(name, rgb, alpha=1, rough=.36, metal=0, emissive=None, double=False):
    pbr = {'baseColorFactor': [*rgb, alpha], 'metallicFactor': metal, 'roughnessFactor': rough}
    material = {'name': name, 'pbrMetallicRoughness': pbr, 'doubleSided': double}
    if alpha < 1:
        material['alphaMode'] = 'BLEND'
    if emissive is not None:
        material['emissiveFactor'] = emissive
    return material

materials = [
    mat('SlimeShell', [.50, .91, .78], .76, rough=.23),
    mat('SlimeCore', [.18, .70, .57], 1, rough=.32, emissive=[.025, .08, .06]),
    mat('SlimeEyes', [.055, .12, .115], 1, rough=.34),
    mat('SlimeHighlights', [.97, 1., .96], 1, rough=.16, emissive=[.10, .10, .10]),
    mat('SlimeCheeks', [.28, .71, .62], 1, rough=.35),
]


def node(name, mesh=None, children=None, translation=None, scale=None, rotation=None, extras=None):
    result = {'name': name}
    if mesh is not None: result['mesh'] = mesh
    if children is not None: result['children'] = children
    if translation is not None: result['translation'] = translation
    if scale is not None: result['scale'] = scale
    if rotation is not None: result['rotation'] = rotation
    if extras is not None: result['extras'] = extras
    nodes.append(result)
    return len(nodes)-1

v, f, n = rounded_box([.96, .90, .96], .135, 12)
shell = add_mesh('rounded_translucent_gel', v, f, 0, normals=n)
# Crystalline nucleus with its own bob/rotation animation.
v, f, n = ellipsoid([.20, .24, .19], [0, 0, 0], 1)
core = add_mesh('faceted_nucleus', v, f, 1, normals=n)
# Raised front-facing eyes rather than borrowing Minecraft's pixel face.
v, f, n = ellipsoid([.072, .112, .036], [0, 0, 0], 2)
eye = add_mesh('oval_eyes', v, f, 2, normals=n)
v, f, n = ellipsoid([.020, .032, .012], [0, 0, 0], 1)
glint = add_mesh('eye_glints', v, f, 3, normals=n)
v, f, n = mouth_curve()
mouth = add_mesh('curved_smile', v, f, 2, normals=n)
v, f, n = ellipsoid([.052, .026, .018], [0, 0, 0], 1)
cheek = add_mesh('gel_cheeks', v, f, 4, normals=n)

root = node('SlimeRoot', children=[1], extras={
    'asteria_asset': 'creature/slime', 'unit': 'meters', 'forward': '-Z',
    'collider': {'shape': 'aabb', 'size': [.78, .84, .78], 'offset': [0, .42, 0]},
    'collider_is_animated': False,
})
visual = node('Visual', children=[2])
body = node('BodyPivot', translation=[0, .5, 0], children=[])
inner = node('InnerCore', mesh=core, translation=[0, -.025, 0], scale=[1,1,1])
body_children = [node('Shell', mesh=shell), inner]
for x in [-.177, .177]:
    body_children.append(node('Eye_L' if x < 0 else 'Eye_R', mesh=eye, translation=[x,.055,-.501]))
    body_children.append(node('Glint_L' if x < 0 else 'Glint_R', mesh=glint, translation=[x-.016,.09,-.538]))
    body_children.append(node('Cheek_L' if x < 0 else 'Cheek_R', mesh=cheek, translation=[x*1.43,-.139,-.491]))
body_children.append(node('Smile', mesh=mouth))
nodes[body]['children'] = body_children
# Dedicated metadata-only child: physics must use this AABB on the unanimated root.
collider_node = node('Hitbox_AABB', translation=[0,.42,0], extras={
    'asteria_collider': {'shape': 'aabb', 'size': [.78,.84,.78], 'solid': True, 'targetable': True},
    'debug_display': False,
})
nodes[root]['children'].append(collider_node)


def tracks(clip, times, body_scale, center_y=None, core_scale=None, core_angle=None):
    """Key visual nodes only. World position, velocity and collider belong to ECS."""
    assert len(times)==len(body_scale)
    center_y = center_y or [0.5*s[1] for s in body_scale]
    if core_scale is None: core_scale = [[1,1,1]]*len(times)
    if core_angle is None: core_angle = [0]*len(times)
    t = accessor(times, kind='SCALAR', bounds=True)
    channels, samplers = [], []
    def add(target_node, path, vals, kind):
        a = accessor(vals, kind=kind)
        idx = len(samplers)
        samplers.append({'input': t, 'output': a, 'interpolation': 'LINEAR'})
        channels.append({'sampler': idx, 'target': {'node': target_node, 'path': path}})
    add(body, 'scale', body_scale, 'VEC3')
    add(body, 'translation', [[0,y,0] for y in center_y], 'VEC3')
    add(inner, 'scale', core_scale, 'VEC3')
    add(inner, 'rotation', [[0, math.sin(a/2), 0, math.cos(a/2)] for a in core_angle], 'VEC4')
    animations.append({'name': clip, 'channels': channels, 'samplers': samplers,
                       'extras': {'loop_recommended': clip in ('Idle','Airborne')}})

tracks('Idle', [0,.5,1,1.5,2],
       [[1,1,1],[1.018,.974,1.018],[1,1,1],[.988,1.021,.988],[1,1,1]],
       core_scale=[[1,1,1],[1.065,.97,1.065],[1,1,1],[.96,1.04,.96],[1,1,1]],
       core_angle=[0,.04,0,-.04,0])
tracks('Anticipate', [0,.07,.17,.24],
       [[1,1,1],[1.09,.845,1.09],[1.125,.76,1.125],[1.09,.83,1.09]],
       core_scale=[[1,1,1],[1.05,.9,1.05],[1.10,.84,1.10],[1.05,.92,1.05]],
       core_angle=[0,-.07,-.10,-.04])
tracks('Airborne', [0,.12,.35,.55,.72],
       [[1.09,.83,1.09],[.91,1.15,.91],[.96,1.085,.96],[.97,1.06,.97],[1,1,1]],
       core_scale=[[1,1,1],[.95,1.08,.95],[.98,1.035,.98],[1,1,1],[1,1,1]],
       core_angle=[0,.13,.23,.11,0])
tracks('Land', [0,.045,.12,.20,.34],
       [[.97,1.055,.97],[1.17,.74,1.17],[1.12,.805,1.12],[.975,1.047,.975],[1,1,1]],
       core_scale=[[1,1,1],[1.12,.85,1.12],[1.05,.94,1.05],[.98,1.025,1.05],[1,1,1]])
tracks('Hurt', [0,.085,.15,.24,.38],
       [[1,1,1],[1.1,.88,1.1],[.94,1.08,.94],[1.025,.968,1.025],[1,1,1]],
       core_angle=[0,-.22,.19,-.08,0])
tracks('Death', [0,.12,.31,.55,.75],
       [[1,1,1],[1.13,.8,1.13],[1.2,.60,1.2],[1.12,.19,1.12],[0.001,.001,.001]],
       center_y=[.5,.4,.3,.095,.0005],
       core_scale=[[1,1,1],[1.1,.88,1.1],[1.2,.7,1.2],[.95,.3,.95],[.001,.001,.001]],
       core_angle=[0,.2,.5,.9,1.2])

scene = {
    'asset': {'version': '2.0', 'generator': 'Asteria procedural slime v1'},
    'scene': 0, 'scenes': [{'name':'Slime', 'nodes':[root]}],
    'nodes': nodes, 'meshes': meshes, 'materials': materials,
    'animations': animations, 'bufferViews': buffer_views, 'accessors': accessors,
    'buffers': [{'byteLength': len(binary)}],
    'extras': {'asset_id': 'asteria:slime_base', 'color_materials': ['SlimeShell','SlimeCore','SlimeCheeks'],
               'collision_source': 'slime.collider.json',
               'notes': 'glTF transforms animate BodyPivot/InnerCore only; physics owns SlimeRoot'},
}
json_chunk = json.dumps(scene, separators=(',', ':'), ensure_ascii=False).encode('utf-8')
json_chunk += b' ' * (-len(json_chunk) % 4)
bin_chunk = bytes(binary) + b'\x00' * (-len(binary) % 4)
size = 12 + 8 + len(json_chunk) + 8 + len(bin_chunk)
glb = (struct.pack('<4sII', b'glTF', 2, size) +
       struct.pack('<I4s', len(json_chunk), b'JSON') + json_chunk +
       struct.pack('<I4s', len(bin_chunk), b'BIN\x00') + bin_chunk)
(OUT / 'slime.glb').write_bytes(glb)

config = {
  'schema_version': 1, 'id': 'asteria:slime_base',
  'asset': 'slime.glb', 'units': 'meters', 'world_up': '+Y', 'forward': '-Z',
  'origin': 'center of ground contact, feet at local y = 0',
  'model_bounds_rest': {'size': [.96,.90,.96], 'center': [0,.5,0], 'approximate_visual_height': .95},
  'collider': {
    'type': 'aabb', 'size': [.78,.84,.78], 'center_offset': [0,.42,0],
    'min_offset': [-.39,0,-.39], 'max_offset': [.39,.84,.39],
    'follows': 'SlimeRoot world transform, NOT BodyPivot scale or GLB animation',
    'solid_world_collision': True, 'targeting': True,
  },
  'animation': {
    'clips': {
      'Idle': {'duration_s':2.0, 'loop':True},
      'Anticipate': {'duration_s':.24, 'loop':False},
      'Airborne': {'duration_s':.72, 'loop':True},
      'Land': {'duration_s':.34, 'loop':False},
      'Hurt': {'duration_s':.38, 'loop':False},
      'Death': {'duration_s':.75, 'loop':False},
    },
    'root_motion': False, 'visual_root': 'BodyPivot',
    'jump_sequence': ['Anticipate','Airborne','Land','Idle'],
    'transition_rule': 'Actual lift, gravity, collision and landing come from movement simulation; animations never move collider.',
  },
  'tint': {
    'authoritative_color_space': 'HSI', 'hue_range': [0,360],
    'material_slots': ['SlimeShell','SlimeCore','SlimeCheeks'],
    'independent_slots': ['SlimeEyes','SlimeHighlights'],
    'runtime_rule': 'Create or cache a material set per species; do not mutate the shared glTF asset material across species.',
    'alpha_of_SlimeShell': .76,
    'core_shade_rule': 'Use same species hue with darker intensity for nucleus; cheek shade is optional.',
  },
  'species_examples': {
    'meadow': {'hue':153, 'saturation':.62, 'intensity':.64},
    'arcane': {'hue':275, 'saturation':.75, 'intensity':.59},
    'ember': {'hue':16, 'saturation':.83, 'intensity':.58},
    'frost': {'hue':199, 'saturation':.57, 'intensity':.74},
  },
  'integration_status': 'Asset only: Bevy spawn, per-species tint, animation graph/controller and voxel collision not implemented in this package.',
}
(OUT / 'slime.collider.json').write_text(json.dumps(config, ensure_ascii=False, indent=2)+'\n', encoding='utf8')
print('Generated', OUT/'slime.glb', len(glb), 'bytes', len(meshes), 'meshes', len(animations), 'animation clips')
