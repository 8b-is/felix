//! Signed Balanced-Ternary Graph Theory (`SignedTernaryGraph`)
//!
//! Implements classical structural balance and spectral signed-graph algorithms
//! (Harary, 1953; Cartwright & Harary, 1956; Zaslavsky, 1982):
//!
//! 1. **Ternary Adjacency Matrix (`A^\sigma` in `{-1, 0, +1}^{N x N}`)**:
//!    - `+1`: positive edge (cooperative / aligned coupling)
//!    - ` 0`: no edge (disconnected / orthogonal)
//!    - `-1`: negative edge (antagonistic / phase-inverted coupling)
//! 2. **Signed Graph Laplacian (`L^\sigma = D - A^\sigma`)**:
//!    For any vertex potential vector `x in Z^N`, the signed Dirichlet quadratic form is:
//!    `x^T L^\sigma x = \sum_{u < v, \sigma_{uv} != 0} (x_u - \sigma_{uv} x_v)^2 >= 0`.
//! 3. **Harary's Structural Balance Theorem**:
//!    A signed graph `(V, E, \sigma)` is structurally balanced (every cycle has product `+1`)
//!    if and only if there exists a gauge assignment `s in {-1, +1}^N` such that
//!    `s^T L^\sigma s = 0` (i.e. `\sigma_{uv} = s_u * s_v` for every active edge).

use std::collections::VecDeque;

/// Edge sign in a balanced-ternary graph `{-1, 0, +1}`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(i8)]
pub enum EdgeSign {
    Negative = -1,
    Absent = 0,
    Positive = 1,
}

impl EdgeSign {
    #[inline]
    pub fn as_i32(self) -> i32 {
        self as i8 as i32
    }
}

/// Result of Harary structural balance analysis on a signed ternary graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralBalanceReport {
    pub num_vertices: usize,
    pub num_edges: usize,
    /// True iff every cycle has an even number of `-1` edges.
    pub is_structurally_balanced: bool,
    /// Gauge partition `s in {-1, +1}^N` (minimizing frustration if unbalanced, or exact if balanced).
    pub gauge_partition: Vec<i8>,
    /// Signed Laplacian quadratic energy `s^T L^\sigma s = 4 * frustrated_edges` (`0` iff balanced).
    pub laplacian_quadratic_energy: i64,
    /// Number of edges that violate `\sigma_{uv} == s_u * s_v` under the BFS partition.
    pub frustrated_edge_count: usize,
}

/// Undirected signed graph with adjacency entries in `{-1, 0, +1}`.
#[derive(Clone, Debug)]
pub struct SignedTernaryGraph {
    pub n: usize,
    pub adj: Vec<Vec<EdgeSign>>,
}

impl SignedTernaryGraph {
    /// Creates an empty signed graph on `n` vertices.
    pub fn new(n: usize) -> Self {
        Self {
            n,
            adj: vec![vec![EdgeSign::Absent; n]; n],
        }
    }

    /// Sets an undirected signed edge `(u, v)` with sign `sign` (`u != v`).
    pub fn set_edge(&mut self, u: usize, v: usize, sign: EdgeSign) {
        if u < self.n && v < self.n && u != v {
            self.adj[u][v] = sign;
            self.adj[v][u] = sign;
        }
    }

    /// Returns the absolute degree `d_u = \sum_v |\sigma_{uv}|` of vertex `u`.
    pub fn degree(&self, u: usize) -> usize {
        self.adj[u]
            .iter()
            .filter(|&&s| s != EdgeSign::Absent)
            .count()
    }

    /// Returns the total number of undirected active edges (`\sigma_{uv} != 0`).
    pub fn edge_count(&self) -> usize {
        let mut count = 0usize;
        for u in 0..self.n {
            for v in (u + 1)..self.n {
                if self.adj[u][v] != EdgeSign::Absent {
                    count += 1;
                }
            }
        }
        count
    }

    /// Computes the Signed Graph Laplacian matrix `L^\sigma = D - A^\sigma`.
    pub fn signed_laplacian(&self) -> Vec<Vec<i32>> {
        let mut lap = vec![vec![0i32; self.n]; self.n];
        for (u, row) in lap.iter_mut().enumerate() {
            row[u] = self.degree(u) as i32;
            for (v, cell) in row.iter_mut().enumerate() {
                if u != v {
                    *cell = -self.adj[u][v].as_i32();
                }
            }
        }
        lap
    }

