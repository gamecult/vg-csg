use std::cell::RefCell;

use bevy_math::Vec3;

use crate::{
    Aabb, Brush, BrushId, BrushOp, ConvexPolygon, ConvexSolid, MaterialId, Primitive, TriangleMesh,
    append_cylinder_z, append_dome_cap_z, append_floret_arm,
    primitives::{DomeCapZSpec, FloretArmSpec},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildWarning {
    SubtractIgnoredForNonBox { brush: String },
    IntersectIgnoredForNonBox { brush: String },
}

#[derive(Clone, Debug, Default)]
pub struct BuildReport {
    pub input_brushes: usize,
    pub emitted_convex_fragments: usize,
    pub operator_brushes: usize,
    pub candidate_pairs: usize,
    pub rejected_pairs: usize,
    pub warnings: Vec<BuildWarning>,
}

#[derive(Clone, Debug)]
pub struct BuildOutput {
    pub mesh: TriangleMesh,
    pub report: BuildReport,
    pub generation: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Assembler {
    brushes: Vec<Brush>,
    compiled: Vec<CompiledBrush>,
    cache: RefCell<Option<BuildOutput>>,
    next_id: u32,
    generation: u64,
}

#[derive(Clone, Debug)]
struct CompiledBrush {
    op: BrushOp,
    material: MaterialId,
    bounds: Aabb,
    geometry: CompiledGeometry,
    name: String,
}

#[derive(Clone, Debug)]
enum CompiledGeometry {
    Box(Aabb),
    Convex(ConvexSolid),
    CylinderZ {
        center: Vec3,
        radius: f32,
        depth: f32,
        segments: usize,
    },
    DomeCapZ {
        center: Vec3,
        radius: f32,
        height: f32,
        rings: usize,
        segments: usize,
    },
    FloretArm {
        anchor: Vec3,
        direction: Vec3,
        length: f32,
        root_width: f32,
        tip_width: f32,
        thickness: f32,
        tip_lift: f32,
    },
}

impl Assembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn brushes(&self) -> &[Brush] {
        &self.brushes
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn add_brush(
        &mut self,
        name: impl Into<String>,
        op: BrushOp,
        primitive: Primitive,
        material: MaterialId,
    ) -> BrushId {
        let id = BrushId(self.next_id);
        self.next_id += 1;
        let brush = Brush::new(id, name, op, primitive, material);
        self.compiled.push(CompiledBrush::from_brush(&brush));
        self.brushes.push(brush);
        self.generation = self.generation.wrapping_add(1);
        self.cache.borrow_mut().take();
        id
    }

    pub fn solid_box(
        &mut self,
        name: impl Into<String>,
        bounds: Aabb,
        material: MaterialId,
    ) -> BrushId {
        self.add_brush(name, BrushOp::Add, Primitive::Box { bounds }, material)
    }

    pub fn cut_box(&mut self, name: impl Into<String>, bounds: Aabb) -> BrushId {
        self.add_brush(
            name,
            BrushOp::Subtract,
            Primitive::Box { bounds },
            MaterialId::default(),
        )
    }

    pub fn solid_oriented_box(
        &mut self,
        name: impl Into<String>,
        center: Vec3,
        size: Vec3,
        rotation: bevy_math::Quat,
        material: MaterialId,
    ) -> BrushId {
        self.add_brush(
            name,
            BrushOp::Add,
            Primitive::OrientedBox {
                center,
                size,
                rotation,
            },
            material,
        )
    }

    pub fn cut_oriented_box(
        &mut self,
        name: impl Into<String>,
        center: Vec3,
        size: Vec3,
        rotation: bevy_math::Quat,
    ) -> BrushId {
        self.add_brush(
            name,
            BrushOp::Subtract,
            Primitive::OrientedBox {
                center,
                size,
                rotation,
            },
            MaterialId::default(),
        )
    }

    pub fn build(&self) -> BuildOutput {
        if let Some(output) = self
            .cache
            .borrow()
            .as_ref()
            .filter(|output| output.generation == self.generation)
        {
            return output.clone();
        }

        let output = self.build_uncached();
        *self.cache.borrow_mut() = Some(output.clone());
        output
    }

    pub fn rebuild(&self) -> BuildOutput {
        self.build_uncached()
    }

    pub fn rebuild_routed_surfaces(&self) -> BuildOutput {
        self.try_build_routed_surfaces()
            .unwrap_or_else(|| self.build_uncached())
    }

    pub fn supports_routed_surfaces(&self) -> bool {
        self.supports_routed_surface_subset()
    }

    fn try_build_routed_surfaces(&self) -> Option<BuildOutput> {
        if !self.supports_routed_surface_subset() {
            return None;
        }

        let mut report = BuildReport {
            input_brushes: self.brushes.len(),
            ..BuildReport::default()
        };
        let mut source = None::<ConvexSolid>;
        let mut surfaces = Vec::<ConvexPolygon>::new();
        let mut previous_cutters = Vec::<ConvexSolid>::new();

        for brush in &self.compiled {
            match brush.op {
                BrushOp::Add => {
                    if source.is_some() {
                        return None;
                    }
                    let solid = brush.convex_cutter()?;
                    surfaces = solid.polygons.clone();
                    source = Some(solid);
                }
                BrushOp::Subtract => {
                    report.operator_brushes += 1;
                    let source_solid = source.as_ref()?;
                    if !source_solid.bounds.intersects(brush.bounds) {
                        report.rejected_pairs += 1;
                        continue;
                    }
                    let cutter = brush.convex_cutter()?;
                    report.candidate_pairs += 1;
                    surfaces = ConvexSolid::route_polygons_outside_of(surfaces, &cutter);

                    let mut caps = ConvexSolid::route_polygons_inside_of(
                        cutter.polygons.clone(),
                        source_solid,
                    );
                    for previous in &previous_cutters {
                        caps = ConvexSolid::route_polygons_outside_of(caps, previous);
                    }
                    for cap in &mut caps {
                        cap.material = source_solid.material;
                        cap.reversed = !cap.reversed;
                    }
                    surfaces.extend(caps);
                    previous_cutters.push(cutter);
                }
                BrushOp::Intersect => return None,
            }
        }

        let mut mesh = TriangleMesh::new();
        ConvexSolid::append_polygons_to_mesh(&surfaces, &mut mesh);
        report.emitted_convex_fragments = surfaces.len();

        Some(BuildOutput {
            mesh,
            report,
            generation: self.generation,
        })
    }

    fn supports_routed_surface_subset(&self) -> bool {
        let mut source = None::<Aabb>;
        let mut candidate_subtracts = 0usize;

        for brush in &self.compiled {
            match brush.op {
                BrushOp::Add => {
                    if source.is_some() || brush.convex_cutter().is_none() {
                        return false;
                    }
                    source = Some(brush.bounds);
                }
                BrushOp::Subtract => {
                    let Some(source_bounds) = source else {
                        return false;
                    };
                    if brush.convex_cutter().is_none() {
                        return false;
                    }
                    if source_bounds.intersects(brush.bounds) {
                        candidate_subtracts += 1;
                        if candidate_subtracts > 1 {
                            return false;
                        }
                    }
                }
                BrushOp::Intersect => return false,
            }
        }

        source.is_some() && candidate_subtracts == 1
    }

    fn build_uncached(&self) -> BuildOutput {
        if self
            .compiled
            .iter()
            .all(|brush| matches!(brush.geometry, CompiledGeometry::Box(_)))
        {
            return self.build_axis_aligned_boxes();
        }

        let mut report = BuildReport {
            input_brushes: self.brushes.len(),
            ..BuildReport::default()
        };
        let mut mesh = TriangleMesh::new();
        let mut solids: Vec<ConvexSolid> = Vec::new();

        for brush in &self.compiled {
            match brush.op {
                BrushOp::Add => match &brush.geometry {
                    CompiledGeometry::Box(bounds) => {
                        solids.push(ConvexSolid::from_aabb(*bounds, brush.material));
                    }
                    CompiledGeometry::Convex(solid) => solids.push(solid.clone()),
                    CompiledGeometry::CylinderZ {
                        center,
                        radius,
                        depth,
                        segments,
                    } => append_cylinder_z(
                        &mut mesh,
                        *center,
                        *radius,
                        *depth,
                        *segments,
                        brush.material,
                    ),
                    CompiledGeometry::DomeCapZ {
                        center,
                        radius,
                        height,
                        rings,
                        segments,
                    } => append_dome_cap_z(
                        &mut mesh,
                        DomeCapZSpec {
                            center: *center,
                            radius: *radius,
                            height: *height,
                            rings: *rings,
                            segments: *segments,
                            material: brush.material,
                        },
                    ),
                    CompiledGeometry::FloretArm {
                        anchor,
                        direction,
                        length,
                        root_width,
                        tip_width,
                        thickness,
                        tip_lift,
                    } => append_floret_arm(
                        &mut mesh,
                        FloretArmSpec {
                            anchor: *anchor,
                            direction: *direction,
                            length: *length,
                            root_width: *root_width,
                            tip_width: *tip_width,
                            thickness: *thickness,
                            tip_lift: *tip_lift,
                            material: brush.material,
                        },
                    ),
                },
                BrushOp::Subtract => {
                    report.operator_brushes += 1;
                    if !brush
                        .bounds
                        .intersects_any(solids.iter().map(|solid| solid.bounds))
                    {
                        report.rejected_pairs += solids.len();
                        continue;
                    }
                    if let Some(cutter) = brush.convex_cutter() {
                        solids = subtract_from_solids(solids, &cutter, &mut report);
                    } else {
                        let has_candidate = record_solid_pairs(&solids, brush.bounds, &mut report);
                        if has_candidate {
                            report
                                .warnings
                                .push(BuildWarning::SubtractIgnoredForNonBox {
                                    brush: brush.name.clone(),
                                });
                        }
                    }
                }
                BrushOp::Intersect => {
                    report.operator_brushes += 1;
                    if !brush
                        .bounds
                        .intersects_any(solids.iter().map(|solid| solid.bounds))
                    {
                        report.rejected_pairs += solids.len();
                        solids.clear();
                        continue;
                    }
                    if let Some(cutter) = brush.convex_cutter() {
                        solids = intersect_solids(solids, &cutter, &mut report);
                    } else {
                        let has_candidate = record_solid_pairs(&solids, brush.bounds, &mut report);
                        solids.clear();
                        if has_candidate {
                            report
                                .warnings
                                .push(BuildWarning::IntersectIgnoredForNonBox {
                                    brush: brush.name.clone(),
                                });
                        }
                    }
                }
            }
        }

        report.emitted_convex_fragments = solids.len();
        for solid in solids {
            solid.append_to_mesh(&mut mesh);
        }

        BuildOutput {
            mesh,
            report,
            generation: self.generation,
        }
    }

    fn build_axis_aligned_boxes(&self) -> BuildOutput {
        let mut report = BuildReport {
            input_brushes: self.brushes.len(),
            ..BuildReport::default()
        };
        let mut boxes: Vec<(Aabb, MaterialId)> = Vec::new();

        for brush in &self.compiled {
            let CompiledGeometry::Box(bounds) = brush.geometry else {
                unreachable!("box-only build path received a non-box brush");
            };
            match brush.op {
                BrushOp::Add => boxes.push((bounds, brush.material)),
                BrushOp::Subtract => {
                    report.operator_brushes += 1;
                    if !bounds.intersects_any(boxes.iter().map(|(bounds, _)| *bounds)) {
                        report.rejected_pairs += boxes.len();
                        continue;
                    }
                    let mut out = Vec::with_capacity(boxes.len() + 4);
                    for (solid, material) in boxes {
                        if solid.intersects(bounds) {
                            report.candidate_pairs += 1;
                            out.extend(
                                solid
                                    .subtract_box(bounds)
                                    .into_iter()
                                    .map(|piece| (piece, material)),
                            );
                        } else {
                            report.rejected_pairs += 1;
                            out.push((solid, material));
                        }
                    }
                    boxes = out;
                }
                BrushOp::Intersect => {
                    report.operator_brushes += 1;
                    if !bounds.intersects_any(boxes.iter().map(|(bounds, _)| *bounds)) {
                        report.rejected_pairs += boxes.len();
                        boxes.clear();
                        continue;
                    }
                    boxes = boxes
                        .into_iter()
                        .filter_map(|(solid, material)| {
                            if let Some(hit) = solid.intersection(bounds) {
                                report.candidate_pairs += 1;
                                Some((hit, material))
                            } else {
                                report.rejected_pairs += 1;
                                None
                            }
                        })
                        .collect();
                }
            }
        }

        let mut mesh = TriangleMesh::new();
        report.emitted_convex_fragments = boxes.len();
        for (bounds, material) in boxes {
            mesh.append_box(bounds, material);
        }

        BuildOutput {
            mesh,
            report,
            generation: self.generation,
        }
    }

    pub fn sample_room_with_door() -> Self {
        let wall = MaterialId(1);
        let floor = MaterialId(2);
        let mut asm = Self::new();
        asm.solid_box(
            "floor",
            Aabb::from_center_size(Vec3::new(0.0, 0.0, -0.1), Vec3::new(8.0, 8.0, 0.2)),
            floor,
        );
        asm.solid_box(
            "north wall",
            Aabb::from_center_size(Vec3::new(0.0, 4.0, 1.5), Vec3::new(8.0, 0.25, 3.0)),
            wall,
        );
        asm.cut_box(
            "door void",
            Aabb::from_center_size(Vec3::new(0.0, 4.0, 1.0), Vec3::new(1.2, 0.5, 2.0)),
        );
        asm
    }
}

impl CompiledBrush {
    fn from_brush(brush: &Brush) -> Self {
        let bounds = brush.bounds();
        let geometry = match &brush.primitive {
            Primitive::Box { bounds } => CompiledGeometry::Box(*bounds),
            Primitive::OrientedBox {
                center,
                size,
                rotation,
            } => CompiledGeometry::Convex(ConvexSolid::box_from_center_size(
                *center,
                *size,
                *rotation,
                brush.material,
            )),
            Primitive::CylinderZ {
                center,
                radius,
                depth,
                segments,
            } => CompiledGeometry::CylinderZ {
                center: *center,
                radius: *radius,
                depth: *depth,
                segments: *segments,
            },
            Primitive::DomeCapZ {
                center,
                radius,
                height,
                rings,
                segments,
            } => CompiledGeometry::DomeCapZ {
                center: *center,
                radius: *radius,
                height: *height,
                rings: *rings,
                segments: *segments,
            },
            Primitive::FloretArm {
                anchor,
                direction,
                length,
                root_width,
                tip_width,
                thickness,
                tip_lift,
            } => CompiledGeometry::FloretArm {
                anchor: *anchor,
                direction: *direction,
                length: *length,
                root_width: *root_width,
                tip_width: *tip_width,
                thickness: *thickness,
                tip_lift: *tip_lift,
            },
        };

        Self {
            op: brush.op,
            material: brush.material,
            bounds,
            geometry,
            name: brush.name.clone(),
        }
    }

    fn convex_cutter(&self) -> Option<ConvexSolid> {
        match &self.geometry {
            CompiledGeometry::Box(bounds) => Some(ConvexSolid::from_aabb(*bounds, self.material)),
            CompiledGeometry::Convex(solid) => Some(solid.clone()),
            _ => None,
        }
    }
}

fn subtract_from_solids(
    solids: Vec<ConvexSolid>,
    cutter: &ConvexSolid,
    report: &mut BuildReport,
) -> Vec<ConvexSolid> {
    let mut out = Vec::with_capacity(solids.len() + 4);
    for solid in solids {
        if solid.bounds.intersects(cutter.bounds) {
            report.candidate_pairs += 1;
            out.extend(solid.subtract_convex_owned(cutter));
        } else {
            report.rejected_pairs += 1;
            out.push(solid);
        }
    }
    out
}

fn intersect_solids(
    solids: Vec<ConvexSolid>,
    cutter: &ConvexSolid,
    report: &mut BuildReport,
) -> Vec<ConvexSolid> {
    let mut out = Vec::with_capacity(solids.len());
    for solid in solids {
        if !solid.bounds.intersects(cutter.bounds) {
            report.rejected_pairs += 1;
            continue;
        }
        report.candidate_pairs += 1;
        if let Some(fragment) = solid.intersect_convex_owned(cutter) {
            out.push(fragment);
        }
    }
    out
}

fn record_solid_pairs(solids: &[ConvexSolid], bounds: Aabb, report: &mut BuildReport) -> bool {
    let mut has_candidate = false;
    for solid in solids {
        if solid.bounds.intersects(bounds) {
            report.candidate_pairs += 1;
            has_candidate = true;
        } else {
            report.rejected_pairs += 1;
        }
    }
    has_candidate
}

#[cfg(test)]
mod tests {
    use bevy_math::Vec3;

    use super::*;

    #[test]
    fn centered_subtract_splits_box_into_six_fragments() {
        let source = Aabb::from_center_size(Vec3::ZERO, Vec3::splat(4.0));
        let cutter = Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0));
        let pieces = source.subtract_box(cutter);

        assert_eq!(pieces.len(), 6);
        assert!(pieces.iter().all(|piece| piece.is_valid()));
        assert!(pieces.iter().all(|piece| !piece.intersects(cutter)));
    }

    #[test]
    fn door_cut_leaves_no_box_fragment_inside_void() {
        let output = Assembler::sample_room_with_door().build();
        let cutter = Aabb::from_center_size(Vec3::new(0.0, 4.0, 1.0), Vec3::new(1.2, 0.5, 2.0));

        assert!(output.report.emitted_convex_fragments > 2);
        for tri in output.mesh.indices.chunks_exact(3) {
            let center = (output.mesh.positions[tri[0] as usize]
                + output.mesh.positions[tri[1] as usize]
                + output.mesh.positions[tri[2] as usize])
                / 3.0;
            assert!(!cutter.contains_point_strict(center, 1.0e-4));
        }
    }

    #[test]
    fn rotated_door_cut_produces_diagonal_portal_faces() {
        let mut asm = Assembler::new();
        asm.solid_box(
            "wall",
            Aabb::from_center_size(Vec3::new(0.0, 0.0, 1.5), Vec3::new(5.0, 0.5, 3.0)),
            MaterialId(1),
        );
        asm.cut_oriented_box(
            "angled void",
            Vec3::new(0.0, 0.0, 1.5),
            Vec3::new(1.5, 2.0, 2.0),
            bevy_math::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
        );

        let output = asm.build();
        assert!(output.report.emitted_convex_fragments > 2);
        assert!(output.mesh.normals.iter().any(|normal| {
            normal.x.abs() > 0.2 && normal.y.abs() > 0.2 && normal.z.abs() < 0.1
        }));
    }

    #[test]
    fn mesh_box_emits_twelve_triangles() {
        let mut mesh = TriangleMesh::new();
        mesh.append_box(Aabb::from_center_size(Vec3::ZERO, Vec3::ONE), MaterialId(7));

        assert_eq!(mesh.triangle_count(), 12);
        assert_eq!(mesh.triangle_materials.len(), 12);
        assert_eq!(mesh.triangle_materials[0], MaterialId(7));
    }

    #[test]
    fn distant_subtract_skips_convex_split_and_keeps_source_solid() {
        let mut asm = Assembler::new();
        asm.solid_box(
            "source",
            Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0)),
            MaterialId(1),
        );
        asm.cut_box(
            "far void",
            Aabb::from_center_size(Vec3::splat(10.0), Vec3::splat(1.0)),
        );

        let output = asm.build();
        assert_eq!(output.report.emitted_convex_fragments, 1);
        assert_eq!(output.report.operator_brushes, 1);
        assert_eq!(output.report.candidate_pairs, 0);
        assert_eq!(output.report.rejected_pairs, 1);
        assert_eq!(output.mesh.triangle_count(), 12);
    }

    #[test]
    fn intersecting_box_keeps_common_convex_region() {
        let mut asm = Assembler::new();
        asm.solid_box(
            "a",
            Aabb::from_center_size(Vec3::ZERO, Vec3::splat(4.0)),
            MaterialId(1),
        );
        asm.add_brush(
            "b",
            BrushOp::Intersect,
            Primitive::Box {
                bounds: Aabb::from_center_size(Vec3::X, Vec3::splat(4.0)),
            },
            MaterialId(1),
        );

        let output = asm.build();
        assert_eq!(output.report.emitted_convex_fragments, 1);
        assert_eq!(output.report.operator_brushes, 1);
        assert_eq!(output.report.candidate_pairs, 1);
        assert_eq!(output.report.rejected_pairs, 0);
        assert_eq!(output.report.warnings.len(), 0);
        assert_eq!(output.mesh.triangle_count(), 12);
    }

    #[test]
    fn cached_build_and_dirty_rebuild_emit_same_output() {
        let mut asm = Assembler::new();
        asm.solid_box(
            "slab",
            Aabb::from_center_size(Vec3::ZERO, Vec3::new(8.0, 8.0, 2.0)),
            MaterialId(1),
        );
        asm.cut_box(
            "center void",
            Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0)),
        );
        asm.cut_oriented_box(
            "angled void",
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(1.25, 4.0, 2.5),
            bevy_math::Quat::from_rotation_z(0.35),
        );

        let stable = asm.build();
        let cached = asm.build();
        let dirty = asm.rebuild();

        assert_eq!(
            stable.report.emitted_convex_fragments,
            dirty.report.emitted_convex_fragments
        );
        assert_eq!(
            stable.report.operator_brushes,
            dirty.report.operator_brushes
        );
        assert_eq!(stable.report.candidate_pairs, dirty.report.candidate_pairs);
        assert_eq!(stable.report.rejected_pairs, dirty.report.rejected_pairs);
        assert_eq!(stable.report.warnings, dirty.report.warnings);
        assert_eq!(stable.mesh.triangle_count(), dirty.mesh.triangle_count());
        assert_eq!(stable.mesh.vertex_count(), dirty.mesh.vertex_count());
        assert_eq!(stable.mesh.positions, dirty.mesh.positions);
        assert_eq!(stable.mesh.indices, dirty.mesh.indices);
        assert_eq!(cached.mesh.positions, stable.mesh.positions);
        assert_eq!(cached.mesh.indices, stable.mesh.indices);
    }

    #[test]
    fn routed_surface_rebuild_emits_clean_center_cut_boundary() {
        let mut asm = Assembler::new();
        let cutter = Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0));
        asm.solid_box(
            "source",
            Aabb::from_center_size(Vec3::ZERO, Vec3::splat(4.0)),
            MaterialId(1),
        );
        asm.cut_box("void", cutter);

        let routed = asm.rebuild_routed_surfaces();

        assert_eq!(routed.report.operator_brushes, 1);
        assert_eq!(routed.report.candidate_pairs, 1);
        assert_eq!(routed.report.rejected_pairs, 0);
        assert_eq!(routed.report.emitted_convex_fragments, 24);
        assert!(routed.mesh.triangle_count() < asm.rebuild().mesh.triangle_count());
        for tri in routed.mesh.indices.chunks_exact(3) {
            let center = (routed.mesh.positions[tri[0] as usize]
                + routed.mesh.positions[tri[1] as usize]
                + routed.mesh.positions[tri[2] as usize])
                / 3.0;
            assert!(!cutter.contains_point_strict(center, 1.0e-4));
        }
    }

    #[test]
    fn routed_surface_rebuild_falls_back_for_intersections() {
        let mut asm = Assembler::new();
        asm.solid_box(
            "source",
            Aabb::from_center_size(Vec3::ZERO, Vec3::splat(4.0)),
            MaterialId(1),
        );
        asm.add_brush(
            "common",
            BrushOp::Intersect,
            Primitive::Box {
                bounds: Aabb::from_center_size(Vec3::X, Vec3::splat(4.0)),
            },
            MaterialId(1),
        );

        let stable = asm.rebuild();
        let routed = asm.rebuild_routed_surfaces();

        assert_eq!(
            routed.report.emitted_convex_fragments,
            stable.report.emitted_convex_fragments
        );
        assert_eq!(routed.mesh.triangle_count(), stable.mesh.triangle_count());
        assert_eq!(routed.mesh.positions, stable.mesh.positions);
        assert_eq!(routed.mesh.indices, stable.mesh.indices);
    }

    #[test]
    fn routed_surface_rebuild_falls_back_for_multiple_candidate_cutters() {
        let mut asm = Assembler::new();
        asm.solid_box(
            "source",
            Aabb::from_center_size(Vec3::ZERO, Vec3::splat(4.0)),
            MaterialId(1),
        );
        asm.cut_oriented_box(
            "a",
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::new(1.0, 5.0, 5.0),
            bevy_math::Quat::from_rotation_z(0.25),
        );
        asm.cut_oriented_box(
            "b",
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(1.0, 5.0, 5.0),
            bevy_math::Quat::from_rotation_z(-0.25),
        );

        let stable = asm.rebuild();
        let routed = asm.rebuild_routed_surfaces();

        assert_eq!(
            routed.report.emitted_convex_fragments,
            stable.report.emitted_convex_fragments
        );
        assert_eq!(routed.mesh.triangle_count(), stable.mesh.triangle_count());
        assert_eq!(routed.mesh.positions, stable.mesh.positions);
        assert_eq!(routed.mesh.indices, stable.mesh.indices);
    }

    #[test]
    fn routed_surface_rebuild_falls_back_when_no_cutter_touches_source() {
        let mut asm = Assembler::new();
        asm.solid_box(
            "source",
            Aabb::from_center_size(Vec3::ZERO, Vec3::splat(4.0)),
            MaterialId(1),
        );
        asm.cut_box(
            "far",
            Aabb::from_center_size(Vec3::splat(20.0), Vec3::splat(2.0)),
        );

        let stable = asm.rebuild();
        let routed = asm.rebuild_routed_surfaces();

        assert_eq!(
            routed.report.emitted_convex_fragments,
            stable.report.emitted_convex_fragments
        );
        assert_eq!(routed.mesh.triangle_count(), stable.mesh.triangle_count());
        assert_eq!(routed.mesh.positions, stable.mesh.positions);
        assert_eq!(routed.mesh.indices, stable.mesh.indices);
    }
}
