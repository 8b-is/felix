//! Ternary Incidence Hypergraphs & Laplacian Operators (`quant_ultra_graphs`)
//!
//! Implements standard discrete graph and hypergraph incidence operators:
//! 1. **Ternary Incidence Matrix (`B in {-1, 0, +1}^{|V| x |E|}`)**:
//!    - `-1`: outgoing / tail incidence
//!    - ` 0`: non-incident
//!    - `+1`: incoming / head incidence
//! 2. **Add-Only Incidence Contraction & Weighted Laplacian (`L = B * W * B^T`)**:
//!    Applies `B^T x` and `B y` using pure integer additions/subtractions, scaled by
//!    dyadic rational edge weights `w_e = p_e * 2^(-k_e)`.
//! 3. **Ramanujan Spectral Bound Check**:
//!    Checks whether the second-largest adjacency eigenvalue magnitude `\lambda` of a
//!    `d`-regular graph satisfies the Alon–Boppana / Ramanujan bound `\lambda <= 2 * sqrt(d - 1)`.
//! 4. **Subset Cut Capacity (`Cut(A, V \ A)`)**:
//!    Computes the weighted cut across any vertex bipartition `(A, V \ A)`.
//! 5. **Kirchhoff / Eulerian Flow Conservation**:
//!    Verifies `\sum_e B_{v, e} == 0` for every vertex `v in V`.

/// Dyadic rational edge weight `w_e = numerator_p * 2^(-shift_k)`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DyadicEdgeWeight {
    pub numerator_p: u32,
    pub shift_k: u8,
}

impl DyadicEdgeWeight {
    #[inline]
    pub fn as_f64(self) -> f64 {
        (self.numerator_p as f64) * 0.5_f64.powi(self.shift_k as i32)
    }
}

/// Ternary incidence matrix `B in {-1, 0, +1}^{|V| x |E|}` with dyadic edge weights.
#[derive(Clone, Debug)]
pub struct TernaryIncidenceGraph {
    pub num_vertices: usize,
    pub num_edges: usize,
    /// Row-major `num_vertices x num_edges` matrix with entries in `{-1, 0, +1}`.
    pub incidence_b: Vec<Vec<i8>>,
    pub edge_weights: Vec<DyadicEdgeWeight>,
}

impl TernaryIncidenceGraph {
    pub fn new(num_vertices: usize, num_edges: usize) -> Self {
        Self {
            num_vertices,
            num_edges,
            incidence_b: vec![vec![0i8; num_edges]; num_vertices],
            edge_weights: vec![
                DyadicEdgeWeight {
                    numerator_p: 1,
                    shift_k: 0,
                };
                num_edges
            ],
        }
    }

    /// Sets the incidence entry `B_{v, e}` in `{-1, 0, +1}`.
    pub fn set_incidence(&mut self, v: usize, e: usize, trit: i8) {
        if v < self.num_vertices && e < self.num_edges {
            self.incidence_b[v][e] = trit.clamp(-1, 1);
        }
    }

    /// Sets the dyadic weight `p * 2^(-k)` for edge `e`.
    pub fn set_edge_weight(&mut self, e: usize, numerator_p: u32, shift_k: u8) {
        if e < self.num_edges {
            self.edge_weights[e] = DyadicEdgeWeight {
                numerator_p,
                shift_k,
            };
        }
    }

    /// Computes the coboundary `d = B^T * x` in `Z^{|E|}` using only integer additions and subtractions.
    pub fn coboundary_bt_x(&self, x: &[i64]) -> Vec<i64> {
        assert_eq!(x.len(), self.num_vertices);
        let mut out = vec![0i64; self.num_edges];
        for (v, row) in self.incidence_b.iter().enumerate() {
            let xv = x[v];
            for (e, &b_ve) in row.iter().enumerate() {
                if b_ve == 1 {
                    out[e] += xv;
                } else if b_ve == -1 {
                    out[e] -= xv;
                }
            }
        }
        out
    }

