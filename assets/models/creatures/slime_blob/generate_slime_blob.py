#!/usr/bin/env python3
"""Generate the rounded slime with fixed authored green depth colors.

The shape/animations are the original slime_blob. Shell tinting and shell
textures are intentionally not runtime data anymore. Seven opaque unlit shell
materials encode center light, main body, rim, base, top and two highlight
levels directly into the GLB.
"""
from __future__ import annotations
import json,math,struct
from pathlib import Path
OUT=Path(__file__).resolve().parent
BODY_WIDTH=1.20;BODY_HEIGHT=1.00;BODY_DEPTH=1.14
NX,NY,NZ=24,20,22
DX,DY,DZ=BODY_WIDTH/NX,BODY_HEIGHT/NY,BODY_DEPTH/NZ
BODY_HALF_HEIGHT=BODY_HEIGHT*.5
SRGB=(('SlimeShell',(.38,.64,.52)),('SlimeShellCenter',(.52,.74,.63)),('SlimeShellOuter',(.22,.43,.34)),('SlimeShellBottom',(.18,.37,.29)),('SlimeShellTop',(.46,.69,.58)),('SlimeShellLight',(.66,.84,.75)),('SlimeShellBright',(.86,.95,.90)))
def lin(c):return c/12.92 if c<=.04045 else ((c+.055)/1.055)**2.4
binary=bytearray();views=[];accessors=[];meshes=[];nodes=[];animations=[]
def store(data,target=None):
 binary.extend(b'\0'*(-len(binary)%4));o=len(binary);binary.extend(data);e={'buffer':0,'byteOffset':o,'byteLength':len(data)}
 if target is not None:e['target']=target
 views.append(e);return len(views)-1
