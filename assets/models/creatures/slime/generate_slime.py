#!/usr/bin/env python3
"""Generate the preserved legacy cubic slime with smooth fixed authored colors.

The original cubic silhouette, animated core and clips are preserved. Shell
depth is encoded directly in linear RGB COLOR_0 values: no runtime shell/core
tint and no shell texture. Internal tessellation exists only to interpolate a
smooth color field; it does not create visible material borders.
"""
from __future__ import annotations
import json, math, struct
from pathlib import Path

OUT=Path(__file__).resolve().parent
W,H,D=.96,.90,.96
GRID=10
binary=bytearray();views=[];accessors=[];meshes=[];nodes=[];animations=[]

def linear(c:float)->float:
    return c/12.92 if c<=.04045 else ((c+.055)/1.055)**2.4

def mix(a:float,b:float,t:float)->float:
    t=max(0.0,min(1.0,t));return a+(b-a)*t

def mix3(a,b,t):
    return tuple(mix(a[i],b[i],t) for i in range(3))

MAIN=(.38,.64,.52)
CENTER=(.49,.72,.60)
EDGE=(.30,.53,.42)
BOTTOM=(.27,.49,.38)
TOP=(.44,.68,.56)
LIGHT=(.58,.78,.68)

def store(data:bytes,target:int|None=None)->int:
    binary.extend(b'\0'*(-len(binary)%4));off=len(binary);binary.extend(data)
    entry={'buffer':0,'byteOffset':off,'byteLength':len(data)}
    if target is not None:entry['target']=target
    views.append(entry);return len(views)-1

