#!/usr/bin/env python3
"""Generate Asteria's reference-driven pink candy slime.

This version intentionally avoids noise textures. The visual richness comes from
explicit horizontal voxel layers, several authored pink material zones, normal
PBR lighting, and a stepped opaque white reflection patch.
"""
from __future__ import annotations
import json, math, struct
from pathlib import Path

OUT=Path(__file__).resolve().parent
CELL=.075
NX,NY,NZ=19,13,17
CX=(NX-1)/2
CZ=(NZ-1)/2
BODY_W=NX*CELL
BODY_H=NY*CELL
BODY_D=NZ*CELL
BODY_HALF_H=BODY_H/2
LAYERS=((7,6),(8,7),(9,7),(9,8),(9,8),(9,8),(9,8),(8,7),(8,7),(7,6),(6,5),(5,4),(3,3))
binary=bytearray();views=[];accessors=[];meshes=[];nodes=[];animations=[]

def store(data,target=None):
    binary.extend(b'\0'*(-len(binary)%4));off=len(binary);binary.extend(data)
    e={'buffer':0,'byteOffset':off,'byteLength':len(data)}
    if target is not None:e['target']=target
    views.append(e);return len(views)-1

def accessor(values,kind='VEC3',component=5126,target=None,bounds=False):
    width={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[kind]
    fmt={5126:'f',5123:'H'}[component]
    e={'bufferView':store(struct.pack('<'+fmt*len(values),*values),target),'componentType':component,'count':len(values)//width,'type':kind}
    if bounds:
        rows=[values[i:i+width] for i in range(0,len(values),width)]
        e['min']=[float(min(r[j] for r in rows)) for j in range(width)]
        e['max']=[float(max(r[j] for r in rows)) for j in range(width)]
    accessors.append(e);return len(accessors)-1

def occupied(ix,iy,iz):
    rx,rz=LAYERS[iy];x=ix-CX;z=iz-CZ
    return (abs(x)/(rx+.48))**3.25+(abs(z)/(rz+.48))**3.25<=1
voxels={(x,y,z)for y in range(NY)for z in range(NZ)for x in range(NX)if occupied(x,y,z)}
FACES=(
 ((1,0,0),((1,0,0),(1,0,1),(1,1,1),(1,1,0))),((-1,0,0),((0,0,1),(0,0,0),(0,1,0),(0,1,1))),
 ((0,1,0),((0,1,0),(1,1,0),(1,1,1),(0,1,1))),((0,-1,0),((0,0,1),(1,0,1),(1,0,0),(0,0,0))),
 ((0,0,1),((1,0,1),(0,0,1),(0,1,1),(1,1,1))),((0,0,-1),((0,0,0),(1,0,0),(1,1,0),(0,1,0))))
MATERIALS=(
 ('SlimeCenter',[.9734,.4910,.5906,1],.42,[.020,.007,.010]),
 ('SlimeMid',[.9823,.4233,.5210,1],.40,[.018,.005,.008]),
 ('SlimeOuter',[.9387,.1500,.2346,1],.38,[.012,.002,.004]),
 ('SlimeBottom',[.9734,.2195,.3140,1],.40,[.015,.003,.005]),
 ('SlimeTop',[.9823,.3325,.4342,1],.36,[.020,.004,.006]),
 ('SlimeLight',[.9911,.8879,.9473,1],.30,[.050,.035,.045]),
 ('SlimeWhite',[1,1,1,1],.24,[.090,.090,.090]),
 ('SlimeEye',[.0976,.0168,.0222,1],1,[0,0,0]),
 ('SlimeEyeHighlight',[1,1,1,1],1,[0,0,0]),
 ('SlimeBlush',[.9823,.8632,.9216,1],1,[0,0,0]),
 ('SlimeMouth',[.115,.026,.034,1],1,[0,0,0]))
materials=[]
for name,color,roughness,emissive in MATERIALS:
    m={'name':name,'pbrMetallicRoughness':{'baseColorFactor':color,'metallicFactor':0,'roughnessFactor':roughness},'doubleSided':False}
    if any(emissive):m['emissiveFactor']=emissive
    materials.append(m)

def mat(ix,iy,iz,n):
    gx=ix-CX;gz=iz-CZ;rx,rz=LAYERS[iy]
    edge=abs(gx)>=rx-1 or abs(gz)>=rz-1
    if n==(0,0,-1):
        bright=(-5<=gx<=-3 and 8<=iy<=10) or (-4<=gx<=-2 and iy==9)
        soft=(-6<=gx<=-2 and 7<=iy<=10) or (-5<=gx<=-1 and 8<=iy<=9) or (-7<=gx<=-5 and 6<=iy<=8)
        if bright:return 6
        if soft:return 5
        if iy<=1:return 3
        if edge:return 2 if gx>0 else 1
        if iy>=9:return 4
        if abs(gx)<=6 and 3<=iy<=8:return 0
        return 1
    if n==(0,1,0):return 5 if iy>=8 else 4
    if n==(0,-1,0):return 3
    if n==(1,0,0):return 2
    if n==(-1,0,0):return 4 if iy>=6 else 1
    if n==(0,0,1):return 2 if edge else 1
    return 1

def bucket():return [[],[],[],[],[]]
UV=((0,1),(1,1),(1,0),(0,0))
def quad(b,pts,n):
    p,no,u,c,i=b;o=len(p)//3
    for j,pt in enumerate(pts):p.extend(pt);no.extend(n);u.extend(UV[j]);c.extend((1,1,1,1))
    i.extend((o,o+2,o+1,o,o+3,o+2))
def primitive(b,m):
    p,n,u,c,i=b
    if not i:return None
    return {'attributes':{'POSITION':accessor(p,bounds=True,target=34962),'NORMAL':accessor(n,target=34962),'TEXCOORD_0':accessor(u,kind='VEC2',target=34962),'COLOR_0':accessor(c,kind='VEC4',target=34962)},'indices':accessor(i,kind='SCALAR',component=5123,target=34963),'material':m,'mode':4}

bs=[bucket()for _ in range(7)]
for ix,iy,iz in sorted(voxels,key=lambda v:(v[1],v[2],v[0])):
    for n,corners in FACES:
        if (ix+n[0],iy+n[1],iz+n[2]) in voxels:continue
        pts=[((ix+cx-CX-.5)*CELL,(iy+cy)*CELL-BODY_HALF_H,(iz+cz-CZ-.5)*CELL)for cx,cy,cz in corners]
        quad(bs[mat(ix,iy,iz,n)],pts,n)
meshes.append({'name':'layered_candy_body','primitives':[p for p in (primitive(b,i)for i,b in enumerate(bs))if p]})
body_mesh=len(meshes)-1

def rect_mesh(name,rects,material_index):
    b=bucket();z=-BODY_D*.5-.007
    for x0,y0,x1,y1 in rects:quad(b,((x1,y0,z),(x0,y0,z),(x0,y1,z),(x1,y1,z)),(0,0,-1))
    meshes.append({'name':name,'primitives':[primitive(b,material_index)]});return len(meshes)-1
eye_l=rect_mesh('EyeLeft',[(-.47,.08,-.22,.16),(-.43,.03,-.26,.08),(-.39,-.005,-.30,.03)],7)
eye_r=rect_mesh('EyeRight',[(.22,.08,.47,.16),(.26,.03,.43,.08),(.30,-.005,.39,.03)],7)
eye_hi=rect_mesh('EyeHighlights',[(-.44,.115,-.37,.165),(.25,.115,.32,.165)],8)
cheeks=rect_mesh('Cheeks',[(-.58,-.075,-.50,-.015),(.50,-.075,.58,-.015)],9)
mouth=rect_mesh('Mouth',[(-.18,-.19,-.155,-.12),(-.155,-.215,-.115,-.19),(-.115,-.235,-.045,-.215),(-.045,-.215,-.02,-.17),(-.02,-.195,.005,-.145),(.005,-.215,.03,-.17),(.03,-.235,.10,-.215),(.10,-.215,.145,-.19),(.145,-.19,.17,-.12)],10)

def bubble_cells(n):
    c=(n-1)*.5;r=n*.57
    return {(x,y,z)for y in range(n)for z in range(n)for x in range(n)if (abs(x-c)**3.4+abs(y-c)**3.4+abs(z-c)**3.4)**(1/3.4)<=r}
def bubble(name,n,scale):
    cells=bubble_cells(n);bb=[bucket(),bucket(),bucket()];cell=scale/n;h=scale*.5
    for ix,iy,iz in sorted(cells,key=lambda v:(v[1],v[2],v[0])):
        for no,corners in FACES:
            if (ix+no[0],iy+no[1],iz+no[2]) in cells:continue
            pts=[(-h+(ix+cx)*cell,-h+(iy+cy)*cell,-h+(iz+cz)*cell)for cx,cy,cz in corners]
            white=no==(0,0,-1) and ix<=n//2 and iy>=n//2
            pale=(not white) and no==(0,1,0)
            quad(bb[2 if white else (1 if pale else 0)],pts,no)
    meshes.append({'name':name,'primitives':[p for p in (primitive(bb[0],1),primitive(bb[1],5),primitive(bb[2],6))if p]});return len(meshes)-1
large_mesh=bubble('large_float_gel_cube',5,.26);small_mesh=bubble('small_float_gel_cube',3,.14)

def node(name,mesh=None,children=None,translation=None,extras=None):
    e={'name':name}
    if mesh is not None:e['mesh']=mesh
    if children is not None:e['children']=children
    if translation is not None:e['translation']=translation
    if extras is not None:e['extras']=extras
    nodes.append(e);return len(nodes)-1
root=node('SlimeRoot',children=[],extras={'asteria_asset':'creature/slime_candy','unit':'meters','forward':'-Z','collider':{'shape':'aabb','size':[.78,.84,.78],'offset':[0,.42,0]},'collider_is_animated':False})
visual=node('Visual',children=[]);body=node('BodyPivot',children=[],translation=[0,BODY_HALF_H,0]);shell=node('Shell',mesh=body_mesh)
face=[node('EyeLeft',mesh=eye_l),node('EyeRight',mesh=eye_r),node('EyeHighlights',mesh=eye_hi),node('Cheeks',mesh=cheeks),node('Mouth',mesh=mouth)]
LB=[.60,1.05,-.02];SB=[.84,.84,-.04];large=node('FloatCubeLarge',mesh=large_mesh,translation=LB);small=node('FloatCubeSmall',mesh=small_mesh,translation=SB)
nodes[body]['children']=[shell,*face];nodes[visual]['children']=[body,large,small]
hit=node('Hitbox_AABB',translation=[0,.42,0],extras={'asteria_collider':{'shape':'aabb','size':[.78,.84,.78],'solid':True,'targetable':True},'debug_display':False});nodes[root]['children']=[visual,hit]

def tracks(name,times,scales,center=None,phase=0):
    center=center if center is not None else [BODY_HALF_H*s[1]for s in scales];ta=accessor(times,kind='SCALAR',bounds=True);chs=[];samps=[]
    def add(target,path,vals,kind):
        out=accessor([v for row in vals for v in row],kind=kind);samps.append({'input':ta,'output':out,'interpolation':'LINEAR'});chs.append({'sampler':len(samps)-1,'target':{'node':target,'path':path}})
    add(body,'scale',scales,'VEC3');add(body,'translation',[[0,y,0]for y in center],'VEC3');den=max(times[-1],1e-6);lp=[];sp=[]
    for t in times:
        a=t/den*math.tau+phase;lp.append([LB[0]+.014*math.sin(a),LB[1]+.030*math.sin(a),LB[2]]);sp.append([SB[0]+.010*math.sin(a+1.3),SB[1]+.022*math.sin(a+1.3),SB[2]])
    add(large,'translation',lp,'VEC3');add(small,'translation',sp,'VEC3');animations.append({'name':name,'channels':chs,'samplers':samps,'extras':{'loop_recommended':name in('Idle','Airborne')}})
tracks('Idle',[0,.5,1,1.5,2],[[1,1,1],[1.014,.980,1.014],[1,1,1],[.990,1.018,.990],[1,1,1]])
tracks('Anticipate',[0,.07,.17,.24],[[1,1,1],[1.08,.86,1.08],[1.11,.79,1.11],[1.08,.85,1.08]],phase=.4)
tracks('Airborne',[0,.12,.35,.55,.72],[[1.08,.85,1.08],[.93,1.12,.93],[.96,1.07,.96],[.98,1.04,.98],[1,1,1]],phase=.8)
tracks('Land',[0,.045,.12,.20,.34],[[.98,1.04,.98],[1.14,.78,1.14],[1.10,.84,1.10],[.98,1.035,.98],[1,1,1]],phase=1.1)
tracks('Hurt',[0,.085,.15,.24,.38],[[1,1,1],[1.08,.90,1.08],[.95,1.06,.95],[1.02,.98,1.02],[1,1,1]],phase=1.8)
tracks('Death',[0,.12,.31,.55,.75],[[1,1,1],[1.10,.84,1.10],[1.16,.65,1.16],[1.10,.20,1.10],[.001,.001,.001]],center=[BODY_HALF_H,.42,.31,.10,.0005],phase=2.2)

scene={'asset':{'version':'2.0','generator':'Asteria reference-layered candy slime v2'},'scene':0,'scenes':[{'name':'SlimeCandy','nodes':[root]}],'nodes':nodes,'meshes':meshes,'materials':materials,'animations':animations,'bufferViews':views,'accessors':accessors,'buffers':[{'byteLength':len(binary)}],'extras':{'asset_id':'asteria:slime_candy','collision_source':'slime_candy.collider.json','voxel_resolution':[NX,NY,NZ],'occupied_voxels':len(voxels),'notes':'Reference-driven opaque candy slime with explicit layers, broad inner-light zones, PBR shell, stepped white reflection and two floating gel cubes.'}}
j=json.dumps(scene,separators=(',',':')).encode();j+=b' '*(-len(j)%4);bb=bytes(binary)+b'\0'*(-len(binary)%4)
glb=struct.pack('<4sII',b'glTF',2,12+8+len(j)+8+len(bb))+struct.pack('<I4s',len(j),b'JSON')+j+struct.pack('<I4s',len(bb),b'BIN\0')+bb
(OUT/'slime_candy.glb').write_bytes(glb)
print(f'Generated {OUT/"slime_candy.glb"}: {len(voxels)} body voxels')
