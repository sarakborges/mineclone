# Portal travel data model

This document defines only the prepared data contract for future inter-dimensional portal travel. It does not define a portal's physical form, activation interaction, item, spell, scroll, crafting recipe, visuals, lifetime, or runtime teleport behavior.

## Content ownership

Portal travel definitions live under `data/portals/` and identify only the destination contract:

- destination dimension;
- coordinate mapping policy;
- root arrival Structure reference.

The initial coordinate policy is `exact`: destination X/Y/Z are the same world coordinates as the origin X/Y/Z. The policy is authored explicitly in data so the travel contract is not hidden in portal runtime code.

No concrete portal definition is authored yet.

## Arrival carving is ordinary Structure authoring

There is no portal-specific carving system and no portal-specific surface-connection algorithm.

`destination.arrivalStructure` references a normal Structure or Structure group. That root Structure owns the arrival-space carve through the existing Structure palette (`clear`, blocks, fluids, attachments, and so on).

If the arrival point is underground or otherwise needs a route to navigable terrain, the arrival Structure uses the same generic `connector.target` graph used by cave entrances and other connected Structures. Follow-up pieces, variation, strength, loop loss, distances, rotation, conflict rules, replacement policy, and all other placement behavior remain owned by the generic Structure system.

Portal code must not duplicate cave connector planning, tunnel generation, surface search, carving, or connected-Structure traversal. A portal only chooses the authored arrival root; the Structure system resolves the graph.

## Current schema

A future portal definition has this shape:

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

The following are intentionally not part of this model yet:

- crafting recipes or material costs;
- scroll/item definitions;
- placement or activation input;
- portal geometry or presentation;
- whether the portal is temporary, persistent, one-way, or bidirectional;
- return-trip lifecycle;
- runtime persistence/state for opened portals.

Those concerns must be designed separately instead of being guessed into the destination/carve data model.
