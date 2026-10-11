//! Minimal cycle basis of a planar graph.
//!
//! Port of David Eberly's `MinimalCycleBasis` (Geometric Tools, Boost
//! license): <https://www.geometrictools.com/Documentation/MinimalCycleBasis.pdf>
//! The original's exact rational arithmetic is replaced by f64 determinants.

use std::collections::{BTreeSet, HashMap};

type Point = [f64; 2];

/// Finds the minimal cycles of a planar graph whose edges meet only at
/// shared vertices. Returned cycles are closed vertex loops without the
/// trailing duplicate; filaments and isolated vertices yield no cycle.
pub(super) fn minimal_cycles(
    positions: &[Point],
    edges: &[[usize; 2]],
) -> Vec<Vec<usize>> {
    let mut graph = Graph {
        vertices: positions
            .iter()
            .enumerate()
            .map(|(index, &position)| Vertex {
                index,
                position,
                adjacents: BTreeSet::new(),
                visited: 0,
            })
            .collect(),
        stamp: 0,
    };
    for &[a, b] in edges {
        if a != b {
            graph.insert(a, b);
        }
    }

    let mut cycles = Vec::new();
    for tree in graph.extract_forest() {
        flatten(tree, &mut cycles);
    }
    cycles
}

/// A cycle and the cycles of subgraphs detached from inside it.
#[derive(Default)]
struct Tree {
    cycle: Vec<usize>,
    children: Vec<Tree>,
}

fn flatten(tree: Tree, out: &mut Vec<Vec<usize>>) {
    let mut cycle = tree.cycle;
    // Closed walks repeat their first vertex at the end.
    cycle.pop();
    if !cycle.is_empty() {
        out.push(cycle);
    }
    for child in tree.children {
        flatten(child, out);
    }
}

struct Vertex {
    /// Index into the caller's positions; clones keep their original's index.
    index: usize,
    position: Point,
    adjacents: BTreeSet<usize>,
    visited: u32,
}

struct Graph {
    vertices: Vec<Vertex>,
    /// Generation counter for `visited`, so searches never need a reset pass.
    stamp: u32,
}

impl Graph {
    fn insert(&mut self, a: usize, b: usize) {
        self.vertices[a].adjacents.insert(b);
        self.vertices[b].adjacents.insert(a);
    }

    fn remove(&mut self, a: usize, b: usize) {
        self.vertices[a].adjacents.remove(&b);
        self.vertices[b].adjacents.remove(&a);
    }

    fn pos(&self, v: usize) -> Point {
        self.vertices[v].position
    }

    fn first_adjacent(&self, v: usize) -> usize {
        *self.vertices[v].adjacents.iter().next().unwrap()
    }

    fn clone_vertex(&mut self, v: usize) -> usize {
        self.vertices.push(Vertex {
            index: self.vertices[v].index,
            position: self.vertices[v].position,
            adjacents: BTreeSet::new(),
            visited: 0,
        });
        self.vertices.len() - 1
    }

    fn extract_forest(&mut self) -> Vec<Tree> {
        self.stamp += 1;
        let stamp = self.stamp;
        let mut components = Vec::new();
        for v in 0..self.vertices.len() {
            if self.vertices[v].adjacents.len() >= 2
                && self.vertices[v].visited != stamp
            {
                components.push(self.depth_first_search(v, stamp));
            }
        }

        let mut forest = Vec::new();
        for mut component in components {
            let tree = self.extract_basis(&mut component);
            if !tree.children.is_empty() || !tree.cycle.is_empty() {
                forest.push(tree);
            }
        }
        forest
    }

    fn depth_first_search(
        &mut self,
        start: usize,
        stamp: u32,
    ) -> Vec<usize> {
        let mut component = Vec::new();
        let mut stack = vec![start];
        self.vertices[start].visited = stamp;
        while let Some(v) = stack.pop() {
            component.push(v);
            let adjacents: Vec<usize> =
                self.vertices[v].adjacents.iter().copied().collect();
            for a in adjacents {
                if self.vertices[a].visited != stamp {
                    self.vertices[a].visited = stamp;
                    stack.push(a);
                }
            }
        }
        component
    }

    fn detached_component(&mut self, clone: usize) -> Vec<usize> {
        self.stamp += 1;
        let stamp = self.stamp;
        self.depth_first_search(clone, stamp)
    }

