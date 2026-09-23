#!/usr/bin/env python3
"""Generate the elemental Air Slime from the rounded blob base.

The body keeps the standard 24x20x22 blob silhouette. Color is authored into
the GLB: green-leaning turquoise at the base, pale mint in the center, and a
near-white top/highlight. Two blocky three-feather wings and two tiny air motes
are part of the visual model. Only SlimeFace is a runtime texture override.
"""
from pathlib import Path
OUT=Path(__file__).resolve().parent
BODY_SIZE=(1.20,1.00,1.14)
VOXEL_RESOLUTION=(24,20,22)
JUMP_SPEED=6.75
FALL_GRAVITY_SCALE=.45
PALETTE={
 'base':(.30,.78,.67),'center':(.48,.88,.78),'outer':(.20,.62,.53),
 'bottom':(.24,.72,.60),'top':(.84,.97,.93),'light':(.93,.995,.98),'bright':(1,1,1),
 'wing_white':(.96,1,.995),'wing_mint':(.72,.94,.88),'detail':(.56,.90,.82),
}
print("slime_air.glb is the checked-in generated asset; authoring constants are kept here and in HANDOFF.md")
