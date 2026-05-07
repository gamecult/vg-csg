# CSG Performance Fixtures

The timing surface is a fixture, not a victory poster.

Run our current kernel:

```powershell
.\tools\run_csg_perf.ps1
```

The script builds `vg_csg` in release mode and writes JSONL to
`experiments/generated/csg-perf-latest.jsonl`. Each record carries:

- `kernel`
- `scenario`
- `brushes`
- warmup and measured iteration counts
- `mean_ns`, `min_ns`, `p50_ns`, `p95_ns`, `max_ns`
- emitted triangle, fragment, and warning counts
- demand-frontier `candidate_pairs` and `rejected_pairs`

The `vg_csg` fixture measures:

- `stable`: cached output after warmup
- `dirty`: full rebuild through the current ordered assembler
- `routed`: experimental surface-router rebuild, falling back to the ordered
  assembler outside its current single-source, single-candidate subtractive
  convex contract. The fixture only emits routed records for supported cases.

`Assembler` caches evaluated output by generation, so repeated `build()` calls
on an unchanged graph return the cached mesh. Dirty and routed generations
still rebuild.

## Scenarios

- `single_center_cut`: the current exact centered box-subtraction parity seam.
- `room_grid_8x8_doors`: repeated additive floors/walls and repeated door
  voids, matching the level-editing workload we care about.
- `rotated_cut_stack_64`: one large slab cut by many oriented boxes.
- `common_box_chain_64`: repeated convex intersections, exercising `Common`
  branch behavior through the ordered assembler.
- `distant_cutters_512`: many irrelevant cutters, proving bounds early-outs
  stay cheap.

## Reference Kernel Slot

The same wrapper can append reference timings:

```powershell
$env:VIBEGEOMETRY_REFERENCE_CSG_PERF = "path\to\reference-perf.exe"
.\tools\run_csg_perf.ps1
```

or:

```powershell
.\tools\run_csg_perf.ps1 -ReferenceCommand "path\to\reference-perf.exe"
```

To run the native RealtimeCSG C++ plugin directly through the public P/Invoke
surface mirrored by its managed Unity library:

```powershell
.\tools\run_csg_perf.ps1 -UseRealtimeCsgCpp
```

To test the native bridge without running the full timing suite:

```powershell
.\tools\run_realtimecsg_cpp_perf.ps1 -OutputPath .\experiments\generated\realtimecsg-cpp-health-latest.jsonl -Health
```

That path builds `tools/realtimecsg_native_bridge`, copies
`RealtimeCSG[1_559].dll` beside the bridge executable, calls only exported
functions already declared by the plugin's managed P/Invoke layer, and writes
native timing records to `experiments/generated/realtimecsg-cpp-perf-latest.jsonl`.
It fails closed if the native plugin emits no mesh descriptions; zero-geometry
timings are poison, not data.

Current direct-DLL status: the bridge successfully loads the native plugin,
creates brush meshes, brushes, and trees, verifies bounds, outlines, raycasts,
and now extracts generated mesh descriptions. The fragile part was construction
discipline: build child nodes before the tree, insert them through the public
Foundation range-insert path, and avoid eagerly setting default brush operation
or flag state.

The reference executable must emit JSONL with the same scenario names. Until
that harness exists, the script appends a `kernel=reference,status=missing`
record instead of faking a comparison. Sad, but hygienic.

The public `LogicalError/Realtime-CSG-demo` source still exposes the relevant
algorithmic surface: half-edge control meshes, polygon bounds, visible flags,
inside/outside/aligned/reverse-aligned categories, crossing polygon splits, and
logical routing tables. Use it as readable doctrine. Use the native bridge as
the performance target.

`vg_csg` now reports the first demand-frontier counters. These are not yet a
new kernel path; they expose the ordered brush stream as affected source/operator
pairs and rejected bounds pairs so future router/index work can prove it is
refusing real work rather than merely rearranging it.

The first routed surface experiment is intentionally narrow. It cleans a single
subtractive convex cut into boundary polygons instead of closed fragment boxes,
but dense repeated cutters currently create surface-piece explosion. Keep dense
rotated subtraction on the ordered kernel until the category router has compact
frontier batching and scratch storage.

Current routed lesson: for `single_center_cut`, routed output emits 48 boundary
triangles instead of the fragment carver's 72 closed-fragment triangles. That is
cleaner surface output, not yet faster output. The implementation still moves
owned polygon vectors around, so it is a topology proof before it is a hot path.

