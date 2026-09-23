#!/usr/bin/env python3
"""Generate the preserved cubic slime with fixed authored green depth colors.

The original cubic silhouette, core and animations are preserved. The shell is
subdivided only to let fixed color regions create volume; there are no visible
borders and no runtime shell/core tint or texture.
"""
from __future__ import annotations
import json,math,struct
from pathlib import Path
OUT=Path(__file__).resolve().parent
W,H,D=.96,.90,.96
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
FACES=(((1,0,0),(0,0,-1),(0,1,0)),((-1,0,0),(0,0,1),(0,1,0)),((0,1,0),(1,0,0),(0,0,-1)),((0,-1,0),(1,0,0),(0,0,1)),((0,0,1),(1,0,0),(0,1,0)),((0,0,-1),(-1,0,0),(0,1,0)))
def material_for(face,u,v):
 if face==5:
  if -.62<u<-.22 and .35<v<.78:return 6
  if -.78<u<-.04 and .18<v<.82:return 5
  if v<-.72:return 3
  if abs(u)>.78 or abs(v)>.88:return 2
  if v>.62:return 4
  if abs(u)<.66 and -.52<v<.52:return 1
  return 0
 if face==2:return 4
 if face==3:return 3
 if face==0:return 2
 if face==1:return 4 if v>.35 else 0
 if face==4:return 2 if abs(u)>.72 else 0
 return 0
def bucket():return[[],[],[],[],[]]
UV=((0,1),(1,1),(1,0),(0,0))
def quad(b,pts,n,reverse=False):
 p,no,u,c,i=b;o=len(p)//3
 for j,pt in enumerate(pts):p.extend(pt);no.extend(n);u.extend(UV[j]);c.extend((1,1,1,1))
 if reverse:i.extend((o,o+2,o+1,o,o+3,o+2))
 else:i.extend((o,o+1,o+2,o,o+2,o+3))
def prim(b,m):
 p,n,u,c,i=b
 if not i:return None
 return{'attributes':{'POSITION':acc(p,bounds=True,target=34962),'NORMAL':acc(n,target=34962),'TEXCOORD_0':acc(u,kind='VEC2',target=34962),'COLOR_0':acc(c,kind='VEC4',target=34962)},'indices':acc(i,kind='SCALAR',component=5123,target=34963),'material':m,'mode':4}
bs=[bucket()for _ in range(7)];grid=12
for fi,(n,h,v) in enumerate(FACES):
 face_half=W/2 if n[0] else H/2 if n[1] else D/2;he=abs(h[0])*W/2+abs(h[1])*H/2+abs(h[2])*D/2;ve=abs(v[0])*W/2+abs(v[1])*H/2+abs(v[2])*D/2
 for iy in range(grid):
  for ix in range(grid):
   u0=-1+2*ix/grid;u1=-1+2*(ix+1)/grid;v0=-1+2*iy/grid;v1=-1+2*(iy+1)/grid;uc=(u0+u1)/2;vc=(v0+v1)/2
   def point(u,vv):return(n[0]*face_half+h[0]*he*u+v[0]*ve*vv,n[1]*face_half+h[1]*he*u+v[1]*ve*vv,n[2]*face_half+h[2]*he*u+v[2]*ve*vv)
   quad(bs[material_for(fi,uc,vc)],(point(u0,v0),point(u1,v0),point(u1,v1),point(u0,v1)),n)
meshes.append({'name':'legacy_layered_shell','primitives':[p for p in(prim(b,i)for i,b in enumerate(bs))if p]});shell_mesh=len(meshes)-1
cb=bucket();extent=(.58,.62,.58);half=[x/2 for x in extent]
for n,h,v in FACES:
 face_half=half[0] if n[0] else half[1] if n[1] else half[2];he=abs(h[0])*half[0]+abs(h[1])*half[1]+abs(h[2])*half[2];ve=abs(v[0])*half[0]+abs(v[1])*half[1]+abs(v[2])*half[2]
 def point(u,vv):return(n[0]*face_half+h[0]*he*u+v[0]*ve*vv,n[1]*face_half+h[1]*he*u+v[1]*ve*vv,n[2]*face_half+h[2]*he*u+v[2]*ve*vv)
 quad(cb,(point(-1,-1),point(1,-1),point(1,1),point(-1,1)),n)
meshes.append({'name':'square_nucleus','primitives':[prim(cb,7)]});core_mesh=len(meshes)-1
fb=bucket();quad(fb,((W/2,-H/2,-.50),(-W/2,-H/2,-.50),(-W/2,H/2,-.50),(W/2,H/2,-.50)),(0,0,-1),reverse=True);meshes.append({'name':'square_pixel_face','primitives':[prim(fb,8)]});face_mesh=len(meshes)-1
materials=[{'name':n,'pbrMetallicRoughness':{'baseColorFactor':[lin(c[0]),lin(c[1]),lin(c[2]),1],'metallicFactor':0,'roughnessFactor':1},'doubleSided':False,'extensions':{'KHR_materials_unlit':{}}}for n,c in SRGB]
materials.append({'name':'SlimeCore','pbrMetallicRoughness':{'baseColorFactor':[lin(.13),lin(.38),lin(.29),1],'metallicFactor':0,'roughnessFactor':1},'doubleSided':False,'extensions':{'KHR_materials_unlit':{}}})
materials.append({'name':'SlimeFace','pbrMetallicRoughness':{'baseColorFactor':[1,1,1,1],'metallicFactor':0,'roughnessFactor':1},'doubleSided':False,'alphaMode':'BLEND','extensions':{'KHR_materials_unlit':{}}})
def node(name,mesh=None,children=None,translation=None,scale=None,extras=None):
 e={'name':name}
 if mesh is not None:e['mesh']=mesh
 if children is not None:e['children']=children
 if translation is not None:e['translation']=translation
 if scale is not None:e['scale']=scale
 if extras is not None:e['extras']=extras
 nodes.append(e);return len(nodes)-1
