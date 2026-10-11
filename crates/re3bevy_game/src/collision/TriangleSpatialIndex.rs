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
        out.clear();
        // Temporary/test COLs can have no prepared cache. They use the same
        // full narrow phase, never fabricated or incomplete collision data.
        if self.indices.len() != count || self.nodes.is_empty() {
            out.extend(0..count);
            return;
        }
        self.collect(0, center, radius, out);
        out.sort_unstable();
    }
    fn collect(&self, node: usize, p: CColVector, radius: f32, out: &mut Vec<usize>) {
        let node = &self.nodes[node];
        if p.x + radius < node.min.x
            || p.x - radius > node.max.x
            || p.y + radius < node.min.y
            || p.y - radius > node.max.y
            || p.z + radius < node.min.z
            || p.z - radius > node.max.z
        {
            return;
        }
        if let Some((left, right)) = node.children {
            self.collect(left, p, radius, out);
            self.collect(right, p, radius, out);
        } else {
            out.extend_from_slice(&self.indices[node.start..node.end]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
