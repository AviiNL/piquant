use valence::prelude::{BlockPos, BlockState, DVec3, Instance, Position};

#[allow(unused)]
#[derive(Debug)]
pub enum RayHit {
    Entity(Position), // todo: add more info
    Block {
        state: BlockState,
        pos: BlockPos,
        offset: DVec3,
    },
}

#[derive(Debug)]
pub struct Ray {
    pub origin: DVec3,
    pub direction: DVec3,
    pub length: f64,
}

impl Ray {
    pub fn new(origin: DVec3, direction: DVec3, length: f64) -> Self {
        Self {
            origin,
            direction,
            length,
        }
    }

    pub fn at(&self, t: f64) -> DVec3 {
        self.origin + self.direction * t
    }
}

pub fn raycast(ray: Ray, instance: &Instance) -> Vec<RayHit> {
    let mut hits = Vec::new();

    let mut t = 0.0;
    let step = 0.01;

    while t < ray.length {
        let pos = ray.at(t);

        let block_pos = BlockPos::at([pos.x, pos.y, pos.z]);

        if let Some(block) = instance.block(block_pos) {
            if hits.iter().any(|hit| {
                if let RayHit::Block { pos, .. } = hit {
                    *pos == block_pos
                } else {
                    false
                }
            }) {
                t += step;
                continue;
            }

            let block_pos_as_dvec3 =
                DVec3::new(block_pos.x as f64, block_pos.y as f64, block_pos.z as f64);

            hits.push(RayHit::Block {
                state: block.state(),
                pos: block_pos,
                offset: pos - block_pos_as_dvec3,
            });
        }

        t += step;
    }

    hits
}
