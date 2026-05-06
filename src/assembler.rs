use bevy_math::Vec3;

use crate::{
    Aabb, Brush, BrushId, BrushOp, MaterialId, Primitive, TriangleMesh, append_cylinder_z,
    append_dome_cap_z, append_floret_arm,
    primitives::{DomeCapZSpec, FloretArmSpec},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildWarning {
    SubtractIgnoredForNonBox { brush: String },
    IntersectUnsupported { brush: String },
}

#[derive(Clone, Debug, Default)]
pub struct BuildReport {
    pub input_brushes: usize,
    pub emitted_box_fragments: usize,
    pub warnings: Vec<BuildWarning>,
}

#[derive(Clone, Debug)]
pub struct BuildOutput {
    pub mesh: TriangleMesh,
    pub report: BuildReport,
    pub generation: u64,
}

#[derive(Clone, Debug)]
struct BoxSolid {
    bounds: Aabb,
    material: MaterialId,
}

#[derive(Clone, Debug, Default)]
pub struct Assembler {
    brushes: Vec<Brush>,
    next_id: u32,
    generation: u64,
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
        self.brushes
            .push(Brush::new(id, name, op, primitive, material));
        self.generation = self.generation.wrapping_add(1);
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

    pub fn build(&self) -> BuildOutput {
        let mut report = BuildReport {
            input_brushes: self.brushes.len(),
            ..BuildReport::default()
        };
        let mut mesh = TriangleMesh::new();
        let mut boxes: Vec<BoxSolid> = Vec::new();

        for brush in &self.brushes {
            match brush.op {
                BrushOp::Add => match &brush.primitive {
                    Primitive::Box { bounds } => boxes.push(BoxSolid {
                        bounds: *bounds,
                        material: brush.material,
                    }),
                    Primitive::CylinderZ {
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
                    Primitive::DomeCapZ {
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
                    Primitive::FloretArm {
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
                    if let Some(cutter) = brush.as_box() {
                        boxes = subtract_from_boxes(&boxes, cutter);
                    } else {
                        report
                            .warnings
                            .push(BuildWarning::SubtractIgnoredForNonBox {
                                brush: brush.name.clone(),
                            });
                    }
                }
                BrushOp::Intersect => {
                    report.warnings.push(BuildWarning::IntersectUnsupported {
                        brush: brush.name.clone(),
                    });
                }
            }
        }

        report.emitted_box_fragments = boxes.len();
        for solid in boxes {
            mesh.append_box(solid.bounds, solid.material);
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

fn subtract_from_boxes(boxes: &[BoxSolid], cutter: Aabb) -> Vec<BoxSolid> {
    let mut out = Vec::with_capacity(boxes.len() + 4);
    for solid in boxes {
        for bounds in solid.bounds.subtract_box(cutter) {
            out.push(BoxSolid {
                bounds,
                material: solid.material,
            });
        }
    }
    out
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

        assert!(output.report.emitted_box_fragments > 2);
        for tri in output.mesh.indices.chunks_exact(3) {
            let center = (output.mesh.positions[tri[0] as usize]
                + output.mesh.positions[tri[1] as usize]
                + output.mesh.positions[tri[2] as usize])
                / 3.0;
            assert!(!cutter.contains_point_strict(center, 1.0e-4));
        }
    }

    #[test]
    fn mesh_box_emits_twelve_triangles() {
        let mut mesh = TriangleMesh::new();
        mesh.append_box(Aabb::from_center_size(Vec3::ZERO, Vec3::ONE), MaterialId(7));

        assert_eq!(mesh.triangle_count(), 12);
        assert_eq!(mesh.triangle_materials.len(), 12);
        assert_eq!(mesh.triangle_materials[0], MaterialId(7));
    }
}