Dense-kernel regression guardrail: `rotated_cut_stack_64` is pinned by tests at
280 fragments, 3404 triangles, 804 candidate pairs, and 9417 rejected pairs.
This protects against "optimizations" that alter ordered CSG semantics while
looking faster. Output-contract changes need to be explicit.

Realtime editing seam: `Assembler` can now mutate brush primitives and
operations by `BrushId`, invalidating cached output and incrementing generation.
`DirtyDemandFrontier` computes the conservative ordered suffix after the first
dirty brush. The prefix before that index is the cacheable region; the suffix is
the live region because ordered CSG decisions propagate forward. This is still a
planning surface, not an incremental mesh rebuild yet.

## Latest Local Baseline

Captured on 2026-05-07 with `.\tools\run_csg_perf.ps1 -UseRealtimeCsgCpp`:

This baseline predates the `candidate_pairs`/`rejected_pairs` fields. New
fixture runs include them; keep the old block until a fresh C++ comparison is
captured rather than mixing timing environments.

```jsonl
{"kernel":"vg_csg","mode":"stable","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":948,"min_ns":800,"p50_ns":900,"p95_ns":1100,"max_ns":1200,"triangles":72,"fragments":6,"warnings":0}
{"kernel":"vg_csg","mode":"dirty","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":6123,"min_ns":4300,"p50_ns":5500,"p95_ns":6000,"max_ns":44700,"triangles":72,"fragments":6,"warnings":0}
{"kernel":"vg_csg","mode":"stable","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":85546,"min_ns":66100,"p50_ns":84900,"p95_ns":103300,"max_ns":114800,"triangles":3072,"fragments":256,"warnings":0}
{"kernel":"vg_csg","mode":"dirty","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":322628,"min_ns":231400,"p50_ns":301600,"p95_ns":428500,"max_ns":841700,"triangles":3072,"fragments":256,"warnings":0}
{"kernel":"vg_csg","mode":"stable","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":162057,"min_ns":107500,"p50_ns":139600,"p95_ns":271700,"max_ns":518200,"triangles":3404,"fragments":280,"warnings":0}
{"kernel":"vg_csg","mode":"dirty","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":4835134,"min_ns":2799000,"p50_ns":4260700,"p95_ns":7080200,"max_ns":10841300,"triangles":3404,"fragments":280,"warnings":0}
{"kernel":"vg_csg","mode":"stable","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":937,"min_ns":700,"p50_ns":900,"p95_ns":1100,"max_ns":9700,"triangles":45,"fragments":1,"warnings":0}
{"kernel":"vg_csg","mode":"dirty","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":700876,"min_ns":396000,"p50_ns":662400,"p95_ns":963600,"max_ns":1582700,"triangles":45,"fragments":1,"warnings":0}
{"kernel":"vg_csg","mode":"stable","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":278,"min_ns":200,"p50_ns":300,"p95_ns":300,"max_ns":400,"triangles":12,"fragments":1,"warnings":0}
{"kernel":"vg_csg","mode":"dirty","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":4762,"min_ns":4500,"p50_ns":4700,"p95_ns":5000,"max_ns":5200,"triangles":12,"fragments":1,"warnings":0}
{"kernel":"realtime_csg_cpp","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":7900,"min_ns":5900,"p50_ns":6300,"p95_ns":12100,"max_ns":16400,"triangles":72,"vertices":144,"mesh_descriptions":3}
{"kernel":"realtime_csg_cpp","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":775500,"min_ns":376100,"p50_ns":670500,"p95_ns":1451600,"max_ns":2108000,"triangles":12800,"vertices":20992,"mesh_descriptions":4}
{"kernel":"realtime_csg_cpp","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":154300,"min_ns":106500,"p50_ns":132900,"p95_ns":240000,"max_ns":380100,"triangles":3498,"vertices":5102,"mesh_descriptions":4}
{"kernel":"realtime_csg_cpp","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":64000,"min_ns":45000,"p50_ns":61400,"p95_ns":90500,"max_ns":224800,"triangles":1536,"vertices":2412,"mesh_descriptions":4}
{"kernel":"realtime_csg_cpp","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":740400,"min_ns":340900,"p50_ns":641200,"p95_ns":1403100,"max_ns":2258600,"triangles":6180,"vertices":12360,"mesh_descriptions":4}
```

Read these as smoke timings, not final benchmark gospel. Stable unchanged
output is no longer embarrassing. Dirty rebuilds are now explicit: box-heavy
dirty paths are respectable, distant bounds rejection is excellent, and dense
rotated subtraction is still the open wound. The category-router kernel remains
the next architectural move; cached output is a guardrail, not a replacement
for the real engine.
