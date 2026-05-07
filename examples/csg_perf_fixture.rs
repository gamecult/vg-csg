use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use bevy_math::{Quat, Vec3};
use vg_csg::{Aabb, Assembler, BrushOp, MaterialId, Primitive};

const WARMUP_ITERS: usize = 8;
const MEASURE_ITERS: usize = 64;

fn main() {
    let cases = [
        PerfCase {
            name: "single_center_cut",
            brushes: 2,
            build: single_center_cut,
        },
        PerfCase {
            name: "room_grid_8x8_doors",
            brushes: 192,
            build: room_grid_8x8_doors,
        },
        PerfCase {
            name: "rotated_cut_stack_64",
            brushes: 65,
            build: rotated_cut_stack_64,
        },
        PerfCase {
            name: "common_box_chain_64",
            brushes: 64,
            build: common_box_chain_64,
        },
        PerfCase {
            name: "distant_cutters_512",
            brushes: 513,
            build: distant_cutters_512,
        },
    ];

    for case in cases {
        run_case(case);
    }
}

#[derive(Clone, Copy)]
struct PerfCase {
    name: &'static str,
    brushes: usize,
    build: fn() -> Assembler,
}

fn run_case(case: PerfCase) {
    let assembler = (case.build)();
    let warmup = assembler.build();
    black_box(warmup.mesh.triangle_count());

    for _ in 0..WARMUP_ITERS {
        let output = assembler.build();
        black_box(output.mesh.triangle_count());
    }

    let mut timings = Vec::with_capacity(MEASURE_ITERS);
    let mut triangles = 0;
    let mut fragments = 0;
    let mut warnings = 0;

    for _ in 0..MEASURE_ITERS {
        let start = Instant::now();
        let output = assembler.build();
        let elapsed = start.elapsed();
        triangles = output.mesh.triangle_count();
        fragments = output.report.emitted_convex_fragments;
        warnings = output.report.warnings.len();
        black_box((triangles, fragments, warnings));
        timings.push(elapsed);
    }

    timings.sort_unstable();
    let total = timings
        .iter()
        .fold(Duration::ZERO, |sum, value| sum + *value);
    let mean_ns = total.as_nanos() / timings.len() as u128;
    let min_ns = timings.first().map_or(0, Duration::as_nanos);
    let max_ns = timings.last().map_or(0, Duration::as_nanos);
    let p50_ns = percentile_ns(&timings, 50);
    let p95_ns = percentile_ns(&timings, 95);

    println!(
        "{{\"kernel\":\"vg_csg\",\"scenario\":\"{}\",\"brushes\":{},\"iterations\":{},\"warmup_iterations\":{},\"mean_ns\":{},\"min_ns\":{},\"p50_ns\":{},\"p95_ns\":{},\"max_ns\":{},\"triangles\":{},\"fragments\":{},\"warnings\":{}}}",
        case.name,
        case.brushes,
        MEASURE_ITERS,
        WARMUP_ITERS,
        mean_ns,
        min_ns,
        p50_ns,
        p95_ns,
        max_ns,
        triangles,
        fragments,
        warnings
    );
}

fn percentile_ns(values: &[Duration], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) * percentile) / 100;
    values[index].as_nanos()
}

fn single_center_cut() -> Assembler {
    let mut asm = Assembler::new();
    asm.solid_box(
        "source",
        Aabb::from_center_size(Vec3::ZERO, Vec3::splat(4.0)),
        MaterialId(1),
    );
    asm.cut_box("void", Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0)));
    asm
}

fn room_grid_8x8_doors() -> Assembler {
    let mut asm = Assembler::new();
    let wall = MaterialId(1);
    let floor = MaterialId(2);
    let cell = 6.0;

    for y in 0..8 {
        for x in 0..8 {
            let center = Vec3::new((x as f32 - 3.5) * cell, (y as f32 - 3.5) * cell, 0.0);
            asm.solid_box(
                format!("floor_{x}_{y}"),
                Aabb::from_center_size(
                    center + Vec3::new(0.0, 0.0, -0.1),
                    Vec3::new(5.6, 5.6, 0.2),
                ),
                floor,
            );
            asm.solid_box(
                format!("north_wall_{x}_{y}"),
                Aabb::from_center_size(
                    center + Vec3::new(0.0, 2.8, 1.5),
                    Vec3::new(5.6, 0.25, 3.0),
                ),
                wall,
            );
            asm.cut_box(
                format!("door_{x}_{y}"),
                Aabb::from_center_size(center + Vec3::new(0.0, 2.8, 1.0), Vec3::new(1.2, 0.5, 2.0)),
            );
        }
    }

    asm
}

fn rotated_cut_stack_64() -> Assembler {
    let mut asm = Assembler::new();
    asm.solid_box(
        "slab",
        Aabb::from_center_size(Vec3::ZERO, Vec3::new(32.0, 32.0, 4.0)),
        MaterialId(1),
    );

    for index in 0..64 {
        let angle = index as f32 * 0.173;
        let radius = 11.0 + (index % 7) as f32 * 0.35;
        let center = Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.0);
        asm.cut_oriented_box(
            format!("rotated_void_{index}"),
            center,
            Vec3::new(1.0 + (index % 3) as f32 * 0.3, 8.0, 5.0),
            Quat::from_rotation_z(angle),
        );
    }

    asm
}

fn common_box_chain_64() -> Assembler {
    let mut asm = Assembler::new();
    asm.solid_box(
        "source",
        Aabb::from_center_size(Vec3::ZERO, Vec3::new(32.0, 16.0, 8.0)),
        MaterialId(1),
    );

    for index in 1..64 {
        let t = index as f32 / 63.0;
        let center = Vec3::new(
            (t - 0.5) * 6.0,
            (t * std::f32::consts::TAU).sin() * 1.5,
            0.0,
        );
        asm.add_brush(
            format!("common_{index}"),
            BrushOp::Intersect,
            Primitive::OrientedBox {
                center,
                size: Vec3::new(30.0 - t * 8.0, 14.0, 7.0),
                rotation: Quat::from_rotation_z(t * 0.2),
            },
            MaterialId(1),
        );
    }

    asm
}

fn distant_cutters_512() -> Assembler {
    let mut asm = Assembler::new();
    asm.solid_box(
        "source",
        Aabb::from_center_size(Vec3::ZERO, Vec3::splat(8.0)),
        MaterialId(1),
    );

    for index in 0..512 {
        let row = index / 32;
        let col = index % 32;
        asm.cut_box(
            format!("far_void_{index}"),
            Aabb::from_center_size(
                Vec3::new(1000.0 + col as f32 * 4.0, 1000.0 + row as f32 * 4.0, 0.0),
                Vec3::splat(1.0),
            ),
        );
    }

    asm
}
