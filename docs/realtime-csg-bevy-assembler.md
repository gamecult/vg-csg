# Realtime CSG Bevy Assembler

## Source Read

LogicalError's RealtimeCSG is useful as an architectural reference, not as a
kernel we can directly port. The public repository describes a Unity editor
plugin for CSG level editing and states that runtime editing is not supported
through a maintained API. It also states that the native C++ source is not
public.

The public C# layer still exposes the parts worth stealing cleanly:

- control meshes made from vertices, half edges, polygons, and texgen/surface
  metadata
- tree, branch, and brush handles
- explicit brush operation types
- dirty generation counters
- a rebuild step that turns authored brushes into renderable meshes

That shape maps well to Bevy: keep authoring data as lightweight brush records,
rebuild only dirty worlds, and emit plain mesh buffers that can later be handed
to Bevy render assets.

## Current Port Shape

`crates/vg_csg` is the first lean Rust organ.

It uses `bevy_math` for `Vec3`, `Vec2`, and Bevy-compatible spatial types while
keeping output as simple arrays:

```rust
use bevy_math::Vec3;
use vg_csg::{LevelDsl, MaterialId};

let mut level = LevelDsl::new();
level.solid_box(
    "wall",
    Vec3::new(0.0, 0.0, 1.5),
    Vec3::new(6.0, 0.3, 3.0),
    MaterialId(1),
);
level.cut_box(
    "door",
    Vec3::new(0.0, 0.0, 1.0),
    Vec3::new(1.2, 0.6, 2.0),
);

let output = level.assemble();
assert!(output.mesh.triangle_count() > 12);
```

The first actual CSG operator is exact AABB subtraction against additive AABB
solids. A cutter splits a source box into up to six surviving slabs. This is
enough for doors, windows, vents, corridor punches, terraces, anchor trenches,
and level blockout loops.

Procedural non-CSG primitives are additive:

- `CylinderZ` for tethers, shafts, columns, conduits, and ring anchors
- `DomeCapZ` for flat-base pressure bubbles and caps
- `FloretArm` for sunflower/Citadel-scale solar and radiator petals

Those primitives are intentionally in the same brush stream as CSG boxes, so a
level script can mix hard architectural cuts with habitat-specific forms:

```rust
level.dome_cap_z(
    "city dome",
    vg_csg::DomeCapZSpec {
        center: Vec3::ZERO,
        radius: 20.0,
        height: 8.0,
        rings: 12,
        segments: 64,
        material: MaterialId(20),
    },
);
level.floret_arm(
    "radiator petal",
    vg_csg::FloretArmSpec {
        anchor: Vec3::new(20.0, 0.0, 0.2),
        direction: Vec3::X,
        length: 120.0,
        root_width: 6.0,
        tip_width: 18.0,
        thickness: 0.12,
        tip_lift: 3.0,
        material: MaterialId(21),
    },
);
```

## Doctrine

Think of brushes as legal claims on volume.

An additive brush says: "this matter exists." A subtractive brush says: "this
space must remain empty." The assembler is the clerk that resolves claims in
order and emits triangles only after the claims stop arguing.

For interactive work, the brush list is the durable design surface. The mesh is
a cache. This matters because agents should revise intent by moving, adding, or
renaming brushes, not by poking triangles after the fact.

Use CSG where negative space carries meaning:

- doorways, windows, hatches, service trenches
- docking sockets and tether anchor gaps
- ring terraces and mechanically repeated cutouts
- fast blockouts where the same cutter can punch many solids

Use additive procedural primitives where a form is better described by a field
or sweep than by carving:

- domes, rings, tethers, panels, radiators
- city-field parcels after street generation
- flower-like utility crests and other habitat-specific silhouettes

The immediate next upgrade is not "more primitives." It is a stronger convex
brush kernel: plane classification, polygon clipping, fragment merge policy,
and a BVH or spatial hash so cutter cost does not scale like a punishment.

## Limits

`vg_csg` is not yet a full RealtimeCSG replacement.

- only box subtractors affect box solids
- non-box subtractors are reported and ignored
- intersect brushes are reported and ignored
- output is Bevy-compatible mesh data, not yet a Bevy `Mesh` asset constructor
- there is no editor gizmo, ECS plugin, or incremental spatial index yet

Those limits are explicit because invisible ambition is how you get haunted by
your own abstractions. The useful loop exists now: script brushes, assemble,
inspect mesh stats, and replace the kernel under the same DSL when the target
demands it.