    fn extract_basis(&mut self, component: &mut Vec<usize>) -> Tree {
        let mut tree = Tree::default();
        while !component.is_empty() {
            self.remove_filaments(component);
            if component.is_empty() {
                break;
            }
            match self.extract_cycle_from_component(component) {
                Some(child) => tree.children.push(child),
                // Inconsistent geometry; abandon this component.
                None => break,
            }
        }
        collapse_single_child(&mut tree);
        tree
    }

    fn remove_filaments(&mut self, component: &mut Vec<usize>) {
        let endpoints: Vec<usize> = component
            .iter()
            .copied()
            .filter(|&v| self.vertices[v].adjacents.len() == 1)
            .collect();

        for endpoint in endpoints {
            let mut v = endpoint;
            // Already consumed while walking another filament.
            if self.vertices[v].adjacents.is_empty() {
                continue;
            }
            while self.vertices[v].adjacents.len() == 1 {
                let next = self.first_adjacent(v);
                self.remove(v, next);
                v = next;
            }
        }

        component.retain(|&v| !self.vertices[v].adjacents.is_empty());
    }

    fn extract_cycle_from_component(
        &mut self,
        component: &mut Vec<usize>,
    ) -> Option<Tree> {
        // The left-most vertex (ties broken by lowest y) is on the
        // component's outer boundary.
        let start = *component.iter().min_by(|&&a, &&b| {
            let (a, b) = (self.pos(a), self.pos(b));
            a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]))
        })?;

        let limit = self
            .vertices
            .iter()
            .map(|v| v.adjacents.len())
            .sum::<usize>()
            + 2;

        let mut closed_walk = vec![start];
        let mut curr = start;
        let mut adj = self.clockwise_most(None, start)?;
        while adj != start {
            if closed_walk.len() > limit {
                return None;
            }
            closed_walk.push(adj);
            let next = self.counterclockwise_most(Some(curr), adj)?;
            curr = adj;
            adj = next;
        }
        closed_walk.push(start);

        let tree = self.extract_cycle_from_closed_walk(closed_walk);
        component.retain(|&v| !self.vertices[v].adjacents.is_empty());
        Some(tree)
    }

    fn extract_cycle_from_closed_walk(
        &mut self,
        mut closed_walk: Vec<usize>,
    ) -> Tree {
        let mut tree = Tree::default();

        // Collapse the walk wherever it revisits a vertex, remembering where
        // the removed sub-walks were attached.
        let mut first_visit: HashMap<usize, usize> = HashMap::new();
        let mut detachments: BTreeSet<usize> = BTreeSet::new();
        let mut n = closed_walk.len();
        let mut i = 1;
        while i + 1 < n {
            let v = closed_walk[i];
            let Some(&i_min) = first_visit.get(&v) else {
                first_visit.insert(v, i);
                i += 1;
                continue;
            };
            detachments.insert(i_min);
            for (j, vertex) in
                closed_walk.iter().enumerate().take(i).skip(i_min + 1)
            {
                first_visit.remove(vertex);
                detachments.remove(&j);
            }
            closed_walk.drain(i_min + 1..=i);
            n = closed_walk.len();
            i = i_min + 1;
        }

        if n > 3 {
            // The walk's start may also hold a detached subgraph; the wedge
            // test below sorts that out, so consider it too.
            detachments.insert(0);

            for &i in &detachments {
                let origin = closed_walk[i];
                let to_next = closed_walk[i + 1];
                let to_prev = if i > 0 {
                    closed_walk[i - 1]
                } else {
                    closed_walk[n - 2]
                };

                let origin_pos = self.pos(origin);
                let d_next = sub(self.pos(to_next), origin_pos);
                let d_prev = sub(self.pos(to_prev), origin_pos);
                let is_convex = det(d_next, d_prev) >= 0.0;

                let prev_index = self.vertices[to_prev].index;
                let next_index = self.vertices[to_next].index;
                // Adjacent vertices strictly inside the wedge between the
                // cycle's incoming and outgoing edges at this vertex.
                let in_wedge: Vec<usize> = self.vertices[origin]
                    .adjacents
                    .iter()
                    .copied()
                    .filter(|&v| {
                        let index = self.vertices[v].index;
                        if index == prev_index || index == next_index
                        {
                            return false;
                        }
                        let d = sub(self.pos(v), origin_pos);
                        let s0 = det(d, d_prev);
                        let s1 = det(d, d_next);
                        if is_convex {
                            s0 > 0.0 && s1 < 0.0
                        } else {
                            s0 > 0.0 || s1 < 0.0
                        }
                    })
                    .collect();

                if !in_wedge.is_empty() {
                    // Hand the wedge's edges to a clone, cutting the nested
                    // subgraph loose, and recurse into it.
                    let clone = self.clone_vertex(origin);
                    for v in in_wedge {
                        self.remove(v, origin);
                        self.insert(v, clone);
                    }
                    let mut component =
                        self.detached_component(clone);
                    tree.children
                        .push(self.extract_basis(&mut component));
                }
            }

            tree.cycle = self.extract_cycle(&closed_walk);
        } else {
            // The walk is start -> v -> start: a subgraph hanging off a
            // filament. Detach it and recurse.
            let current = closed_walk[0];
            let next = closed_walk[1];
            let clone = self.clone_vertex(current);
            self.remove(next, current);
            self.insert(next, clone);
            let mut component = self.detached_component(clone);
            tree.children.push(self.extract_basis(&mut component));
            collapse_single_child(&mut tree);
        }

        tree
    }

    /// Records the cycle, then removes its edges that no other cycle shares.
    fn extract_cycle(&mut self, closed_walk: &[usize]) -> Vec<usize> {
        let cycle = closed_walk
            .iter()
            .map(|&v| self.vertices[v].index)
            .collect();

        // The clockwise-most edge is always removable.
        let mut v0 = closed_walk[0];
        let mut v1 = closed_walk[1];
        let branch =
            (self.vertices[v0].adjacents.len() > 2).then_some(v0);
        self.remove(v0, v1);

        // Peel edges counterclockwise until a branch vertex.
        while Some(v1) != branch
            && self.vertices[v1].adjacents.len() == 1
        {
            let next = self.first_adjacent(v1);
            self.remove(v1, next);
            v1 = next;
        }

        if v1 != v0 {
            // Stop at v1: a filament newly exposed there is left for the
            // caller's filament removal.
            let branch = v1;
            while v0 != branch
                && self.vertices[v0].adjacents.len() == 1
            {
                v1 = self.first_adjacent(v0);
                self.remove(v0, v1);
                v0 = v1;
            }
        }

        cycle
    }

    fn clockwise_most(
        &self,
        prev: Option<usize>,
        curr: usize,
    ) -> Option<usize> {
        let d_curr = match prev {
            Some(p) => sub(self.pos(curr), self.pos(p)),
            None => [0.0, -1.0],
        };

        let mut next = None;
        let mut d_next = [0.0; 2];
        let mut convex = false;
        for &adj in &self.vertices[curr].adjacents {
            // No backtracking.
            if Some(adj) == prev {
                continue;
            }
            let d_adj = sub(self.pos(adj), self.pos(curr));
            if next.is_none() {
                next = Some(adj);
                d_next = d_adj;
                convex = det(d_next, d_curr) <= 0.0;
                continue;
            }

            let s0 = det(d_curr, d_adj);
            let s1 = det(d_next, d_adj);
            if convex {
                if s0 < 0.0 || s1 < 0.0 {
                    next = Some(adj);
                    d_next = d_adj;
                    convex = det(d_next, d_curr) <= 0.0;
                }
            } else if s0 < 0.0 && s1 < 0.0 {
                next = Some(adj);
                d_next = d_adj;
                convex = det(d_next, d_curr) < 0.0;
            }
        }
        next
    }

    fn counterclockwise_most(
        &self,
        prev: Option<usize>,
        curr: usize,
    ) -> Option<usize> {
        let d_curr = match prev {
            Some(p) => sub(self.pos(curr), self.pos(p)),
            None => [0.0, -1.0],
        };

        let mut next = None;
        let mut d_next = [0.0; 2];
        let mut convex = false;
        for &adj in &self.vertices[curr].adjacents {
            // No backtracking.
            if Some(adj) == prev {
                continue;
            }
            let d_adj = sub(self.pos(adj), self.pos(curr));
            if next.is_none() {
                next = Some(adj);
                d_next = d_adj;
                convex = det(d_next, d_curr) <= 0.0;
                continue;
            }

            let s0 = det(d_curr, d_adj);
            let s1 = det(d_next, d_adj);
            let take = if convex {
                s0 > 0.0 && s1 > 0.0
            } else {
                s0 > 0.0 || s1 > 0.0
            };
            if take {
                next = Some(adj);
                d_next = d_adj;
                convex = det(d_next, d_curr) <= 0.0;
            }
        }
        next
    }
}

/// Replaces a parent holding nothing but a single child with that child.
fn collapse_single_child(tree: &mut Tree) {
    if tree.cycle.is_empty() && tree.children.len() == 1 {
        let child = tree.children.pop().unwrap();
        tree.cycle = child.cycle;
        tree.children = child.children;
    }
}

fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}

fn det(a: Point, b: Point) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
