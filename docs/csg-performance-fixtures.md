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

The reference executable must emit JSONL with the same scenario names. Until
that harness exists, the script appends a `kernel=reference,status=missing`
record instead of faking a comparison. Sad, but hygienic.

The public `LogicalError/Realtime-CSG-demo` source already exposes the relevant
algorithmic surface: half-edge control meshes, polygon bounds, visible flags,
inside/outside/aligned/reverse-aligned categories, crossing polygon splits, and
logical routing tables. A reference benchmark should call that public demo
kernel headlessly or through a small public-source harness, not the closed Unity
native plugin.

## Latest Local Baseline

Captured on 2026-05-07 with `.\tools\run_csg_perf.ps1`:

```jsonl
{"kernel":"vg_csg","scenario":"single_center_cut","brushes":2,"iterations":64,"warmup_iterations":8,"mean_ns":34295,"min_ns":21200,"p50_ns":35400,"p95_ns":43000,"max_ns":52500,"triangles":72,"fragments":6,"warnings":0}
{"kernel":"vg_csg","scenario":"room_grid_8x8_doors","brushes":192,"iterations":64,"warmup_iterations":8,"mean_ns":11564459,"min_ns":4988300,"p50_ns":9626400,"p95_ns":20738900,"max_ns":33797500,"triangles":3072,"fragments":256,"warnings":0}
{"kernel":"vg_csg","scenario":"rotated_cut_stack_64","brushes":65,"iterations":64,"warmup_iterations":8,"mean_ns":12646675,"min_ns":8271900,"p50_ns":12512900,"p95_ns":16605400,"max_ns":19622900,"triangles":3404,"fragments":280,"warnings":0}
{"kernel":"vg_csg","scenario":"common_box_chain_64","brushes":64,"iterations":64,"warmup_iterations":8,"mean_ns":829393,"min_ns":609900,"p50_ns":751800,"p95_ns":1082100,"max_ns":1478800,"triangles":45,"fragments":1,"warnings":0}
{"kernel":"vg_csg","scenario":"distant_cutters_512","brushes":513,"iterations":64,"warmup_iterations":8,"mean_ns":534860,"min_ns":461700,"p50_ns":469400,"p95_ns":754700,"max_ns":1031900,"triangles":12,"fragments":1,"warnings":0}
{"status":"missing","reason":"Set VIBEGEOMETRY_REFERENCE_CSG_PERF or pass -ReferenceCommand with an executable that emits the same JSONL scenario records.","kernel":"reference"}
```

Read these as smoke timings, not final benchmark gospel. The fixtures are stable
enough to catch regressions and compare against a reference harness once wired.
