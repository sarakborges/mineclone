#!/usr/bin/env python3
"""Generate the compact cubic Anemo slime model."""
from __future__ import annotations
import json, struct
from pathlib import Path
OUT=Path(__file__).resolve().parent
B=bytearray(); V=[]; A=[]; M=[]; N=[]; AN=[]

def store(vals,fmt,target=None):
    B.extend(b'\0'*(-len(B)%4)); o=len(B); B.extend(struct.pack('<'+fmt*len(vals),*vals))
    e={'buffer':0,'byteOffset':o,'byteLength':len(B)-o}
    if target:e['target']=target
    V.append(e); return len(V)-1
def acc(vals,typ,fmt='f',comp=5126,target=None,mm=False):
    n={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[typ]
    e={'bufferView':store(vals,fmt,target),'componentType':comp,'count':len(vals)//n,'type':typ}
    if mm:
        rows=[vals[i:i+n] for i in range(0,len(vals),n)]
        e['min']=[min(r[j] for r in rows) for j in range(n)]; e['max']=[max(r[j] for r in rows) for j in range(n)]
    A.append(e); return len(A)-1

P=[-.5,0,-.5, .5,0,-.5, .5,1,-.5, -.5,1,-.5, -.5,0,.5, .5,0,.5, .5,1,.5, -.5,1,.5]
I=[0,2,1,0,3,2, 1,6,5,1,2,6, 5,7,4,5,6,7, 4,3,0,4,7,3, 3,6,2,3,7,6, 4,1,5,4,0,1]
C=[]
for y in (0,0,1,1,0,0,1,1):
    c=(.20,.66,.55,1) if y==0 else (.86,.98,.94,1); C.extend(c)
pi=acc(P,'VEC3',target=34962,mm=True); ii=acc(I,'SCALAR','H',5123,34963); ci=acc(C,'VEC4',target=34962)
FP=[.39,.18,-.506,-.39,.18,-.506,-.39,.60,-.506,.39,.60,-.506]
FU=[0,1,1,1,1,0,0,0]; FI=[0,1,2,0,2,3]
fpi=acc(FP,'VEC3',target=34962,mm=True); fui=acc(FU,'VEC2',target=34962); fii=acc(FI,'SCALAR','H',5123,34963)

def mat(name,c,blend=False):
    e={'name':name,'pbrMetallicRoughness':{'baseColorFactor':[c[0],c[1],c[2],1],'metallicFactor':0,'roughnessFactor':1},
       'extensions':{'KHR_materials_unlit':{}}}
    if blend:e['alphaMode']='BLEND'
    return e
MAT=[mat('AirShell',(1,1,1)),mat('AirCore',(.25,.72,.62)),mat('SlimeFace',(1,1,1),True),
     mat('AirWingWhite',(.94,1,.98)),mat('AirWingMint',(.56,.90,.82)),mat('AirDetail',(.45,.86,.76))]
M.append({'name':'Body','primitives':[{'attributes':{'POSITION':pi,'COLOR_0':ci},'indices':ii,'material':0}]})
for name,mi in [('Core',1),('WingWhite',3),('WingMint',4),('Detail',5)]:
    M.append({'name':name,'primitives':[{'attributes':{'POSITION':pi},'indices':ii,'material':mi}]})
M.append({'name':'Face','primitives':[{'attributes':{'POSITION':fpi,'TEXCOORD_0':fui},'indices':fii,'material':2}]})

def node(name,mesh=None,ch=None,t=None,s=None,extra=None):
    e={'name':name}
    if mesh is not None:e['mesh']=mesh
    if ch is not None:e['children']=ch
    if t is not None:e['translation']=t
    if s is not None:e['scale']=s
    if extra is not None:e['extras']=extra
    N.append(e); return len(N)-1
root=node('SlimeRoot',ch=[],extra={'asteria_asset':'creature/slime_anemo_legacy'})
visual=node('Visual',ch=[])
body=node('BodyPivot',ch=[],t=[0,0,0])
shell=node('Shell',0,s=[.96,.90,.96])
core=node('InnerCore',1,t=[0,.15,0],s=[.54,.58,.54])
face=node('Face',5)
rw=[]; lw=[]
for x,y,sz,mesh in ((.57,.39,[.18,.22,.13],3),(.72,.31,[.17,.19,.12],4),(.86,.24,[.15,.16,.11],3)):
    rw.append(node('R',mesh,t=[x,y,0],s=sz)); lw.append(node('L',mesh,t=[-x,y,0],s=sz))
m1=node('AirMote',4,t=[.30,.70,-.13],s=[.075,.075,.075]); m2=node('AirMote',4,t=[-.27,.64,-.15],s=[.06,.06,.06])
N[body]['children']=[shell,core,face,*rw,*lw,m1,m2]; N[visual]['children']=[body]
hit=node('Hitbox_AABB',t=[0,.42,0],extra={'asteria_collider':{'shape':'aabb','size':[.78,.84,.78],'solid':True,'targetable':True}})
N[root]['children']=[visual,hit]

def anim(name,times,scales):
    ti=acc(times,'SCALAR',mm=True)
    so=acc([v for row in scales for v in row],'VEC3')
    AN.append({'name':name,'channels':[{'sampler':0,'target':{'node':body,'path':'scale'}}],
               'samplers':[{'input':ti,'output':so,'interpolation':'LINEAR'}]})
anim('Idle',[0,1,2],[[1,1,1],[1.018,.974,1.018],[1,1,1]])
anim('Anticipate',[0,.12,.24],[[1,1,1],[1.12,.77,1.12],[1.09,.83,1.09]])
anim('Airborne',[0,.24,.72],[[1.09,.83,1.09],[.92,1.14,.92],[1,1,1]])
anim('Land',[0,.10,.34],[[1,1,1],[1.16,.76,1.16],[1,1,1]])
anim('Hurt',[0,.12,.38],[[1,1,1],[1.09,.89,1.09],[1,1,1]])
anim('Death',[0,.35,.75],[[1,1,1],[1.18,.58,1.18],[.001,.001,.001]])
G={'asset':{'version':'2.0','generator':'Asteria cubic Anemo v1'},'scene':0,'scenes':[{'nodes':[root]}],
   'extensionsUsed':['KHR_materials_unlit'],'nodes':N,'meshes':M,'materials':MAT,'animations':AN,
   'bufferViews':V,'accessors':A,'buffers':[{'byteLength':len(B)}],
   'extras':{'asset_id':'asteria:slime_anemo_legacy','collision_source':'slime_anemo_legacy.collider.json'}}
j=json.dumps(G,separators=(',',':')).encode(); j+=b' '*(-len(j)%4); b=bytes(B)+b'\0'*(-len(B)%4)
glb=struct.pack('<4sII',b'glTF',2,12+8+len(j)+8+len(b))+struct.pack('<I4s',len(j),b'JSON')+j+struct.pack('<I4s',len(b),b'BIN\0')+b
(OUT/'slime_anemo_legacy.glb').write_bytes(glb)
print(f'Generated {OUT/"slime_anemo_legacy.glb"} ({len(glb)} bytes)')