    /// Applies the weighted Laplacian `y = (B * W * B^T) * x`.
    pub fn apply_weighted_laplacian(&self, x: &[i64]) -> Vec<f64> {
        let bt_x = self.coboundary_bt_x(x);
        let mut weighted_edges = vec![0.0_f64; self.num_edges];
        for e in 0..self.num_edges {
            weighted_edges[e] = (bt_x[e] as f64) * self.edge_weights[e].as_f64();
        }

        let mut y = vec![0.0_f64; self.num_vertices];
        for (v, row) in self.incidence_b.iter().enumerate() {
            let mut acc = 0.0_f64;
            for (e, &b_ve) in row.iter().enumerate() {
                if b_ve == 1 {
                    acc += weighted_edges[e];
                } else if b_ve == -1 {
                    acc -= weighted_edges[e];
                }
            }
            y[v] = acc;
        }
        y
    }

    /// Checks Kirchhoff / Eulerian zero-sum flow conservation at every vertex:
    /// `\forall v \in V, \sum_{e} B_{v, e} == 0`.
    pub fn is_kirchhoff_balanced(&self) -> bool {
        self.incidence_b
            .iter()
            .all(|row| row.iter().map(|&t| t as i32).sum::<i32>() == 0)
    }

    /// Computes the weighted cut capacity across a vertex subset mask `in_subset_a`:
    /// An edge `e` is cut if it is incident (`B_{v,e} != 0`) to at least one vertex in `A`
    /// and at least one vertex in `V \ A`.
    pub fn compute_subset_cut_capacity(&self, in_subset_a: &[bool]) -> f64 {
        assert_eq!(in_subset_a.len(), self.num_vertices);
        let mut cut_sum = 0.0_f64;
        for e in 0..self.num_edges {
            let mut touches_a = false;
            let mut touches_complement = false;
            for (v, &in_a) in in_subset_a.iter().enumerate() {
                if self.incidence_b[v][e] != 0 {
                    if in_a {
                        touches_a = true;
                    } else {
                        touches_complement = true;
                    }
                }
            }
            if touches_a && touches_complement {
                cut_sum += self.edge_weights[e].as_f64();
            }
        }
        cut_sum
    }

    /// Checks the Ramanujan bound `\lambda <= 2 * sqrt(d - 1)` for a `d`-regular graph (`d >= 2`).
    #[inline]
    pub fn is_ramanujan_eigenvalue_bounded(degree_d: usize, lambda_2_abs: f64) -> bool {
        if degree_d < 2 {
            return false;
        }
        let bound = 2.0 * ((degree_d - 1) as f64).sqrt();
        lambda_2_abs <= bound + 1e-12
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directed_cycle_kirchhoff_balance_and_laplacian() {
        // Directed 3-cycle: e0=(0->1), e1=(1->2), e2=(2->0)
        // Each vertex has one -1 (outgoing) and one +1 (incoming) -> row sum = 0.
        let mut g = TernaryIncidenceGraph::new(3, 3);
        g.set_incidence(0, 0, -1);
        g.set_incidence(1, 0, 1);

        g.set_incidence(1, 1, -1);
        g.set_incidence(2, 1, 1);

        g.set_incidence(2, 2, -1);
        g.set_incidence(0, 2, 1);

        assert!(g.is_kirchhoff_balanced());

        // Constant potential vector x = [5, 5, 5] lies in the kernel of B^T and L
        let y = g.apply_weighted_laplacian(&[5, 5, 5]);
        assert!(y.iter().all(|&v| v.abs() < 1e-12));

        // Cut of A = {0} vs V \ A = {1, 2} cuts edges e0 and e2 (weight 1.0 + 1.0 = 2.0)
        let cut = g.compute_subset_cut_capacity(&[true, false, false]);
        assert!((cut - 2.0).abs() < 1e-12);

        // 6-regular hypercube Q_6 has 2*sqrt(5) ≈ 4.4721; verify Ramanujan check
        assert!(TernaryIncidenceGraph::is_ramanujan_eigenvalue_bounded(6, 4.0));
        assert!(!TernaryIncidenceGraph::is_ramanujan_eigenvalue_bounded(6, 5.0));
    }
}
