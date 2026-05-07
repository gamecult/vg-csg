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
the live region because ordered CSG decisions propagate forward. The first
implementation now uses that boundary for box-only and general convex prefix
checkpoints. The category-router path still needs the same treatment.

Incremental edit seam: ordered CSG now stores prefix checkpoints after each
brush for both the box and general convex builders. A tail edit can resume from
the checkpoint immediately before the dirty brush and replay only the live
suffix. The result is parity-checked against a full rebuild. Cache validity is
tracked as a prefix boundary so repeated edits do not deep-copy stale
checkpoints. Final mesh emission is still whole-output.

Mesh reuse seam: incremental rebuilds now keep the previous generation's cached
mesh and reuse it only when suffix replay proves no geometry was touched. This
is intentionally exact and conservative: distant rejected edits can skip mesh
emission, while candidate cuts still emit the full output mesh. The report
exposes `reused_mesh` so the fixture and tests do not infer reuse from timing.

Benchmark correction: `rebuild()` must bypass checkpoint caches. An earlier
fixture pass accidentally let the dirty box baseline travel through cached box
state, which made incremental box edits look better than they were. The live
policy is stricter: box dirty edits use the direct builder unless a future real
mesh patcher proves otherwise; general convex dirty edits keep the valid-prefix
checkpoint path.

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

## Incremental Dirty Smoke Run

Captured on 2026-05-07 with
`cargo run -p vg_csg --release --example csg_perf_fixture`. Dirty modes mutate
the tail brush each iteration. `incremental_dirty` uses prefix checkpoints and
falls back to full replay only when no valid checkpoint exists.

```jsonl
{"kernel":"vg_csg","mode":"stable","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":506,"min_ns":500,"p50_ns":500,"p95_ns":500,"max_ns":800,"triangles":72,"fragments":6,"warnings":0,"candidate_pairs":1,"rejected_pairs":0}
{"kernel":"vg_csg","mode":"dirty","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":3937,"min_ns":3700,"p50_ns":3800,"p95_ns":4100,"max_ns":12700,"triangles":72,"fragments":6,"warnings":0,"candidate_pairs":1,"rejected_pairs":0}
{"kernel":"vg_csg","mode":"incremental_dirty","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":4204,"min_ns":4000,"p50_ns":4100,"p95_ns":5100,"max_ns":6000,"triangles":72,"fragments":6,"warnings":0,"candidate_pairs":1,"rejected_pairs":0}
{"kernel":"vg_csg","mode":"stable","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":68625,"min_ns":41300,"p50_ns":62600,"p95_ns":99600,"max_ns":150000,"triangles":3072,"fragments":256,"warnings":0,"candidate_pairs":64,"rejected_pairs":8128}
{"kernel":"vg_csg","mode":"dirty","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":817095,"min_ns":428800,"p50_ns":664300,"p95_ns":1392000,"max_ns":3642000,"triangles":3108,"fragments":259,"warnings":0,"candidate_pairs":65,"rejected_pairs":8127}
{"kernel":"vg_csg","mode":"incremental_dirty","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":104501,"min_ns":45200,"p50_ns":61400,"p95_ns":283600,"max_ns":374000,"triangles":3108,"fragments":259,"warnings":0,"candidate_pairs":65,"rejected_pairs":8127}
{"kernel":"vg_csg","mode":"stable","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":14776,"min_ns":9500,"p50_ns":10300,"p95_ns":14200,"max_ns":233700,"triangles":3404,"fragments":280,"warnings":0,"candidate_pairs":804,"rejected_pairs":9417}
{"kernel":"vg_csg","mode":"dirty","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":3490834,"min_ns":2233100,"p50_ns":3133400,"p95_ns":5120800,"max_ns":6423700,"triangles":3404,"fragments":280,"warnings":0,"candidate_pairs":805,"rejected_pairs":9416}
{"kernel":"vg_csg","mode":"incremental_dirty","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":415318,"min_ns":242200,"p50_ns":305100,"p95_ns":874600,"max_ns":1777700,"triangles":3404,"fragments":280,"warnings":0,"candidate_pairs":805,"rejected_pairs":9416}
{"kernel":"vg_csg","mode":"stable","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":651,"min_ns":500,"p50_ns":600,"p95_ns":700,"max_ns":800,"triangles":45,"fragments":1,"warnings":0,"candidate_pairs":63,"rejected_pairs":0}
{"kernel":"vg_csg","mode":"dirty","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":488343,"min_ns":360200,"p50_ns":394100,"p95_ns":832100,"max_ns":1936700,"triangles":45,"fragments":1,"warnings":0,"candidate_pairs":63,"rejected_pairs":0}
{"kernel":"vg_csg","mode":"incremental_dirty","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":10150,"min_ns":9500,"p50_ns":9600,"p95_ns":13100,"max_ns":17200,"triangles":45,"fragments":1,"warnings":0,"candidate_pairs":63,"rejected_pairs":0}
{"kernel":"vg_csg","mode":"stable","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":218,"min_ns":200,"p50_ns":200,"p95_ns":300,"max_ns":400,"triangles":12,"fragments":1,"warnings":0,"candidate_pairs":0,"rejected_pairs":512}
{"kernel":"vg_csg","mode":"dirty","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":110825,"min_ns":35700,"p50_ns":52200,"p95_ns":115600,"max_ns":2843900,"triangles":12,"fragments":1,"warnings":0,"candidate_pairs":0,"rejected_pairs":512}
{"kernel":"vg_csg","mode":"incremental_dirty","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":2856,"min_ns":2400,"p50_ns":2500,"p95_ns":3600,"max_ns":4100,"triangles":12,"fragments":1,"warnings":0,"candidate_pairs":0,"rejected_pairs":512}
```

Tail-edit lesson: prefix caching cuts the room-grid dirty mean by roughly 7.8x,
the rotated-stack dirty mean by roughly 8.4x, the common-box chain by roughly
48x, and the distant-cutter case by roughly 39x in this run. A follow-up mesh
reuse seam pushed distant rejected edits into low microseconds by reusing the
previous mesh when no geometry was touched. The single-cut case is still
slightly slower because cache machinery dominates. The next hard target is
dirty mesh range patching for candidate edits; replaying less CSG work is not
enough when every real cut still serializes the whole result mesh.

Corrected lesson after separating `build()` from `rebuild()`: box candidate
edits are currently better served by the direct box builder, not checkpoint
replay. The checkpoint path still wins hard for general convex tail edits:
`rotated_cut_stack_64` remains an order-of-magnitude style win in smoke runs,
and `common_box_chain_64` stays far below full dirty rebuild. Rejected general
convex suffixes can expose `reused_mesh:true`; the fixture includes
`distant_oriented_cutters_128` for that case.
