//! Immutable acceleration over authored COL triangle bounds. Candidate
//! indices are restored to COL order before native narrow-phase evaluation.
use super::CColVector;

const TRIANGLES_PER_LEAF: usize = 8;
#[derive(Debug, Clone, PartialEq)]
struct BoundsNode {
    min: CColVector,
    max: CColVector,
    start: usize,
    end: usize,
    children: Option<(usize, usize)>,
}
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TriangleSpatialIndex {
    nodes: Vec<BoundsNode>,
    indices: Vec<usize>,
}
impl TriangleSpatialIndex {
    pub fn from_bounds(bounds: &[(CColVector, CColVector)]) -> Self {
        let mut index = Self {
            nodes: Vec::new(),
            indices: (0..bounds.len()).collect(),
        };
        if !bounds.is_empty() {
            index.build(bounds, 0, bounds.len());
        }
        index
    }
    fn build(&mut self, bounds: &[(CColVector, CColVector)], start: usize, end: usize) -> usize {
        let mut min = CColVector::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = CColVector::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for &i in &self.indices[start..end] {
            let (a, b) = bounds[i];
            min.x = min.x.min(a.x);
            min.y = min.y.min(a.y);
            min.z = min.z.min(a.z);
            max.x = max.x.max(b.x);
            max.y = max.y.max(b.y);
            max.z = max.z.max(b.z);
        }
        let node = self.nodes.len();
        self.nodes.push(BoundsNode {
            min,
            max,
            start,
            end,
            children: None,
        });
        if end - start > TRIANGLES_PER_LEAF {
            let extent = [max.x - min.x, max.y - min.y, max.z - min.z];
            let axis = (0..3)
                .max_by(|a, b| extent[*a].total_cmp(&extent[*b]))
                .unwrap();
            let midpoint = |i: usize| {
                let (a, b) = bounds[i];
                match axis {
                    0 => a.x + b.x,
                    1 => a.y + b.y,
                    _ => a.z + b.z,
                }
            };
            let mid = start + (end - start) / 2;
            self.indices[start..end].select_nth_unstable_by(mid - start, |a, b| {
                midpoint(*a).total_cmp(&midpoint(*b)).then(a.cmp(b))
            });
            let left = self.build(bounds, start, mid);
            let right = self.build(bounds, mid, end);
            self.nodes[node].children = Some((left, right));
        }
        node
    }
    pub fn sphere_candidates(
        &self,
        count: usize,
        center: CColVector,
        radius: f32,
        out: &mut Vec<usize>,
    ) {
        self.box_candidates(
            count,
            CColVector::new(center.x - radius, center.y - radius, center.z - radius),
            CColVector::new(center.x + radius, center.y + radius, center.z + radius),
            out,
        );
    }

    /// Conservative authored triangle candidates for a local-space footprint.
    /// Narrow phase still decides inclusion, in the original COL order.
    pub fn box_candidates(
        &self,
        count: usize,
        min: CColVector,
        max: CColVector,
        out: &mut Vec<usize>,
    ) {
        out.clear();
        // Temporary/test COLs can have no prepared cache. They use the same
        // full narrow phase, never fabricated or incomplete collision data.
        if self.indices.len() != count || self.nodes.is_empty() {
            out.extend(0..count);
            return;
        }
        self.collect(0, min, max, out);
        out.sort_unstable();
    }
    fn collect(&self, node: usize, min: CColVector, max: CColVector, out: &mut Vec<usize>) {
        let node = &self.nodes[node];
        if max.x < node.min.x
            || min.x > node.max.x
            || max.y < node.min.y
            || min.y > node.max.y
            || max.z < node.min.z
            || min.z > node.max.z
        {
            return;
        }
        if let Some((left, right)) = node.children {
            self.collect(left, min, max, out);
            self.collect(right, min, max, out);
        } else {
            out.extend_from_slice(&self.indices[node.start..node.end]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footprint_candidates_cover_overlaps_in_all_axes_in_original_order() {
        let bounds: Vec<_> = (0..256).map(|i| {
            let x = (i % 16) as f32 * 10.0;
            let y = (i / 16) as f32 * 10.0;
            let z = (i % 3) as f32 * 5.0;
            (CColVector::new(x, y, z), CColVector::new(x + 2.0, y + 2.0, z + 1.0))
        }).collect();
        let tree = TriangleSpatialIndex::from_bounds(&bounds);
        let mut candidates = Vec::new();
        for x in [-1.0, 0.0, 2.0, 10.0, 43.0, 140.0] {
            for z in [0.0, 1.0, 5.0, 10.0] {
                let min = CColVector::new(x, 20.0, z);
                let max = CColVector::new(x + 8.0, 32.0, z + 1.0);
                tree.box_candidates(bounds.len(), min, max, &mut candidates);
                assert!(candidates.windows(2).all(|pair| pair[0] < pair[1]));
                assert!(candidates.len() < bounds.len());
                for (id, (a, b)) in bounds.iter().enumerate() {
                    if max.x >= a.x && min.x <= b.x && max.y >= a.y && min.y <= b.y
                        && max.z >= a.z && min.z <= b.z {
                        assert!(candidates.binary_search(&id).is_ok(), "omitted triangle {id}");
                    }
                }
            }
        }
        tree.box_candidates(260, CColVector::new(0.0, 0.0, 0.0), CColVector::new(1.0, 1.0, 1.0), &mut candidates);
        assert_eq!(candidates, (0..260).collect::<Vec<_>>());
    }
    #[test]
    fn candidates_conservatively_cover_bounds_and_retain_native_order() {
        let bounds: Vec<_> = (0..257)
            .map(|i| {
                let x = ((i * 37) % 257) as f32 - 128.0;
                (
                    CColVector::new(x, -1.0, -2.0),
                    CColVector::new(x + 2.0, 1.0, 2.0),
                )
            })
            .collect();
        let tree = TriangleSpatialIndex::from_bounds(&bounds);
        let mut candidates = Vec::new();
        for x in -140..140 {
            for r in [0.0, 0.1, 2.0, 30.0] {
                tree.sphere_candidates(
                    bounds.len(),
                    CColVector::new(x as f32, 0.0, 0.0),
                    r,
                    &mut candidates,
                );
                assert!(candidates.windows(2).all(|pair| pair[0] < pair[1]));
                for (i, (min, max)) in bounds.iter().enumerate() {
                    if x as f32 + r >= min.x && x as f32 - r <= max.x {
                        assert!(
                            candidates.binary_search(&i).is_ok(),
                            "omitted original COL triangle {i}"
                        );
                    }
                }
            }
        }
        tree.sphere_candidates(300, CColVector::new(0.0, 0.0, 0.0), 1.0, &mut candidates);
        assert_eq!(candidates, (0..300).collect::<Vec<_>>());
    }
}
