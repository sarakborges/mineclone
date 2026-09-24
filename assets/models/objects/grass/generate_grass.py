#!/usr/bin/env python3
"""Generate Asteria's reusable low-poly grass world-object model."""
from __future__ import annotations
import json,math,struct
from pathlib import Path
OUT=Path(__file__).resolve().parent
B=(
(-.18,-.10,.58,.105,.050,.040,.020,-.045,.020,-16,.76),(0,-.06,.72,.115,.055,.045,.020,.020,-.025,7,.98),(.17,-.08,.52,.095,.045,.035,.018,.055,.010,20,.82),(-.10,.08,.46,.090,.042,.032,.016,-.055,-.010,28,.88),(.09,.10,.63,.105,.048,.038,.018,.035,.035,-24,.92),(-.25,.07,.36,.085,.040,.030,.015,-.065,.030,11,.70),(.25,.06,.40,.082,.038,.028,.014,.070,.020,-11,.74),(-.02,.19,.50,.092,.042,.032,.016,-.010,.065,38,.84),(.03,-.20,.43,.088,.040,.030,.015,.015,-.070,-36,.79))
P=[];N=[];C=[];I=[]
def sub(a,b):return tuple(a[i]-b[i] for i in range(3))
def cr(a,b):return(a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
def dt(a,b):return sum(a[i]*b[i] for i in range(3))
def nm(v):l=math.sqrt(dt(v,v));return tuple(x/l for x in v)
def rt(x,z,y):c,s=math.cos(y),math.sin(y);return x*c+z*s,-x*s+z*c
def q(ps,c,br,f):
 ps=list(ps);n=cr(sub(ps[1],ps[0]),sub(ps[2],ps[0]));fc=tuple(sum(p[i] for p in ps)/4 for i in range(3))
 if dt(n,sub(fc,c))<0:ps.reverse();n=cr(sub(ps[1],ps[0]),sub(ps[2],ps[0]))
 n=nm(n);o=len(P)//3;s=max(0,min(1,br*f))
 for p in ps:P.extend(p);N.extend(n);C.extend((s,s,s,1))
 I.extend((o,o+1,o+2,o,o+2,o+3))
for cx,cz,ht,bw,bd,tw,td,lx,lz,dg,br in B:
 y=math.radians(dg)
 def p(x,yy,z,t=False):rx,rz=rt(x,z,y);return(cx+rx+(lx if t else 0),yy,cz+rz+(lz if t else 0))
 b0,b1,b2,b3=p(-bw/2,0,-bd/2),p(bw/2,0,-bd/2),p(bw/2,0,bd/2),p(-bw/2,0,bd/2);t0,t1,t2,t3=p(-tw/2,ht,-td/2,1),p(tw/2,ht,-td/2,1),p(tw/2,ht,td/2,1),p(-tw/2,ht,td/2,1);c=(cx+lx/2,ht/2,cz+lz/2)
 for ps,f in (((b0,b1,b2,b3),.52),((t0,t1,t2,t3),1.08),((b0,b1,t1,t0),.82),((b1,b2,t2,t1),.93),((b2,b3,t3,t2),1),((b3,b0,t0,t3),.72)):q(ps,c,br,f)
bb=bytearray();views=[];acc=[]
def st(data,target):bb.extend(b"\0"*(-len(bb)%4));o=len(bb);bb.extend(data);views.append({"buffer":0,"byteOffset":o,"byteLength":len(data),"target":target});return len(views)-1
def ac(v,k,ct,t,b=False):
 w={"SCALAR":1,"VEC3":3,"VEC4":4}[k];fmt={5126:"f",5123:"H"}[ct];e={"bufferView":st(struct.pack("<"+fmt*len(v),*v),t),"componentType":ct,"count":len(v)//w,"type":k}
 if b:r=[v[i:i+w]for i in range(0,len(v),w)];e["min"]=[float(min(x[j]for x in r))for j in range(w)];e["max"]=[float(max(x[j]for x in r))for j in range(w)]
 acc.append(e);return len(acc)-1
scene={"asset":{"version":"2.0","generator":"Asteria grass object generator v1"},"scene":0,"scenes":[{"name":"GrassObject","nodes":[0]}],"nodes":[{"name":"GrassRoot","children":[1],"extras":{"asteria_asset":"object/grass","unit":"meters","origin":"ground_center","forward":"-Z","collision":"none","tint_material":"GrassTint"}},{"name":"Visual","mesh":0}],"meshes":[{"name":"grass_tuft","primitives":[{"attributes":{"POSITION":ac(P,"VEC3",5126,34962,1),"NORMAL":ac(N,"VEC3",5126,34962),"COLOR_0":ac(C,"VEC4",5126,34962)},"indices":ac(I,"SCALAR",5123,34963),"material":0,"mode":4}]}],"materials":[{"name":"GrassTint","pbrMetallicRoughness":{"baseColorFactor":[1,1,1,1],"metallicFactor":0,"roughnessFactor":1},"doubleSided":False}],"bufferViews":views,"accessors":acc,"buffers":[{"byteLength":len(bb)}],"extras":{"asset_id":"asteria:grass_object","kind":"world_object","blade_count":len(B),"tintable":True,"bounds_meters":{"width":.67,"height":.72,"depth":.56}}}
j=json.dumps(scene,separators=(",",":")).encode();j+=b" "*(-len(j)%4);bn=bytes(bb)+b"\0"*(-len(bb)%4);g=struct.pack("<4sII",b"glTF",2,12+8+len(j)+8+len(bn))+struct.pack("<I4s",len(j),b"JSON")+j+struct.pack("<I4s",len(bn),b"BIN\0")+bn;(OUT/"grass.glb").write_bytes(g)
