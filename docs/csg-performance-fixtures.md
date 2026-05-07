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

The `vg_csg` fixture measures the stable-tree path after warmup. `Assembler`
caches evaluated output by generation, so repeated `build()` calls on an
unchanged graph return the cached mesh. Dirty generations still rebuild.

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
{"kernel":"vg_csg","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":743,"min_ns":600,"p50_ns":700,"p95_ns":800,"max_ns":1300,"triangles":72,"fragments":6,"warnings":0}
{"kernel":"vg_csg","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":106331,"min_ns":78000,"p50_ns":89500,"p95_ns":161600,"max_ns":328900,"triangles":3072,"fragments":256,"warnings":0}
{"kernel":"vg_csg","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":141673,"min_ns":117600,"p50_ns":128900,"p95_ns":185900,"max_ns":296900,"triangles":3404,"fragments":280,"warnings":0}
{"kernel":"vg_csg","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":940,"min_ns":800,"p50_ns":900,"p95_ns":1000,"max_ns":1000,"triangles":45,"fragments":1,"warnings":0}
{"kernel":"vg_csg","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":717,"min_ns":500,"p50_ns":500,"p95_ns":600,"max_ns":12000,"triangles":12,"fragments":1,"warnings":0}
{"kernel":"realtime_csg_cpp","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":9500,"min_ns":6600,"p50_ns":7800,"p95_ns":13500,"max_ns":24900,"triangles":72,"vertices":144,"mesh_descriptions":3}
{"kernel":"realtime_csg_cpp","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":1167600,"min_ns":493200,"p50_ns":944700,"p95_ns":2337700,"max_ns":3141500,"triangles":12800,"vertices":20992,"mesh_descriptions":4}
{"kernel":"realtime_csg_cpp","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":272400,"min_ns":122800,"p50_ns":200600,"p95_ns":519800,"max_ns":1843100,"triangles":3498,"vertices":5102,"mesh_descriptions":4}
{"kernel":"realtime_csg_cpp","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":63600,"min_ns":44900,"p50_ns":59100,"p95_ns":88000,"max_ns":259700,"triangles":1536,"vertices":2412,"mesh_descriptions":4}
{"kernel":"realtime_csg_cpp","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":573000,"min_ns":316200,"p50_ns":423000,"p95_ns":1235000,"max_ns":2374500,"triangles":6180,"vertices":12360,"mesh_descriptions":4}
```

Read these as smoke timings, not final benchmark gospel. Stable unchanged
output is no longer embarrassing. Dirty rebuilds for dense rotated subtraction
still need the category-router kernel; cached output is a guardrail, not a
replacement for the real engine.
