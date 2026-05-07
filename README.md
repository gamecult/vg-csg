# vg-csg

Lean constructive geometry tooling for Rust/Bevy-math scene blockouts.

`vg-csg` is the extracted CSG kernel from VibeGeometry. It is not trying to be
a full CAD kernel. It is a small, inspectable machine for agent-authored level
and habitat geometry:

- ordered additive, subtractive, and intersecting brush streams
- axis-aligned and oriented box CSG through convex plane splitting
- RealtimeCSG-shaped public vocabulary for operations, branches, trees, and
  dirty brush generations
- exact observable parity fixtures where the public RealtimeCSG/demo API surface
  overlaps
- mesh-array output suitable for Bevy-side ingestion
- dirty-frontier and prefix-checkpoint experiments for realtime editing
- additive habitat primitives used by the current examples

The crate is young and sharp in the places young crates are sharp. It exists so
we can build concrete fixtures, profile real edits, and grow the API from
working procedural scenes instead of designing the perfect museum label first.

## Quick Start

```powershell
cargo test
cargo run --example csg_room
cargo run --example dream_machine_level
cargo run --example industrial_maintenance_loop
```

The showpiece examples write OBJ/MTL/SVG artifacts under:

```text
experiments/generated/
```

## Core Shape

```rust
use bevy_math::{Quat, Vec3};
use vg_csg::{LevelDsl, MaterialId};

let mut level = LevelDsl::new();
level.solid_box(
    "wall",
    Vec3::new(0.0, 0.0, 1.5),
    Vec3::new(5.0, 0.3, 3.0),
    MaterialId(1),
);
level.cut_oriented_box(
    "angled doorway",
    Vec3::new(0.0, 0.0, 1.2),
    Vec3::new(1.4, 0.8, 2.2),
    Quat::from_rotation_z(0.25),
);

let output = level.assemble();
println!("triangles={}", output.mesh.triangle_count());
```

## Doctrine

CSG is the adjudicator, not the authoring language. Good procedural scenes
should still be written in the coordinate system that belongs to the domain:
room bay, curved shell segment, manifold face, staging rail, brush tree, dirty
frontier. Compile those local claims into CSG brushes late.

That bias is visible in `examples/industrial_maintenance_loop.rs`, where a
curved habitat room is authored in tangent/radial/up space and then lowered into
world-space oriented boxes and mesh bands.

## Docs

Research and implementation notes live in `docs/`:

- `docs/realtime-csg-bevy-assembler.md`
- `docs/realtime-csg-doctrine.md`
- `docs/csg-performance-fixtures.md`

## License

MIT