def accessor(values,kind='VEC3',component=5126,target=None,bounds=False):
    width={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[kind]
    fmt={5126:'f',5123:'H'}[component]
    entry={'bufferView':store(struct.pack('<'+fmt*len(values),*values),target),'componentType':component,'count':len(values)//width,'type':kind}
    if bounds:
        rows=[values[i:i+width] for i in range(0,len(values),width)]
        entry['min']=[float(min(row[j] for row in rows)) for j in range(width)]
        entry['max']=[float(max(row[j] for row in rows)) for j in range(width)]
    accessors.append(entry);return len(accessors)-1

FACES=(
    ((1,0,0),(0,0,-1),(0,1,0)),
    ((-1,0,0),(0,0,1),(0,1,0)),
    ((0,1,0),(1,0,0),(0,0,-1)),
    ((0,-1,0),(1,0,0),(0,0,1)),
    ((0,0,1),(1,0,0),(0,1,0)),
    ((0,0,-1),(-1,0,0),(0,1,0)),
)

def shell_color(face:int,u:float,v:float):
    color=MAIN
    if face==5:
        center=max(0.0,1.0-(abs(u)/.92)**2-((v+.06)/1.0)**2)
        color=mix3(color,CENTER,center*.72)
        edge=max(abs(u),abs(v))
        color=mix3(color,EDGE,max(0.0,(edge-.60)/.40)*.48)
        color=mix3(color,BOTTOM,max(0.0,(-v-.48)/.52)*.34)
        color=mix3(color,TOP,max(0.0,(v-.50)/.50)*.32)
        dx=(u+.43)/.42;dy=(v-.47)/.40
        glow=max(0.0,1.0-(dx*dx+dy*dy))
        color=mix3(color,LIGHT,glow*.52)
    elif face==0:
        color=mix3(MAIN,EDGE,.44)
        color=mix3(color,BOTTOM,max(0.0,(-v-.25)/.75)*.22)
    elif face==1:
        color=mix3(MAIN,TOP,.22)
        color=mix3(color,LIGHT,max(0.0,(v-.20)/.80)*.18)
    elif face==2:
        color=mix3(TOP,LIGHT,.20)
    elif face==3:
        color=mix3(BOTTOM,EDGE,.12)
    elif face==4:
        color=mix3(MAIN,EDGE,.18)
    return tuple(linear(channel) for channel in color)

positions=[];normals=[];uvs=[];colors=[];indices=[]
for face_index,(normal,horizontal,vertical) in enumerate(FACES):
    face_half=W/2 if normal[0] else H/2 if normal[1] else D/2
    horizontal_extent=abs(horizontal[0])*W/2+abs(horizontal[1])*H/2+abs(horizontal[2])*D/2
    vertical_extent=abs(vertical[0])*W/2+abs(vertical[1])*H/2+abs(vertical[2])*D/2
    base=len(positions)//3
    for iy in range(GRID+1):
        v=-1+2*iy/GRID
        for ix in range(GRID+1):
            u=-1+2*ix/GRID
            positions.extend((
                normal[0]*face_half+horizontal[0]*horizontal_extent*u+vertical[0]*vertical_extent*v,
                normal[1]*face_half+horizontal[1]*horizontal_extent*u+vertical[1]*vertical_extent*v,
                normal[2]*face_half+horizontal[2]*horizontal_extent*u+vertical[2]*vertical_extent*v,
            ))
            normals.extend(normal);uvs.extend((ix/GRID,1-iy/GRID))
            colors.extend((*shell_color(face_index,u,v),1.0))
    for iy in range(GRID):
        for ix in range(GRID):
            a=base+iy*(GRID+1)+ix;b=a+1;c=a+GRID+2;d=a+GRID+1
            indices.extend((a,b,c,a,c,d))

attrs={
    'POSITION':accessor(positions,bounds=True,target=34962),
    'NORMAL':accessor(normals,target=34962),
    'TEXCOORD_0':accessor(uvs,kind='VEC2',target=34962),
    'COLOR_0':accessor(colors,kind='VEC4',target=34962),
}
meshes.append({'name':'legacy_smooth_fixed_color_shell','primitives':[{
    'attributes':attrs,'indices':accessor(indices,kind='SCALAR',component=5123,target=34963),'material':0,'mode':4,
}]})
shell_mesh=len(meshes)-1

def make_cube(name,extent,material_index):
    pos=[];nrm=[];tex=[];col=[];idx=[];half=[value/2 for value in extent]
    for normal,horizontal,vertical in FACES:
        face_half=half[0] if normal[0] else half[1] if normal[1] else half[2]
        he=abs(horizontal[0])*half[0]+abs(horizontal[1])*half[1]+abs(horizontal[2])*half[2]
        ve=abs(vertical[0])*half[0]+abs(vertical[1])*half[1]+abs(vertical[2])*half[2]
        offset=len(pos)//3
        for vertex,(u,v) in enumerate(((-1,-1),(1,-1),(1,1),(-1,1))):
            pos.extend((normal[0]*face_half+horizontal[0]*he*u+vertical[0]*ve*v,
                        normal[1]*face_half+horizontal[1]*he*u+vertical[1]*ve*v,
                        normal[2]*face_half+horizontal[2]*he*u+vertical[2]*ve*v))
            nrm.extend(normal);tex.extend(((0,1),(1,1),(1,0),(0,0))[vertex]);col.extend((1,1,1,1))
        idx.extend((offset,offset+1,offset+2,offset,offset+2,offset+3))
    mesh={'name':name,'primitives':[{
        'attributes':{'POSITION':accessor(pos,bounds=True,target=34962),'NORMAL':accessor(nrm,target=34962),'TEXCOORD_0':accessor(tex,kind='VEC2',target=34962),'COLOR_0':accessor(col,kind='VEC4',target=34962)},
        'indices':accessor(idx,kind='SCALAR',component=5123,target=34963),'material':material_index,'mode':4,
    }]}
    meshes.append(mesh);return len(meshes)-1

core_mesh=make_cube('square_nucleus',(.58,.62,.58),1)

face_positions=(W/2,-H/2,-.50,-W/2,-H/2,-.50,-W/2,H/2,-.50,W/2,H/2,-.50)
face_normals=(0,0,-1)*4
face_uvs=(0,1,1,1,1,0,0,0)
face_indices=(0,1,2,0,2,3)
meshes.append({'name':'square_pixel_face','primitives':[{
    'attributes':{'POSITION':accessor(face_positions,bounds=True,target=34962),'NORMAL':accessor(face_normals,target=34962),'TEXCOORD_0':accessor(face_uvs,kind='VEC2',target=34962)},
    'indices':accessor(face_indices,kind='SCALAR',component=5123,target=34963),'material':2,'mode':4,
}]})
face_mesh=len(meshes)-1

materials=[
    {'name':'SlimeShell','pbrMetallicRoughness':{'baseColorFactor':[1,1,1,1],'metallicFactor':0,'roughnessFactor':1},'doubleSided':False,'extensions':{'KHR_materials_unlit':{}}},
    {'name':'SlimeCore','pbrMetallicRoughness':{'baseColorFactor':[linear(.14),linear(.40),linear(.30),1],'metallicFactor':0,'roughnessFactor':1},'doubleSided':False,'extensions':{'KHR_materials_unlit':{}}},
    {'name':'SlimeFace','pbrMetallicRoughness':{'baseColorFactor':[1,1,1,1],'metallicFactor':0,'roughnessFactor':1},'doubleSided':False,'alphaMode':'BLEND','extensions':{'KHR_materials_unlit':{}}},
]

def node(name,mesh=None,children=None,translation=None,scale=None,extras=None):
    entry={'name':name}
    if mesh is not None:entry['mesh']=mesh
    if children is not None:entry['children']=children
    if translation is not None:entry['translation']=translation
    if scale is not None:entry['scale']=scale
    if extras is not None:entry['extras']=extras
    nodes.append(entry);return len(nodes)-1

root=node('SlimeRoot',children=[],extras={'asteria_asset':'creature/slime','unit':'meters','forward':'-Z','collider':{'shape':'aabb','size':[.78,.84,.78],'offset':[0,.42,0]},'collider_is_animated':False})
visual=node('Visual',children=[])
body=node('BodyPivot',children=[],translation=[0,.5,0])
inner=node('InnerCore',mesh=core_mesh,translation=[0,-.025,0],scale=[1,1,1])
shell=node('Shell',mesh=shell_mesh)
face=node('Face',mesh=face_mesh,translation=[0,0,0])
nodes[body]['children']=[shell,inner,face];nodes[visual]['children']=[body]
hit=node('Hitbox_AABB',translation=[0,.42,0],extras={'asteria_collider':{'shape':'aabb','size':[.78,.84,.78],'solid':True,'targetable':True},'debug_display':False})
nodes[root]['children']=[visual,hit]

def tracks(name,times,body_scale,center_y=None,core_scale=None,core_angle=None):
    center_y=center_y if center_y is not None else [.5*scale[1] for scale in body_scale]
    core_scale=core_scale if core_scale is not None else [[1,1,1] for _ in times]
    core_angle=core_angle if core_angle is not None else [0 for _ in times]
    time_accessor=accessor(times,kind='SCALAR',bounds=True);channels=[];samplers=[]
    def add(target,path,values,kind):
        output=accessor([value for row in values for value in row],kind=kind)
        samplers.append({'input':time_accessor,'output':output,'interpolation':'LINEAR'})
        channels.append({'sampler':len(samplers)-1,'target':{'node':target,'path':path}})
    add(body,'scale',body_scale,'VEC3');add(body,'translation',[[0,y,0] for y in center_y],'VEC3')
    add(inner,'scale',core_scale,'VEC3');add(inner,'rotation',[[0,math.sin(angle/2),0,math.cos(angle/2)] for angle in core_angle],'VEC4')
    animations.append({'name':name,'channels':channels,'samplers':samplers,'extras':{'loop_recommended':name in ('Idle','Airborne')}})

tracks('Idle',[0,.5,1,1.5,2],[[1,1,1],[1.018,.974,1.018],[1,1,1],[.988,1.021,.988],[1,1,1]],core_scale=[[1,1,1],[1.065,.97,1.065],[1,1,1],[.96,1.04,.96],[1,1,1]],core_angle=[0,.04,0,-.04,0])
tracks('Anticipate',[0,.07,.17,.24],[[1,1,1],[1.09,.845,1.09],[1.125,.76,1.125],[1.09,.83,1.09]],core_scale=[[1,1,1],[1.05,.9,1.05],[1.10,.84,1.10],[1.05,.92,1.05]],core_angle=[0,-.07,-.10,-.04])
tracks('Airborne',[0,.12,.35,.55,.72],[[1.09,.83,1.09],[.91,1.15,.91],[.96,1.085,.96],[.97,1.06,.97],[1,1,1]],core_scale=[[1,1,1],[.95,1.08,.95],[.98,1.035,.98],[1,1,1],[1,1,1]],core_angle=[0,.13,.23,.11,0])
tracks('Land',[0,.045,.12,.20,.34],[[.97,1.055,.97],[1.17,.74,1.17],[1.12,.805,1.12],[.975,1.047,.975],[1,1,1]],core_scale=[[1,1,1],[1.12,.85,1.12],[1.05,.94,1.05],[.98,1.025,.98],[1,1,1]])
tracks('Hurt',[0,.085,.15,.24,.38],[[1,1,1],[1.1,.88,1.1],[.94,1.08,.94],[1.025,.968,1.025],[1,1,1]],core_angle=[0,-.22,.19,-.08,0])
tracks('Death',[0,.12,.31,.55,.75],[[1,1,1],[1.13,.8,1.13],[1.2,.60,1.2],[1.12,.19,1.12],[.001,.001,.001]],center_y=[.5,.4,.3,.095,.0005],core_scale=[[1,1,1],[1.1,.88,1.1],[1.2,.7,1.2],[.95,.3,.95],[.001,.001,.001]],core_angle=[0,.2,.5,.9,1.2])

scene={'asset':{'version':'2.0','generator':'Asteria smooth fixed-color legacy slime v5'},'scene':0,'scenes':[{'name':'Slime','nodes':[root]}],'extensionsUsed':['KHR_materials_unlit'],'nodes':nodes,'meshes':meshes,'materials':materials,'animations':animations,'bufferViews':views,'accessors':accessors,'buffers':[{'byteLength':len(binary)}],'extras':{'asset_id':'asteria:slime_base','color_materials':['SlimeShell','SlimeCore','SlimeFace'],'collision_source':'slime.collider.json','notes':'Preserved cubic legacy slime with smooth fixed RGB vertex-color depth; no runtime shell/core tint or texture. Face winding points outward toward -Z.'}}
json_chunk=json.dumps(scene,separators=(',',':')).encode();json_chunk+=b' '*(-len(json_chunk)%4)
bin_chunk=bytes(binary)+b'\0'*(-len(binary)%4)
glb=struct.pack('<4sII',b'glTF',2,12+8+len(json_chunk)+8+len(bin_chunk))+struct.pack('<I4s',len(json_chunk),b'JSON')+json_chunk+struct.pack('<I4s',len(bin_chunk),b'BIN\0')+bin_chunk
(OUT/'slime.glb').write_bytes(glb)
print(f'Generated {OUT/"slime.glb"}: smooth {GRID}x{GRID} shell grid per face')
