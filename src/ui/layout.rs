//! Minimal force-directed layout: spring links, Coulomb repulsion, centre gravity, Verlet-ish damping.
use crate::graph::{Graph, NodeType};
use egui::{Pos2, Vec2};

pub struct Body {
    pub id: String,
    pub pos: Pos2,
    pub vel: Vec2,
    pub r: f32,
    pub pinned: bool,
}

pub struct Layout {
    pub bodies: Vec<Body>,
    pub links: Vec<(usize, usize)>,
    pub alpha: f32,
}

pub fn radius(kind: NodeType) -> f32 {
    match kind {
        NodeType::Repo => 14.0,
        NodeType::Agent => 12.0,
        NodeType::Service => 11.0,
        NodeType::Container => 10.0,
        _ => 8.0,
    }
}

impl Layout {
    pub fn new(g: &Graph, centre: Pos2) -> Self {
        let n = g.nodes.len().max(1) as f32;
        let bodies: Vec<Body> = g
            .nodes
            .iter()
            .enumerate()
            .map(|(i, nd)| {
                // Phyllotaxis start so the simulation does not begin in a singularity.
                let a = i as f32 * 2.399963;
                let d = 14.0 * (i as f32).sqrt() * (400.0 / n.sqrt()).min(40.0) / 10.0;
                Body {
                    id: nd.id.clone(),
                    pos: centre + Vec2::angled(a) * d,
                    vel: Vec2::ZERO,
                    r: radius(nd.kind),
                    pinned: false,
                }
            })
            .collect();
        let idx = |id: &str| g.nodes.iter().position(|n| n.id == id);
        let links = g
            .edges
            .iter()
            .filter_map(|e| Some((idx(&e.from)?, idx(&e.to)?)))
            .collect();
        Self {
            bodies,
            links,
            alpha: 1.0,
        }
    }

    pub fn index(&self, id: &str) -> Option<usize> {
        self.bodies.iter().position(|b| b.id == id)
    }

    pub fn step(&mut self, centre: Pos2, visible: &[bool]) {
        if self.alpha < 0.002 {
            return;
        }
        let n = self.bodies.len();
        let mut force = vec![Vec2::ZERO; n];
        // Repulsion (O(n^2); fine for a few hundred nodes).
        for i in 0..n {
            if !visible[i] {
                continue;
            }
            for j in (i + 1)..n {
                if !visible[j] {
                    continue;
                }
                let d = self.bodies[i].pos - self.bodies[j].pos;
                let dist2 = d.length_sq().max(25.0);
                let f = d / dist2.sqrt() * (9000.0 / dist2);
                force[i] += f;
                force[j] -= f;
                // Collision padding.
                let min = self.bodies[i].r + self.bodies[j].r + 26.0;
                if dist2.sqrt() < min {
                    let push = d.normalized() * (min - dist2.sqrt()) * 0.5;
                    force[i] += push;
                    force[j] -= push;
                }
            }
        }
        // Springs.
        for &(a, b) in &self.links {
            if !visible[a] || !visible[b] {
                continue;
            }
            let d = self.bodies[b].pos - self.bodies[a].pos;
            let len = d.length().max(0.01);
            let f = d / len * (len - 110.0) * 0.08;
            force[a] += f;
            force[b] -= f;
        }
        // Gravity + integrate.
        for i in 0..n {
            if !visible[i] {
                continue;
            }
            let b = &mut self.bodies[i];
            let g = (centre - b.pos) * 0.012;
            if b.pinned {
                b.vel = Vec2::ZERO;
                continue;
            }
            b.vel = (b.vel + (force[i] + g) * self.alpha) * 0.6;
            b.pos += b.vel;
        }
        self.alpha *= 0.985;
    }

    pub fn reheat(&mut self) {
        self.alpha = self.alpha.max(0.4);
    }
}