root=node('SlimeRoot',children=[],extras={'asteria_asset':'creature/slime','unit':'meters','forward':'-Z','collider':{'shape':'aabb','size':[.78,.84,.78],'offset':[0,.42,0]},'collider_is_animated':False});visual=node('Visual',children=[]);body=node('BodyPivot',children=[],translation=[0,.5,0]);inner=node('InnerCore',mesh=core_mesh,translation=[0,-.025,0],scale=[1,1,1]);shell=node('Shell',mesh=shell_mesh);face=node('Face',mesh=face_mesh,translation=[0,0,0]);nodes[body]['children']=[shell,inner,face];nodes[visual]['children']=[body];hit=node('Hitbox_AABB',translation=[0,.42,0],extras={'asteria_collider':{'shape':'aabb','size':[.78,.84,.78],'solid':True,'targetable':True},'debug_display':False});nodes[root]['children']=[visual,hit]
def track(name,times,body_scale,center=None,core_scale=None,core_angle=None):
 center=center if center is not None else[.5*s[1]for s in body_scale];core_scale=core_scale if core_scale is not None else[[1,1,1]for _ in times];core_angle=core_angle if core_angle is not None else[0 for _ in times];ta=acc(times,kind='SCALAR',bounds=True);ch=[];sa=[]
 def add(target,path,vals,kind):out=acc([v for row in vals for v in row],kind=kind);sa.append({'input':ta,'output':out,'interpolation':'LINEAR'});ch.append({'sampler':len(sa)-1,'target':{'node':target,'path':path}})
 add(body,'scale',body_scale,'VEC3');add(body,'translation',[[0,y,0]for y in center],'VEC3');add(inner,'scale',core_scale,'VEC3');add(inner,'rotation',[[0,math.sin(a/2),0,math.cos(a/2)]for a in core_angle],'VEC4');animations.append({'name':name,'channels':ch,'samplers':sa,'extras':{'loop_recommended':name in('Idle','Airborne')}})
track('Idle',[0,.5,1,1.5,2],[[1,1,1],[1.018,.974,1.018],[1,1,1],[.988,1.021,.988],[1,1,1]],core_scale=[[1,1,1],[1.065,.97,1.065],[1,1,1],[.96,1.04,.96],[1,1,1]],core_angle=[0,.04,0,-.04,0]);track('Anticipate',[0,.07,.17,.24],[[1,1,1],[1.09,.845,1.09],[1.125,.76,1.125],[1.09,.83,1.09]],core_scale=[[1,1,1],[1.05,.9,1.05],[1.10,.84,1.10],[1.05,.92,1.05]],core_angle=[0,-.07,-.10,-.04]);track('Airborne',[0,.12,.35,.55,.72],[[1.09,.83,1.09],[.91,1.15,.91],[.96,1.085,.96],[.97,1.06,.97],[1,1,1]],core_scale=[[1,1,1],[.95,1.08,.95],[.98,1.035,.98],[1,1,1],[1,1,1]],core_angle=[0,.13,.23,.11,0]);track('Land',[0,.045,.12,.20,.34],[[.97,1.055,.97],[1.17,.74,1.17],[1.12,.805,1.12],[.975,1.047,.975],[1,1,1]],core_scale=[[1,1,1],[1.12,.85,1.12],[1.05,.94,1.05],[.98,1.025,.98],[1,1,1]]);track('Hurt',[0,.085,.15,.24,.38],[[1,1,1],[1.1,.88,1.1],[.94,1.08,.94],[1.025,.968,1.025],[1,1,1]],core_angle=[0,-.22,.19,-.08,0]);track('Death',[0,.12,.31,.55,.75],[[1,1,1],[1.13,.8,1.13],[1.2,.60,1.2],[1.12,.19,1.12],[.001,.001,.001]],center=[.5,.4,.3,.095,.0005],core_scale=[[1,1,1],[1.1,.88,1.1],[1.2,.7,1.2],[.95,.3,.95],[.001,.001,.001]],core_angle=[0,.2,.5,.9,1.2])
scene={'asset':{'version':'2.0','generator':'Asteria fixed-color legacy slime v4'},'scene':0,'scenes':[{'name':'Slime','nodes':[root]}],'extensionsUsed':['KHR_materials_unlit'],'nodes':nodes,'meshes':meshes,'materials':materials,'animations':animations,'bufferViews':views,'accessors':accessors,'buffers':[{'byteLength':len(binary)}],'extras':{'asset_id':'asteria:slime_base','color_materials':[m['name']for m in materials],'collision_source':'slime.collider.json','notes':'Original cubic legacy slime with subdivided fixed authored green depth zones; no shell/core tint or texture override.'}}
j=json.dumps(scene,separators=(',',':')).encode();j+=b' '*(-len(j)%4);bb=bytes(binary)+b'\0'*(-len(binary)%4);glb=struct.pack('<4sII',b'glTF',2,12+8+len(j)+8+len(bb))+struct.pack('<I4s',len(j),b'JSON')+j+struct.pack('<I4s',len(bb),b'BIN\0')+bb;(OUT/'slime.glb').write_bytes(glb)