    /// Evaluates the exact signed Dirichlet quadratic form:
    /// `x^T L^\sigma x = \sum_{u < v, \sigma_{uv} != 0} (x_u - \sigma_{uv} x_v)^2 >= 0`.
    pub fn evaluate_laplacian_quadratic_form(&self, x: &[i8]) -> i64 {
        assert_eq!(x.len(), self.n);
        let mut sum = 0i64;
        for u in 0..self.n {
            for v in (u + 1)..self.n {
                let s = self.adj[u][v].as_i32() as i64;
                if s != 0 {
                    let diff = (x[u] as i64) - s * (x[v] as i64);
                    sum += diff * diff;
                }
            }
        }
        sum
    }

    /// Applies a switching transformation `D_s = diag(s)` (`s_u in {-1, +1}`):
    /// `\sigma'_{uv} = s_u * \sigma_{uv} * s_v`.
    /// By Zaslavsky's theorem, a balanced graph switches to an all-positive graph (`\sigma'_{uv} >= 0`).
    pub fn gauge_switch(&self, s: &[i8]) -> Self {
        assert_eq!(s.len(), self.n);
        let mut out = Self::new(self.n);
        for u in 0..self.n {
            for v in (u + 1)..self.n {
                let prod = (s[u] as i32) * self.adj[u][v].as_i32() * (s[v] as i32);
                let new_sign = match prod {
                    1 => EdgeSign::Positive,
                    -1 => EdgeSign::Negative,
                    _ => EdgeSign::Absent,
                };
                out.set_edge(u, v, new_sign);
            }
        }
        out
    }

    /// Runs Harary's BFS 2-coloring algorithm across all connected components to test
    /// structural balance and compute the signed Laplacian energy.
    pub fn analyze_structural_balance(&self) -> StructuralBalanceReport {
        let mut s = vec![0i8; self.n];
        let mut queue = VecDeque::new();

        for start in 0..self.n {
            if s[start] != 0 {
                continue;
            }
            s[start] = 1;
            queue.push_back(start);

            while let Some(u) = queue.pop_front() {
                for v in 0..self.n {
                    let edge = self.adj[u][v];
                    if edge != EdgeSign::Absent && s[v] == 0 {
                        s[v] = s[u] * (edge as i8);
                        queue.push_back(v);
                    }
                }
            }
        }

        let mut frustrated = 0usize;
        for u in 0..self.n {
            for v in (u + 1)..self.n {
                let edge = self.adj[u][v];
                if edge != EdgeSign::Absent && (s[u] * s[v]) != (edge as i8) {
                    frustrated += 1;
                }
            }
        }

        let energy = self.evaluate_laplacian_quadratic_form(&s);

        StructuralBalanceReport {
            num_vertices: self.n,
            num_edges: self.edge_count(),
            is_structurally_balanced: frustrated == 0 && energy == 0,
            gauge_partition: s,
            laplacian_quadratic_energy: energy,
            frustrated_edge_count: frustrated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harary_balanced_signed_graph_and_switching() {
        // Two coalitions {0, 1} and {2, 3}:
        // Positive edges within coalitions, negative edges across coalitions.
        let mut g = SignedTernaryGraph::new(4);
        g.set_edge(0, 1, EdgeSign::Positive);
        g.set_edge(2, 3, EdgeSign::Positive);
        g.set_edge(0, 2, EdgeSign::Negative);
        g.set_edge(1, 3, EdgeSign::Negative);
        g.set_edge(0, 3, EdgeSign::Negative);

        let report = g.analyze_structural_balance();
        assert!(report.is_structurally_balanced);
        assert_eq!(report.frustrated_edge_count, 0);
        assert_eq!(report.laplacian_quadratic_energy, 0);
        assert_eq!(report.gauge_partition, vec![1, 1, -1, -1]);

        // Switching by the gauge partition turns every active edge positive
        let switched = g.gauge_switch(&report.gauge_partition);
        for u in 0..4 {
            for v in (u + 1)..4 {
                assert_ne!(switched.adj[u][v], EdgeSign::Negative);
            }
        }
    }

    #[test]
    fn test_frustrated_negative_triangle() {
        // K_3 with three negative edges (-1 * -1 * -1 = -1 -> unbalanced)
        let mut g = SignedTernaryGraph::new(3);
        g.set_edge(0, 1, EdgeSign::Negative);
        g.set_edge(1, 2, EdgeSign::Negative);
        g.set_edge(0, 2, EdgeSign::Negative);

        let report = g.analyze_structural_balance();
        assert!(!report.is_structurally_balanced);
        assert_eq!(report.frustrated_edge_count, 1);
        // Each frustrated edge contributes (1 - (-1)*(1))^2 = 4 to x^T L^\sigma x
        assert_eq!(report.laplacian_quadratic_energy, 4);
    }
}
