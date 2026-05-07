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

The older public demo is more important than the Unity product for clean
algorithm work. Sander van Rossen's 2009-2010 blog series describes the
principles directly: consistent epsilon classification, half-edges, polygon
cutting, convex brush construction, boolean categorization, and the idea that
subtraction can be expressed as composed logical operations. The
`LogicalError/Realtime-CSG-demo` repository is public C# code from the article
"Real Time Constructive Solid Geometry" in *Game Development Tools*. It is now
the preferred research input for algorithmic behavior because it is public
source tied to the article rather than a closed Unity native kernel.

Verified public sources:

- Sander van Rossen, "Realtime CSG - Part 1"
- Sander van Rossen, "Realtime CSG - Part 5"
- Sander van Rossen and Matthew Baranowski, "Real-Time Constructive Solid
  Geometry", *Game Development Tools*
- `LogicalError/Realtime-CSG-demo`

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

The first actual CSG operator was exact AABB subtraction against additive AABB
solids. The current kernel has moved to convex decomposition: an additive
convex solid is split against each outward cutter plane, subtraction emits the
outside fragments, and intersection keeps the inside remainder. This supports
angled box cuts, real `Common` branch output for convex boxes, and gives us the
correct shape for a future general convex brush kernel.

Polygons now carry the first routing metadata the mature kernel needs:
category, visibility, reversal, and bounds. Whole polygons can already be
classified as inside, outside, aligned, or reverse-aligned against a convex
brush when they do not cross cutter planes. That is deliberately not sold as
the full router yet. It is the labeled wire we will plug the splitter into.

The important doctrinal adjustment from the public demo and blog series is that
real-time CSG should think in **classification**, not only carving. A polygon is
inside, outside, aligned, or reverse-aligned relative to another node. Boolean
operations are then mostly routing tables for those categories. Our current
fragment-splitting implementation is a stepping stone; the faster mature kernel
should preserve brush polygons, classify them through the tree, and emit only
the visible categories.

The expanded doctrine lives in
`docs/research/realtime-csg-doctrine.md`; keep this file focused on the current
crate shape.

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

Use CSG where boolean space carries meaning:

- doorways, windows, hatches, service trenches
- docking sockets and tether anchor gaps
- ring terraces and mechanically repeated cutouts
- fast blockouts where the same cutter can punch many solids
- common-volume filters where rooms, shafts, or constraint regions overlap

Use additive procedural primitives where a form is better described by a field
or sweep than by carving:

- domes, rings, tethers, panels, radiators
- city-field parcels after street generation
- flower-like utility crests and other habitat-specific silhouettes

The immediate next upgrade is not "more primitives." It is the category-router
kernel: split crossing polygons, route classified source polygons through
boolean branches, preserve material/surface metadata, and then add a BVH or
spatial hash so cutter cost does not scale like a punishment.

## Limits

`vg_csg` is not yet a full RealtimeCSG replacement.

- subtractive convex support currently covers axis-aligned and oriented boxes
- non-box subtractors are reported and ignored
- intersect/common support currently covers axis-aligned and oriented boxes
- output is Bevy-compatible mesh data, not yet a Bevy `Mesh` asset constructor
- there is no editor gizmo, ECS plugin, or incremental spatial index yet
- current subtraction emits split fragments with generated cap polygons; the
  target kernel should move toward article/demo-style category routing over
  brush polygons for speed and cleaner material behavior

Those limits are explicit because invisible ambition is how you get haunted by
your own abstractions. The useful loop exists now: script brushes, assemble,
inspect mesh stats, and replace the kernel under the same DSL when the target
demands it.
