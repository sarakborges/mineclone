#!/usr/bin/env python3
"""Generate Asteria's candy-pink kawaii voxel slime.

The visual is self-contained in the GLB: opaque/unlit pink materials, stepped
voxel highlights, geometric sleepy eyes/blush/mouth, and two floating gel cubes.
No shell transparency or texture sampling is involved.
"""
from __future__ import annotations
import json, math, struct
from pathlib import Path

OUT = Path(__file__).resolve().parent
binary = bytearray(); views=[]; accessors=[]; meshes=[]; nodes=[]; animations=[]
CELL=.080; NX,NY,NZ=18,13,17
BODY_WIDTH=NX*CELL; BODY_HEIGHT=NY*CELL; BODY_DEPTH=NZ*CELL; BODY_HALF_HEIGHT=BODY_HEIGHT*.5

def store(data,target=None):
    binary.extend(b'\0'*(-len(binary)%4)); off=len(binary); binary.extend(data)
    e={'buffer':0,'byteOffset':off,'byteLength':len(data)}
    if target is not None:e['target']=target
    views.append(e); return len(views)-1

def accessor(values,kind='VEC3',component=5126,target=None,bounds=False):
    width={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[kind]; assert len(values)%width==0
    fmt={5126:'f',5123:'H'}[component]; payload=struct.pack('<'+fmt*len(values),*values)
    e={'bufferView':store(payload,target),'componentType':component,'count':len(values)//width,'type':kind}
    if bounds:
        rows=[values[i:i+width] for i in range(0,len(values),width)]
        e['min']=[float(min(r[j] for r in rows)) for j in range(width)]
        e['max']=[float(max(r[j] for r in rows)) for j in range(width)]
    accessors.append(e); return len(accessors)-1

def profile(t):
    keys=((0,.78),(.05,.88),(.14,.96),(.30,1),(.48,.985),(.63,.92),(.76,.78),(.86,.61),(.93,.43),(.975,.30),(1,.23))
    for (at,ar),(bt,br) in zip(keys,keys[1:]):
        if t<=bt:
            u=max(0,min(1,(t-at)/(bt-at))); u=u*u*(3-2*u); return ar+(br-ar)*u
    return keys[-1][1]

def occupied(ix,iy,iz):
    x=(ix+.5)*CELL-BODY_WIDTH*.5; y=(iy+.5)*CELL-BODY_HEIGHT*.5; z=(iz+.5)*CELL-BODY_DEPTH*.5
    r=profile((y+BODY_HALF_HEIGHT)/BODY_HEIGHT); rx=BODY_WIDTH*.5*r; rz=BODY_DEPTH*.5*r
    return rx>0 and rz>0 and abs(x/rx)**2.62+abs(z/rz)**2.62<=1

voxels={(x,y,z) for y in range(NY) for z in range(NZ) for x in range(NX) if occupied(x,y,z)}
FACE_SPECS=[
 ((1,0,0),((1,0,0),(1,0,1),(1,1,1),(1,1,0))),((-1,0,0),((0,0,1),(0,0,0),(0,1,0),(0,1,1))),
 ((0,1,0),((0,1,0),(1,1,0),(1,1,1),(0,1,1))),((0,-1,0),((0,0,1),(1,0,1),(1,0,0),(0,0,0))),
 ((0,0,1),((1,0,1),(0,0,1),(0,1,1),(1,1,1))),((0,0,-1),((0,0,0),(1,0,0),(1,1,0),(0,1,0)))]

def outer_shade(px,py,pz,w=BODY_WIDTH,h=BODY_HEIGHT,d=BODY_DEPTH):
    nx=px/(w*.5); ny=py/(h*.5); nz=pz/(d*.5)
    length=max((nx*nx+ny*ny+nz*nz)**.5,1e-6); nx/=length;ny/=length;nz/=length
    lx,ly,lz=-.50,.73,-.47; light_len=(lx*lx+ly*ly+lz*lz)**.5; lx/=light_len;ly/=light_len;lz/=light_len
    half_lambert=((nx*lx+ny*ly+nz*lz)+1)*.5
    return .84+.16*max(0,min(1,half_lambert))

def highlight_class(ix,iy,normal):
    if normal!=(0,0,-1): return 0
    soft=(3<=ix<=6 and 7<=iy<=10) or (5<=ix<=7 and 7<=iy<=9) or (3<=ix<=5 and 6<=iy<=8)
    bright=(4<=ix<=5 and 8<=iy<=10) or (5<=ix<=6 and 8<=iy<=9)
    return 2 if bright else (1 if soft else 0)

def bucket(): return [[],[],[],[],[]]

def append_quad(bucket_data,points,normal,shade):
    positions,normals,uvs,colors,indices=bucket_data; offset=len(positions)//3
    for i,point in enumerate(points):
        positions.extend(point); normals.extend(normal)
        uvs.extend(((0,1),(1,1),(1,0),(0,0))[i]); colors.extend((shade,shade,shade,1))
    indices.extend((offset,offset+2,offset+1,offset,offset+3,offset+2))

def primitive(bucket_data,material_index):
    positions,normals,uvs,colors,indices=bucket_data
    if not indices:return None
    return {'attributes':{
        'POSITION':accessor(positions,bounds=True,target=34962),
        'NORMAL':accessor(normals,target=34962),
        'TEXCOORD_0':accessor(uvs,kind='VEC2',target=34962),
        'COLOR_0':accessor(colors,kind='VEC4',target=34962)},
        'indices':accessor(indices,kind='SCALAR',component=5123,target=34963),
        'material':material_index,'mode':4}

def make_body():
    buckets=[bucket(),bucket(),bucket()]
    for ix,iy,iz in sorted(voxels,key=lambda value:(value[1],value[2],value[0])):
        for normal,corners in FACE_SPECS:
            if (ix+normal[0],iy+normal[1],iz+normal[2]) in voxels: continue
            points=[]; shades=[]
            for cx,cy,cz in corners:
                px=-BODY_WIDTH*.5+(ix+cx)*CELL; py=-BODY_HEIGHT*.5+(iy+cy)*CELL; pz=-BODY_DEPTH*.5+(iz+cz)*CELL
                points.append((px,py,pz)); shades.append(outer_shade(px,py,pz))
            append_quad(buckets[highlight_class(ix,iy,normal)],points,normal,sum(shades)/4)
    meshes.append({'name':'candy_gumdrop_body','primitives':[p for p in (
        primitive(buckets[0],0),primitive(buckets[1],1),primitive(buckets[2],2)) if p]})
    return len(meshes)-1

def make_rect_mesh(name,rects,material_index):
    positions=[]; normals=[]; uvs=[]; colors=[]; indices=[]; z=-BODY_DEPTH*.5-.006
    for x0,y0,x1,y1 in rects:
        offset=len(positions)//3
        points=((x1,y0,z),(x0,y0,z),(x0,y1,z),(x1,y1,z))
        for i,point in enumerate(points):
            positions.extend(point); normals.extend((0,0,-1))
            uvs.extend(((0,1),(1,1),(1,0),(0,0))[i]); colors.extend((1,1,1,1))
        indices.extend((offset,offset+1,offset+2,offset,offset+2,offset+3))
    attrs={'POSITION':accessor(positions,bounds=True,target=34962),'NORMAL':accessor(normals,target=34962),'TEXCOORD_0':accessor(uvs,kind='VEC2',target=34962),'COLOR_0':accessor(colors,kind='VEC4',target=34962)}
    meshes.append({'name':name,'primitives':[{'attributes':attrs,'indices':accessor(indices,kind='SCALAR',component=5123,target=34963),'material':material_index,'mode':4}]})
    return len(meshes)-1

def rounded_cube_voxels(n):
    center=(n-1)*.5; radius=n*.56; result=set()
    for y in range(n):
        for z in range(n):
            for x in range(n):
                dx,dy,dz=abs(x-center),abs(y-center),abs(z-center)
                if (dx**3.2+dy**3.2+dz**3.2)**(1/3.2)<=radius: result.add((x,y,z))
    return result

def make_bubble(name,n,scale):
    cells=rounded_cube_voxels(n); buckets=[bucket(),bucket()]; cell=scale/n; half=scale*.5
    for ix,iy,iz in sorted(cells,key=lambda value:(value[1],value[2],value[0])):
        for normal,corners in FACE_SPECS:
            if (ix+normal[0],iy+normal[1],iz+normal[2]) in cells: continue
            points=[]; shades=[]
            for cx,cy,cz in corners:
                px=-half+(ix+cx)*cell;py=-half+(iy+cy)*cell;pz=-half+(iz+cz)*cell
                points.append((px,py,pz));shades.append(outer_shade(px,py,pz,scale,scale,scale))
            glint=normal==(0,0,-1) and ix<=max(1,n//2-1) and iy>=n//2
            append_quad(buckets[1 if glint else 0],points,normal,sum(shades)/4)
    meshes.append({'name':name,'primitives':[p for p in (primitive(buckets[0],0),primitive(buckets[1],2)) if p]})
    return len(meshes)-1

def material(name,rgba):
    return {'name':name,'pbrMetallicRoughness':{'baseColorFactor':rgba,'metallicFactor':0,'roughnessFactor':1},'doubleSided':False,'extensions':{'KHR_materials_unlit':{}}}

materials=[
    material('SlimeShell',[1,.64,.77,1]),
    material('SlimeHighlightSoft',[1,.86,.92,1]),
    material('SlimeHighlightBright',[1,.99,1,1]),
    material('SlimeEye',[.27,.09,.13,1]),
    material('SlimeEyeHighlight',[1,.99,1,1]),
    material('SlimeBlush',[1,.86,.91,1]),
    material('SlimeMouth',[.43,.16,.22,1]),
]
body_mesh=make_body()
eye_l=make_rect_mesh('EyeLeft',[(-.47,.13,-.22,.23),(-.43,.08,-.26,.13)],3)
eye_r=make_rect_mesh('EyeRight',[(.22,.13,.47,.23),(.26,.08,.43,.13)],3)
eye_hi=make_rect_mesh('EyeHighlights',[(-.43,.18,-.36,.24),(.26,.18,.33,.24)],4)
blush=make_rect_mesh('Cheeks',[(-.58,-.01,-.49,.07),(.49,-.01,.58,.07)],5)
mouth=make_rect_mesh('Mouth',[(-.16,-.14,-.13,-.06),(-.13,-.17,-.07,-.14),(-.07,-.19,-.01,-.17),(-.01,-.16,.02,-.11),(.02,-.19,.08,-.17),(.08,-.17,.14,-.14),(.14,-.14,.17,-.06)],6)
large_bubble_mesh=make_bubble('large_float_gel_cube',5,.28)
small_bubble_mesh=make_bubble('small_float_gel_cube',3,.15)

def node(name,mesh=None,children=None,translation=None,extras=None):
    entry={'name':name}
    if mesh is not None:entry['mesh']=mesh
    if children is not None:entry['children']=children
    if translation is not None:entry['translation']=translation
    if extras is not None:entry['extras']=extras
    nodes.append(entry);return len(nodes)-1

root=node('SlimeRoot',children=[],extras={'asteria_asset':'creature/slime_candy','unit':'meters','forward':'-Z','collider':{'shape':'aabb','size':[.78,.84,.78],'offset':[0,.42,0]},'collider_is_animated':False})
visual=node('Visual',children=[])
body=node('BodyPivot',children=[],translation=[0,BODY_HALF_HEIGHT,0])
shell=node('Shell',mesh=body_mesh)
face_nodes=[node('EyeLeft',mesh=eye_l),node('EyeRight',mesh=eye_r),node('EyeHighlights',mesh=eye_hi),node('Cheeks',mesh=blush),node('Mouth',mesh=mouth)]
large_base=[.58,1.03,-.02];small_base=[.83,.83,-.05]
large=node('FloatCubeLarge',mesh=large_bubble_mesh,translation=large_base)
small=node('FloatCubeSmall',mesh=small_bubble_mesh,translation=small_base)
nodes[body]['children']=[shell,*face_nodes];nodes[visual]['children']=[body,large,small]
collider=node('Hitbox_AABB',translation=[0,.42,0],extras={'asteria_collider':{'shape':'aabb','size':[.78,.84,.78],'solid':True,'targetable':True},'debug_display':False})
nodes[root]['children']=[visual,collider]

def tracks(clip,times,scales,center=None,phase=0):
    center=center if center is not None else [BODY_HALF_HEIGHT*scale[1] for scale in scales]
    time_accessor=accessor(times,kind='SCALAR',bounds=True);channels=[];samplers=[]
    def add(target,path,values,kind):
        output=accessor([value for row in values for value in row],kind=kind)
        samplers.append({'input':time_accessor,'output':output,'interpolation':'LINEAR'})
        channels.append({'sampler':len(samplers)-1,'target':{'node':target,'path':path}})
    add(body,'scale',scales,'VEC3');add(body,'translation',[[0,y,0] for y in center],'VEC3')
    denominator=max(times[-1],1e-6);large_positions=[];small_positions=[]
    for time in times:
        angle=time/denominator*math.tau+phase
        large_positions.append([large_base[0]+.018*math.sin(angle),large_base[1]+.035*math.sin(angle),large_base[2]])
        small_positions.append([small_base[0]+.012*math.sin(angle+1.3),small_base[1]+.026*math.sin(angle+1.3),small_base[2]])
    add(large,'translation',large_positions,'VEC3');add(small,'translation',small_positions,'VEC3')
    animations.append({'name':clip,'channels':channels,'samplers':samplers,'extras':{'loop_recommended':clip in ('Idle','Airborne')}})

tracks('Idle',[0,.5,1,1.5,2],[[1,1,1],[1.018,.974,1.018],[1,1,1],[.988,1.021,.988],[1,1,1]])
tracks('Anticipate',[0,.07,.17,.24],[[1,1,1],[1.09,.845,1.09],[1.125,.76,1.125],[1.09,.83,1.09]],phase=.4)
tracks('Airborne',[0,.12,.35,.55,.72],[[1.09,.83,1.09],[.91,1.15,.91],[.96,1.085,.96],[.97,1.06,.97],[1,1,1]],phase=.8)
tracks('Land',[0,.045,.12,.20,.34],[[.97,1.055,.97],[1.17,.74,1.17],[1.12,.805,1.12],[.975,1.047,.975],[1,1,1]],phase=1.1)
tracks('Hurt',[0,.085,.15,.24,.38],[[1,1,1],[1.1,.88,1.1],[.94,1.08,.94],[1.025,.968,1.025],[1,1,1]],phase=1.8)
tracks('Death',[0,.12,.31,.55,.75],[[1,1,1],[1.13,.8,1.13],[1.2,.60,1.2],[1.12,.19,1.12],[.001,.001,.001]],center=[BODY_HALF_HEIGHT,.42,.31,.10,.0005],phase=2.2)

scene={'asset':{'version':'2.0','generator':'Asteria candy kawaii voxel slime v1'},'scene':0,'scenes':[{'name':'SlimeCandy','nodes':[root]}],'extensionsUsed':['KHR_materials_unlit'],'nodes':nodes,'meshes':meshes,'materials':materials,'animations':animations,'bufferViews':views,'accessors':accessors,'buffers':[{'byteLength':len(binary)}],'extras':{'asset_id':'asteria:slime_candy','color_materials':[m['name'] for m in materials],'collision_source':'slime_candy.collider.json','voxel_resolution':[NX,NY,NZ],'occupied_voxels':len(voxels),'notes':'Squat pastel-pink gumdrop slime based on the supplied reference, with sleepy geometric face, stepped opaque highlights and two floating gel cubes.'}}
json_chunk=json.dumps(scene,separators=(',',':'),ensure_ascii=False).encode();json_chunk+=b' '*(-len(json_chunk)%4)
bin_chunk=bytes(binary)+b'\0'*(-len(binary)%4)
glb=struct.pack('<4sII',b'glTF',2,12+8+len(json_chunk)+8+len(bin_chunk))+struct.pack('<I4s',len(json_chunk),b'JSON')+json_chunk+struct.pack('<I4s',len(bin_chunk),b'BIN\0')+bin_chunk
(OUT/'slime_candy.glb').write_bytes(glb)
print(f'Generated {OUT/"slime_candy.glb"}: {len(voxels)} body voxels')
