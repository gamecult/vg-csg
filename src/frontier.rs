use crate::{Brush, BrushOp};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DemandPair {
    pub source_index: usize,
    pub operator_index: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DemandFrontier {
    pub pairs: Vec<DemandPair>,
    pub rejected_pairs: usize,
    pub operator_brushes: usize,
}

impl DemandFrontier {
    pub fn from_ordered_brushes(brushes: &[Brush]) -> Self {
        let mut frontier = Self::default();
        let mut active_sources = Vec::<usize>::new();

        for (operator_index, brush) in brushes.iter().enumerate() {
            match brush.op {
                BrushOp::Add => active_sources.push(operator_index),
                BrushOp::Subtract => {
                    frontier.operator_brushes += 1;
                    frontier.collect_pairs(brushes, &active_sources, operator_index);
                }
                BrushOp::Intersect => {
                    frontier.operator_brushes += 1;
                    let retained = frontier.collect_pairs(brushes, &active_sources, operator_index);
                    active_sources = retained;
                }
            }
        }

        frontier
    }

    pub fn candidate_pairs(&self) -> usize {
        self.pairs.len()
    }

    fn collect_pairs(
        &mut self,
        brushes: &[Brush],
        active_sources: &[usize],
        operator_index: usize,
    ) -> Vec<usize> {
        let operator_bounds = brushes[operator_index].bounds();
        let mut retained = Vec::new();

        for &source_index in active_sources {
            if brushes[source_index].bounds().intersects(operator_bounds) {
                self.pairs.push(DemandPair {
                    source_index,
                    operator_index,
                });
                retained.push(source_index);
            } else {
                self.rejected_pairs += 1;
            }
        }

        retained
    }
}

#[cfg(test)]
mod tests {
    use bevy_math::{Quat, Vec3};

    use super::*;
    use crate::{Aabb, Brush, MaterialId, Primitive};

    fn brush(name: &str, op: BrushOp, bounds: Aabb) -> Brush {
        Brush::new(
            crate::BrushId(0),
            name,
            op,
            Primitive::Box { bounds },
            MaterialId(1),
        )
    }

    #[test]
    fn frontier_keeps_only_touching_ordered_pairs() {
        let brushes = vec![
            brush(
                "source",
                BrushOp::Add,
                Aabb::from_center_size(Vec3::ZERO, Vec3::splat(4.0)),
            ),
            brush(
                "far cutter",
                BrushOp::Subtract,
                Aabb::from_center_size(Vec3::splat(20.0), Vec3::ONE),
            ),
            brush(
                "near cutter",
                BrushOp::Subtract,
                Aabb::from_center_size(Vec3::ZERO, Vec3::ONE),
            ),
        ];

        let frontier = DemandFrontier::from_ordered_brushes(&brushes);

        assert_eq!(frontier.operator_brushes, 2);
        assert_eq!(frontier.rejected_pairs, 1);
        assert_eq!(
            frontier.pairs,
            vec![DemandPair {
                source_index: 0,
                operator_index: 2,
            }]
        );
    }

    #[test]
    fn intersection_frontier_reduces_active_sources() {
        let brushes = vec![
            brush(
                "a",
                BrushOp::Add,
                Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0)),
            ),
            brush(
                "b",
                BrushOp::Add,
                Aabb::from_center_size(Vec3::new(20.0, 0.0, 0.0), Vec3::splat(2.0)),
            ),
            Brush::new(
                crate::BrushId(2),
                "common",
                BrushOp::Intersect,
                Primitive::OrientedBox {
                    center: Vec3::ZERO,
                    size: Vec3::splat(3.0),
                    rotation: Quat::IDENTITY,
                },
                MaterialId(1),
            ),
            brush(
                "cut",
                BrushOp::Subtract,
                Aabb::from_center_size(Vec3::ZERO, Vec3::ONE),
            ),
        ];

        let frontier = DemandFrontier::from_ordered_brushes(&brushes);

        assert_eq!(frontier.operator_brushes, 2);
        assert_eq!(frontier.rejected_pairs, 1);
        assert_eq!(
            frontier.pairs,
            vec![
                DemandPair {
                    source_index: 0,
                    operator_index: 2,
                },
                DemandPair {
                    source_index: 0,
                    operator_index: 3,
                },
            ]
        );
    }
}
