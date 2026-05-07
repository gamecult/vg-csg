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

That path builds `tools/realtimecsg_native_bridge`, copies
`RealtimeCSG[1_559].dll` beside the bridge executable, calls only exported
functions already declared by the plugin's managed P/Invoke layer, and writes
native timing records to `experiments/generated/realtimecsg-cpp-perf-latest.jsonl`.
It fails closed if the native plugin emits no mesh descriptions; zero-geometry
timings are poison, not data.

Current direct-DLL status: the bridge successfully loads the native plugin and
creates brush meshes, brushes, and trees through the mirrored P/Invoke surface,
but outside the Unity-hosted plugin lifecycle the native query currently returns
success with `meshDescriptionCount = 0` even for a single additive cube. The
failure is useful: it proves the exported heart can be reached directly, and it
also proves we still need the missing initialization or lifecycle surface before
using the C++ plugin as a timing oracle.

The reference executable must emit JSONL with the same scenario names. Until
that harness exists, the script appends a `kernel=reference,status=missing`
record instead of faking a comparison. Sad, but hygienic.

The public `LogicalError/Realtime-CSG-demo` source still exposes the relevant
algorithmic surface: half-edge control meshes, polygon bounds, visible flags,
inside/outside/aligned/reverse-aligned categories, crossing polygon splits, and
logical routing tables. Use it as readable doctrine. Use the native bridge as
the performance target once the missing initialization path is identified.

## Latest Local Baseline

Captured on 2026-05-07 with `.\tools\run_csg_perf.ps1`:

```jsonl
{"kernel":"vg_csg","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":37314,"min_ns":27300,"p50_ns":32400,"p95_ns":45600,"max_ns":203900,"triangles":72,"fragments":6,"warnings":0}
{"kernel":"vg_csg","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":8413054,"min_ns":4710000,"p50_ns":8020100,"p95_ns":11060600,"max_ns":18551700,"triangles":3072,"fragments":256,"warnings":0}
{"kernel":"vg_csg","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":14952303,"min_ns":8627100,"p50_ns":14678000,"p95_ns":19088600,"max_ns":28753400,"triangles":3404,"fragments":280,"warnings":0}
{"kernel":"vg_csg","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":891484,"min_ns":616700,"p50_ns":823700,"p95_ns":1312000,"max_ns":3726400,"triangles":45,"fragments":1,"warnings":0}
{"kernel":"vg_csg","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":552914,"min_ns":459100,"p50_ns":471500,"p95_ns":775900,"max_ns":1421600,"triangles":12,"fragments":1,"warnings":0}
{"status":"missing","reason":"Set VIBEGEOMETRY_REFERENCE_CSG_PERF or pass -ReferenceCommand with an executable that emits the same JSONL scenario records.","kernel":"reference"}
```

Read these as smoke timings, not final benchmark gospel. The fixtures are stable
enough to catch regressions and compare against a reference harness once wired.
