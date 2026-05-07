# Realtime CSG Doctrine

## Provenance

This note distills public material around Sander van Rossen and Matthew
Baranowski's "Real-Time Constructive Solid Geometry" article and the later
RealtimeCSG ecosystem. It excludes closed native kernel internals.

Primary clean inputs:

- Sander van Rossen, [Realtime CSG - Part 1](https://sandervanrossen.blogspot.com/2009/12/realtime-csg-part-1.html)
- Sander van Rossen, [Realtime CSG - Part 5](https://sandervanrossen.blogspot.com/2010/05/realtime-csg-part-5.html)
- Sander van Rossen, [Realtime CSG - Optimizations](https://sandervanrossen.blogspot.com/2010/05/realtime-csg-optimizations.html)
- [`LogicalError/Realtime-CSG-demo`](https://github.com/LogicalError/Realtime-CSG-demo)
- Patrick Kaster, [BSO3](https://patrickkaster.github.io/BSO3/index.html)
- [Geometry in Milliseconds - Real-Time Constructive Solid Geometry](https://www.classcentral.com/course/youtube-geometry-in-milliseconds-real-time-constructive-solid-geometry-158033)
- [three.js CSG support discussion](https://bleepingcoder.com/three-js/427091361/constructive-solid-geometry-csg-support)

## Core Model

Realtime CSG is not primarily a triangle boolean problem. It is a brush polygon
visibility problem.

The article/blog/demo model starts with convex brushes, cuts source brush
polygons only when needed, classifies the resulting polygon pieces through a
CSG tree, and emits the pieces that remain visible. It does not treat the final
mesh as the authority. The brush tree is the authority; the mesh is cache and
evidence.

The highest-value shift for `vg_csg` is therefore:

```text
from: subtract cutter -> produce new convex fragments -> triangulate fragments
to:   source brush polygons -> split only at crossing planes -> classify through tree -> emit visible polygon pieces
```

Our current convex splitter is useful scaffolding, but it is still a carver.
The target kernel is a classifier.

## Doctrines

### 1. Consistency Beats Accuracy

Floating point geometry fails first through inconsistent decisions, not through
small absolute error. Every plane test needs one shared epsilon policy:

- distance `< -epsilon` is inside
- distance `> epsilon` is outside
- values between are coplanar

Do not mix inclusive/exclusive comparisons casually. The same point must not be
inside for one subsystem and coplanar for another because someone got cute with
`>=`.

For intersections, compute from a canonical direction. If an edge crosses a
plane, pick one input order for interpolation and use it everywhere. Otherwise
adjacent polygons will receive near-identical but non-identical vertices, and
the mesh will die by a thousand decimal-place paper cuts.

### 2. Brushes Are Convex Plane Sets

A brush should be authored and cached as a set of planes, not merely as a
triangle soup. Brush mesh creation is the act of finding valid intersections
between those planes and linking the resulting topology.

Implication for `vg_csg`: add a `ConvexBrush` authored by planes, then derive
polygons, half-edges, bounds, and render buffers from that. Boxes, cylinders,
wedges, stairs, panels, and dome approximations should become constructors for
plane sets where possible.

### 3. Half-Edges Are Working Topology, Not Render Data

Rendering buffers are bad workbenches for CSG. Downstream discussions around
three.js make the same point from another angle: render-oriented geometries
often lose topology, indexing, material grouping, or edit intent.

Use half-edge-like topology while cutting and classifying:

- split one edge and update both adjacent polygons consistently
- preserve polygon identity, plane identity, material, and visibility
- convert to Bevy mesh buffers only at the final boundary

This does not require a literal pointer-heavy half-edge graph forever. The
important thing is that the working structure carries adjacency and polygon
ownership better than a pile of final triangles.

### 4. Boolean Operations Are Routing Tables

The elegant part of the public demo is category routing. A polygon piece is
classified as:

- inside
- outside
- aligned/touching inside
- reverse-aligned/touching outside

Union, intersection, and subtraction can then be expressed through the same
logical OR machinery with category parameter reversal:

- union: `A || B`
- intersection: `!(!A || !B)`
- subtraction: `!(!A || B)`

This matters because subtraction stops being a special monster. It becomes
category routing plus orientation reversal. Less bespoke logic, fewer places
for edge cases to hide and invoice us later.

### 5. Visible Polygons Are Source Polygons With Rejections

The target algorithm should avoid inventing new surface area except at cuts.
It starts with all brush polygons and removes or hides the pieces that the tree
rejects.

Top-level output policy:

- pieces inside the final tree are invisible
- pieces outside the final tree are invisible
- aligned pieces stay visible
- reverse-aligned pieces need winding reversal
- overlapping coplanar surfaces are resolved by tree/order policy so duplicate
  faces do not z-fight

Implication for `vg_csg`: material propagation should be natural if source
polygons remain source polygons. Generated cut faces need explicit policy:
inherit cutter material, source material, or a domain material such as "raw
concrete cut." Do not leave that as accidental array order.

### 6. Bounds Are Part Of The Algorithm

Realtime performance comes from refusing work. Branch bounds and brush bounds
allow early-outs before cutting:

- if polygons cannot touch a branch in union, route to the other side or
  outside immediately
- if polygons are outside either side of an intersection, they are outside
- if subtraction cannot touch the right/cutter branch, categorize only the left

The optimization post reports a major speedup from branch-specific bounds
checks alone. This is not a garnish. Bounds are semantic gates.

Implication for `vg_csg`: every brush, polygon batch, and branch needs cheap
bounds. Later BVH, hashed grid, or sweep-and-prune should be measured against
the built-in tree culling instead of assumed superior by vibes in a lab coat.

The EpiphanyAquarium research memory adds a sharper rule: treat spatial queries
as a demand problem before treating them as an index problem. GigaVoxels uses
visible rays as the oracle for which bricks deserve high resolution. Wronski fog
uses the camera froxel volume as the field where sources are injected once and
sampled many times. Dreams' prototype refined frustum cells, shortened object
lists as cells split, then switched to per-pixel sorted lists that could truncate
at the first solid hit.

For `vg_csg`, that translates to consumer-shaped query frontiers:

- dirty brush movement asks for affected brush pairs, not a world rebuild;
- branch evaluation asks for left/right bounds gates before polygon tests;
- an editor viewport or tile asks for surfaces that can contribute to the
  requested output, not every latent overlap in the level;
- repeated cutters ask for cached pair classifications or brush-frontier batches,
  not a thousand independent list walks.

This does not outlaw BVHs, grids, or sweep-and-prune. It demotes them. They are
candidate storage layouts for a demand frontier, not the central idea. The
central idea is that the request already contains spatial information; make the
kernel exploit that before building a generic little bureaucracy with boxes.

### 7. Realtime Means Incremental, Not Global

The public notes frame the algorithm as fast for updating affected brushes, not
as a promise to rebuild the whole world every frame.

Interactive loop doctrine:

- base brush meshes are cacheable
- when a brush moves, reprocess it and the brushes it touches
- brush independence creates parallel work units
- global rebuild is a debugging tool and benchmark, not the intended editor
  path

Implication for Bevy: store authoring brushes as ECS/editor state, maintain a
dirty set, find touching brushes through bounds/spatial index, then rebuild
only affected render meshes.

### 8. CPU Cache Is A First-Class Design Constraint

Sander explicitly calls out cache-aware rewrite potential and half-edges as the
largest pain point. The three.js discussion arrives at the same pressure from a
different ecosystem: representation conversions, de-indexing, and optimization
passes dominate runtime when data layout is wrong.

Rust doctrine:

- favor index-based arenas over pointer graphs
- store hot plane, vertex, edge, polygon fields in compact arrays
- avoid temporary list storms in the inner classifier
- use flags/ranges/scratch arenas before heap allocation
- keep render buffers as output, not internal truth

The first correct implementation can be plain. The fast implementation should
be built with data layout in mind before the code ossifies into a shapely little
performance tax.

### 9. Generated Mesh Optimization Is A Separate Pass

The public material notes remaining T-junctions, unmerged polygons, and
unoptimized output. Downstream engine discussions agree: CSG and final render
mesh optimization are related but distinct jobs.

Keep the kernel responsible for correct categorized polygon output. Then run
separate passes for:

- polygon merge
- T-junction repair
- vertex welding
- material grouping
- Bevy mesh/index-buffer packing
- normal/UV generation or preservation

Do not contaminate the boolean kernel with every render-buffer cleanup trick.
That way lies soup with comments.

### 10. CSG Belongs Beside Procedural Grammars

BSO3 is useful philosophically: it ports the article algorithm into a growth
grammar modeller by making boolean set operations a node type. For VibeGeometry,
this means the Rust CSG assembler should not replace procedural generation. It
should be one grammar operator among many.

Use CSG for:

- hard architectural voids
- interlocking modular parts
- tunnels, hatches, shafts, sockets, stairs, cut floors
- level-design iteration where negative space is authored

Use procedural fields and meshes for:

- cities, settlements, organic detail, clouds, fields, decoration
- high-frequency surface elaboration
- forms whose identity is distribution rather than volume claim

The level DSL should let these coexist. Boolean nodes cut space; procedural
nodes grow matter and detail around that space.

## Implementation Target For `vg_csg`

Near-term kernel shape:

```text
ConvexBrush {
  planes
  polygons
  half_edges
  vertices
  bounds
}

Branch {
  op: Union | Intersect | Subtract
  left
  right
  bounds
}

Process brush:
  clone or borrow cached base polygons
  classify polygon pieces through root
  mark visible / invisible / reversed
  emit visible pieces into render mesh
```

`vg_csg` now exposes the first version of that public tree surface:

- `CsgOperationType::{Additive, Subtractive, Intersecting}` with parity-tested
  ordinals matching the public API
- `CsgBranchOp::{Addition, Subtraction, Common}` matching the public demo
- `CsgTreeArena`, `CsgTree`, `CsgTreeBranch`, `CsgTreeBrush`, and `CsgNodeId`
- branch child replacement and operation mutation
- brush operation mutation
- compilation from tree intent into the current ordered assembler backend

This is still not the final classifier kernel. It is the API skeleton that lets
grammar and editor code speak in RealtimeCSG-shaped boolean trees while the
backend grows into the category router.

Algorithm migration steps:

1. Add polygon categories and visible/reversed flags to `vg_csg`.
2. Add plane-set brush construction independent of `Aabb`.
3. Add a topology arena with vertices, directed edges, polygons, and polygon
   plane/material ids.
4. Implement polygon-plane split with one epsilon policy and canonical
   intersection direction.
5. Implement brush classification against convex brush planes.
6. Implement boolean branch routing using category list swizzles.
7. Add bounds early-outs before every expensive classification.
8. Add dirty-set and touching-brush discovery.
9. Add a demand-frontier prototype that produces affected brush-pair batches
   from dirty brushes, branch bounds, and requested output scope.
10. Convert categorized visible polygons to Bevy mesh buffers.
11. Benchmark global rebuild, affected-brush rebuild, and worst-case overlap
    scenes before making performance claims.

## Acceptance Tests To Add

- Where `vg_csg` exposes the same behavior as public RealtimeCSG/demo APIs,
  maintain exact observable parity fixtures. These should compare public
  behavior such as category vocabulary, brush plane/polygon counts, split
  counts, visible/reversed semantics, bounds early-outs, and mesh output
  invariants. Do not use vague "looks right" tests where a public seam has a
  countable contract.
- Two overlapping additive boxes produce no duplicate coplanar surface.
- Subtraction is implemented as `!(!A || B)` and matches direct cases.
- Intersection is implemented as `!(!A || !B)` and matches direct cases.
- Reverse-aligned output flips winding exactly once.
- A moved cutter only dirties itself and brushes whose bounds overlap it.
- Polygon split creates shared vertices for adjacent polygons on the same cut.
- Repeated cuts do not create strict-inside triangle centers in the void.
- Material ids survive classification on source faces and follow an explicit
  policy on cut faces.
- Global rebuild and affected-brush rebuild produce identical render buffers
  for a deterministic scene.

## Open Questions

- Should cut faces inherit source material, cutter material, or a DSL-specified
  cut material?
- Should Bevy output preserve large n-gons for later optimization, or triangulate
  at the boundary immediately?
- What is the smallest demand frontier after branch bounds: dirty affected
  brush pairs, viewport/tile-frontier batches, pair-classification caches, or no
  extra structure until profiling says tree culling is insufficient?
- If a frontier needs storage, which layout wins under dirty rebuild timings:
  loose grid, BVH, sweep-and-prune, pair cache, or sorted frontier batches?
- How much topology should live in reusable library code versus Bevy ECS
  components?