def acc(values,kind='VEC3',component=5126,target=None,bounds=False):
 w={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[kind];fmt={5126:'f',5123:'H'}[component];e={'bufferView':store(struct.pack('<'+fmt*len(values),*values),target),'componentType':component,'count':len(values)//w,'type':kind}
 if bounds:
  rows=[values[i:i+w]for i in range(0,len(values),w)];e['min']=[float(min(r[j]for r in rows))for j in range(w)];e['max']=[float(max(r[j]for r in rows))for j in range(w)]
 accessors.append(e);return len(accessors)-1
def profile(t):
 k=((0,.66),(.06,.79),(.15,.91),(.28,.995),(.44,1),(.58,.965),(.70,.89),(.80,.76),(.88,.60),(.94,.36),(.975,.14),(1,.02))
 for(a,ar),(b,br)in zip(k,k[1:]):
  if t<=b:u=(t-a)/(b-a);u=u*u*(3-2*u);return ar+(br-ar)*u
 return .02
def occupied(ix,iy,iz):
 x=(ix+.5)*DX-BODY_WIDTH*.5;y=(iy+.5)*DY-BODY_HEIGHT*.5;z=(iz+.5)*DZ-BODY_DEPTH*.5;t=(y+BODY_HALF_HEIGHT)/BODY_HEIGHT;r=profile(t);x-=.032*max(0,(t-.62)/.38)**1.7;rx=BODY_WIDTH*.5*r;rz=BODY_DEPTH*.5*r
 return rx>0 and rz>0 and abs(x/rx)**2.35+abs(z/rz)**2.35<=1
vox={(x,y,z)for y in range(NY)for z in range(NZ)for x in range(NX)if occupied(x,y,z)}
F=(((1,0,0),((1,0,0),(1,0,1),(1,1,1),(1,1,0))),((-1,0,0),((0,0,1),(0,0,0),(0,1,0),(0,1,1))),((0,1,0),((0,1,0),(1,1,0),(1,1,1),(0,1,1))),((0,-1,0),((0,0,1),(1,0,1),(1,0,0),(0,0,0))),((0,0,1),((1,0,1),(0,0,1),(0,1,1),(1,1,1))),((0,0,-1),((0,0,0),(1,0,0),(1,1,0),(0,1,0))))
CX=(NX-1)/2;CZ=(NZ-1)/2
def material_for(ix,iy,iz,n):
 gx=(ix-CX)/(NX*.5);gy=iy/(NY-1);gz=(iz-CZ)/(NZ*.5)
 if n==(0,0,-1):
  if -.58<gx<-.24 and .63<gy<.86:return 6
  if -.72<gx<-.10 and .50<gy<.88:return 5
  if gy<.13:return 3
  if abs(gx)>.72:return 2
  if gy>.78:return 4
  if abs(gx)<.58 and .20<gy<.72:return 1
  return 0
 if n==(0,1,0):return 5 if gy>.72 else 4
 if n==(0,-1,0):return 3
 if n==(1,0,0):return 2
 if n==(-1,0,0):return 4 if gy>.62 else 0
 if n==(0,0,1):return 2 if abs(gx)>.66 or abs(gz)>.66 else 0
 return 0
def bucket():return[[],[],[],[],[]]
UV=((0,1),(1,1),(1,0),(0,0))
def quad(b,pts,n):
 p,no,u,c,i=b;o=len(p)//3
 for j,pt in enumerate(pts):p.extend(pt);no.extend(n);u.extend(UV[j]);c.extend((1,1,1,1))
 i.extend((o,o+2,o+1,o,o+3,o+2))
def prim(b,m):
 p,n,u,c,i=b
 if not i:return None
 return{'attributes':{'POSITION':acc(p,bounds=True,target=34962),'NORMAL':acc(n,target=34962),'TEXCOORD_0':acc(u,kind='VEC2',target=34962),'COLOR_0':acc(c,kind='VEC4',target=34962)},'indices':acc(i,kind='SCALAR',component=5123,target=34963),'material':m,'mode':4}
bs=[bucket()for _ in range(7)]
for ix,iy,iz in sorted(vox,key=lambda v:(v[1],v[2],v[0])):
 for n,corners in F:
  if(ix+n[0],iy+n[1],iz+n[2])in vox:continue
  pts=[(-BODY_WIDTH*.5+(ix+cx)*DX,-BODY_HEIGHT*.5+(iy+cy)*DY,-BODY_DEPTH*.5+(iz+cz)*DZ)for cx,cy,cz in corners];quad(bs[material_for(ix,iy,iz,n)],pts,n)
meshes.append({'name':'rounded_voxel_blob_fixed_color','primitives':[p for p in(prim(b,i)for i,b in enumerate(bs))if p]});shell_mesh=len(meshes)-1
fb=bucket();z=-BODY_DEPTH*.5-.006;w=.86;h=.48;yc=-.075;quad(fb,((w/2,yc-h/2,z),(-w/2,yc-h/2,z),(-w/2,yc+h/2,z),(w/2,yc+h/2,z)),(0,0,-1));fb[4][:]=[0,1,2,0,2,3];meshes.append({'name':'pixel_face_decal','primitives':[prim(fb,7)]});face_mesh=len(meshes)-1
materials=[{'name':n,'pbrMetallicRoughness':{'baseColorFactor':[lin(c[0]),lin(c[1]),lin(c[2]),1],'metallicFactor':0,'roughnessFactor':1},'doubleSided':False,'extensions':{'KHR_materials_unlit':{}}}for n,c in SRGB]
materials.append({'name':'SlimeFace','pbrMetallicRoughness':{'baseColorFactor':[1,1,1,1],'metallicFactor':0,'roughnessFactor':1},'doubleSided':False,'alphaMode':'BLEND','extensions':{'KHR_materials_unlit':{}}})
def node(name,mesh=None,children=None,translation=None,extras=None):
 e={'name':name}
 if mesh is not None:e['mesh']=mesh
 if children is not None:e['children']=children
 if translation is not None:e['translation']=translation
 if extras is not None:e['extras']=extras
 nodes.append(e);return len(nodes)-1
root=node('SlimeRoot',children=[],extras={'asteria_asset':'creature/slime_blob','unit':'meters','forward':'-Z','collider':{'shape':'aabb','size':[.78,.84,.78],'offset':[0,.42,0]},'collider_is_animated':False});visual=node('Visual',children=[]);body=node('BodyPivot',children=[],translation=[0,BODY_HALF_HEIGHT,0]);shell=node('Shell',mesh=shell_mesh);face=node('Face',mesh=face_mesh);nodes[body]['children']=[shell,face];nodes[visual]['children']=[body];hit=node('Hitbox_AABB',translation=[0,.42,0],extras={'asteria_collider':{'shape':'aabb','size':[.78,.84,.78],'solid':True,'targetable':True},'debug_display':False});nodes[root]['children']=[visual,hit]
def track(name,times,scales,center=None):
 center=center if center is not None else[BODY_HALF_HEIGHT*s[1]for s in scales];ta=acc(times,kind='SCALAR',bounds=True);ch=[];sa=[]
 def add(target,path,vals,kind):out=acc([v for row in vals for v in row],kind=kind);sa.append({'input':ta,'output':out,'interpolation':'LINEAR'});ch.append({'sampler':len(sa)-1,'target':{'node':target,'path':path}})
 add(body,'scale',scales,'VEC3');add(body,'translation',[[0,y,0]for y in center],'VEC3');animations.append({'name':name,'channels':ch,'samplers':sa,'extras':{'loop_recommended':name in('Idle','Airborne')}})
track('Idle',[0,.5,1,1.5,2],[[1,1,1],[1.018,.974,1.018],[1,1,1],[.988,1.021,.988],[1,1,1]]);track('Anticipate',[0,.07,.17,.24],[[1,1,1],[1.09,.845,1.09],[1.125,.76,1.125],[1.09,.83,1.09]]);track('Airborne',[0,.12,.35,.55,.72],[[1.09,.83,1.09],[.91,1.15,.91],[.96,1.085,.96],[.97,1.06,.97],[1,1,1]]);track('Land',[0,.045,.12,.20,.34],[[.97,1.055,.97],[1.17,.74,1.17],[1.12,.805,1.12],[.975,1.047,.975],[1,1,1]]);track('Hurt',[0,.085,.15,.24,.38],[[1,1,1],[1.1,.88,1.1],[.94,1.08,.94],[1.025,.968,1.025],[1,1,1]]);track('Death',[0,.12,.31,.55,.75],[[1,1,1],[1.13,.8,1.13],[1.2,.60,1.2],[1.12,.19,1.12],[.001,.001,.001]],center=[BODY_HALF_HEIGHT,.4,.3,.095,.0005])
scene={'asset':{'version':'2.0','generator':'Asteria fixed-color rounded slime v3'},'scene':0,'scenes':[{'name':'SlimeBlob','nodes':[root]}],'extensionsUsed':['KHR_materials_unlit'],'nodes':nodes,'meshes':meshes,'materials':materials,'animations':animations,'bufferViews':views,'accessors':accessors,'buffers':[{'byteLength':len(binary)}],'extras':{'asset_id':'asteria:slime_blob','color_materials':[m['name']for m in materials],'collision_source':'slime_blob.collider.json','voxel_resolution':[NX,NY,NZ],'occupied_voxels':len(vox),'notes':'Original rounded shape with fixed authored green depth zones; face quad winding corrected for -Z front visibility.'}}
j=json.dumps(scene,separators=(',',':')).encode();j+=b' '*(-len(j)%4);bb=bytes(binary)+b'\0'*(-len(binary)%4);glb=struct.pack('<4sII',b'glTF',2,12+8+len(j)+8+len(bb))+struct.pack('<I4s',len(j),b'JSON')+j+struct.pack('<I4s',len(bb),b'BIN\0')+bb;(OUT/'slime_blob.glb').write_bytes(glb)
