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

## Latest Local Baseline

Captured on 2026-05-07 with `.\tools\run_csg_perf.ps1 -UseRealtimeCsgCpp`:

```jsonl
{"kernel":"vg_csg","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":30817,"min_ns":21200,"p50_ns":25300,"p95_ns":41600,"max_ns":144900,"triangles":72,"fragments":6,"warnings":0}
{"kernel":"vg_csg","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":6558703,"min_ns":4415500,"p50_ns":6232300,"p95_ns":8580100,"max_ns":10348300,"triangles":3072,"fragments":256,"warnings":0}
{"kernel":"vg_csg","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":12701656,"min_ns":9057100,"p50_ns":12045700,"p95_ns":17285200,"max_ns":29792900,"triangles":3404,"fragments":280,"warnings":0}
{"kernel":"vg_csg","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":890965,"min_ns":617100,"p50_ns":871800,"p95_ns":1214900,"max_ns":1989700,"triangles":45,"fragments":1,"warnings":0}
{"kernel":"vg_csg","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":635082,"min_ns":460900,"p50_ns":578300,"p95_ns":873800,"max_ns":1361200,"triangles":12,"fragments":1,"warnings":0}
{"kernel":"realtime_csg_cpp","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":8300,"min_ns":5500,"p50_ns":7300,"p95_ns":11700,"max_ns":22000,"triangles":72,"vertices":144,"mesh_descriptions":3}
{"kernel":"realtime_csg_cpp","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":530000,"min_ns":379700,"p50_ns":446300,"p95_ns":915500,"max_ns":1537100,"triangles":12800,"vertices":20992,"mesh_descriptions":4}
{"kernel":"realtime_csg_cpp","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":112800,"min_ns":93600,"p50_ns":99900,"p95_ns":143600,"max_ns":314200,"triangles":3498,"vertices":5102,"mesh_descriptions":4}
{"kernel":"realtime_csg_cpp","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":76800,"min_ns":57200,"p50_ns":64400,"p95_ns":104900,"max_ns":428700,"triangles":1536,"vertices":2412,"mesh_descriptions":4}
{"kernel":"realtime_csg_cpp","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":324600,"min_ns":240400,"p50_ns":263400,"p95_ns":534100,"max_ns":1530400,"triangles":6180,"vertices":12360,"mesh_descriptions":4}
```

Read these as smoke timings, not final benchmark gospel. They are already sharp
enough to show the target: our current ordered Rust backend is not yet competing
with the C++ classifier/router on mesh-heavy workloads.
