# Portal travel and activation

Asteria separates portal activation/geometry from the eventual inter-dimensional travel runtime. Portal frames and the Dimensional Slicer now own how a portal is opened in the world; `data/portals/` remains the prepared destination/arrival contract for travel itself.

## Dimensional Slicer

`asteria:dimensional_slicer` is an ordinary inventory item. Its stack metadata key `target_dimension` identifies the dimension the Slicer is attuned to connect.

The initial Creative Inventory copy is authored at runtime with:

```text
target_dimension=asteria:umbral
```

The metadata is part of stack identity and persistence. Explicitly authored stacks may use another valid target such as `asteria:overworld`.

The Slicer is consumed only after the complete portal interior passes preflight and every portal layer is placed successfully. Invalid/open frames, unsupported dimensions, occupied interiors, unloaded boundaries, or failed placement do not consume it.

## Portal-frame blocks

Portal-frame capability is block-data-driven through normal block tags:

```json
{
  "tags": [
    "is_portal_frame",
    "accept_dimension:asteria:overworld",
    "accept_dimension:asteria:umbral"
  ]
}
```

`is_portal_frame` marks a block as usable in portal boundaries. Each `accept_dimension:<dimension-id>` tag authorizes that target dimension for the frame material.

Glass is the initial frame block and accepts both Overworld and Umbral.

## Player-authored portal shape

A portal has no fixed rectangle or authored size. The player builds a closed planar boundary from compatible portal-frame blocks and right-clicks any block of that boundary with an attuned Dimensional Slicer.

Activation tests the three voxel planes independently. For each plane, it flood-fills empty cells adjacent to the clicked frame block. A candidate is valid only when the region is fully closed by frame blocks that accept the Slicer's target dimension. Encountering a non-frame solid, fluid, unloaded voxel, an open boundary, or the safety size/span limits rejects that candidate.

Exactly one valid plane must be resolved. Its plane determines the centered layer orientation:

- YZ plane -> `right` layer face;
- XZ plane -> `top` layer face;
- XY plane -> `front` layer face.

The complete enclosed interior is then filled with the portal's centered Layer. Centered Layers are explicitly allowed to exist in empty voxels; ordinary surface Layers still require a supporting block.

The current Umbral connection uses `asteria:umbral_portal` for both `asteria:umbral` and `asteria:overworld` targets. This is the visual/geometry identity of the Overworld <-> Umbral portal pair, not a second block type.

## Travel destination contract

Portal travel definitions live under `data/portals/` and identify only the destination contract:

- destination dimension;
- coordinate mapping policy;
- root arrival Structure reference.

The initial coordinate policy is `exact`: destination X/Y/Z are the same world coordinates as the origin X/Y/Z. The policy is authored explicitly in data so the travel contract is not hidden in portal runtime code.

No concrete travel definition is authored yet.

## Arrival carving is ordinary Structure authoring

There is no portal-specific carving system and no portal-specific surface-connection algorithm.

`destination.arrivalStructure` references a normal Structure or Structure group. That root Structure owns the arrival-space carve through the existing Structure palette (`clear`, blocks, fluids, attachments, and so on).

If the arrival point is underground or otherwise needs a route to navigable terrain, the arrival Structure uses the same generic `connector.target` graph used by cave entrances and other connected Structures. Follow-up pieces, variation, strength, loop loss, distances, rotation, conflict rules, replacement policy, and all other placement behavior remain owned by the generic Structure system.

Portal code must not duplicate cave connector planning, tunnel generation, surface search, carving, or connected-Structure traversal. A portal only chooses the authored arrival root; the Structure system resolves the graph.

## Travel schema

A future portal travel definition has this shape:

```json
{
  "id": "asteria:example",
  "destination": {
    "dimension": "asteria:example_dimension",
    "coordinateMapping": "exact",
    "arrivalStructure": "asteria:example_arrival"
  }
}
```

`arrivalStructure` may reference either one Structure or a Structure group, using the same reference semantics already supported by `StructureRegistry`.

## Explicitly deferred

The following remain intentionally outside the implemented activation stage:

- crafting recipes or material costs for Dimensional Slicers;
- runtime teleport execution;
- arrival Structure authoring for concrete portal destinations;
- temporary/persistent, one-way, or bidirectional travel lifecycle rules;
- return-trip lifecycle;
- opened-portal travel state beyond the persisted voxel Layer geometry.

Those concerns must extend the existing portal/Structure contracts rather than introducing parallel carving or connection systems.
