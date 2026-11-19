use crate::terrain::Terrain;

const LANDED_CLEARANCE: f32 = 2.0;
const MAX_CLEARANCE: f32 = 160.0;

#[derive(Default)]
pub struct Controls {
    pub accelerate: bool,
    pub brake: bool,
    pub left: bool,
    pub right: bool,
    pub climb: bool,
    pub descend: bool,
}

pub struct Flight {
    pub x: f32,
    pub z: f32,
    pub altitude: f32,
    pub yaw: f32,
    pub waypoint_x: f32,
    pub waypoint_z: f32,
    pub complete: bool,
    pub speed: f32,
    clearance: f32,
}

impl Flight {
    pub fn new(terrain: &Terrain) -> Self {
        let x = terrain.width as f32 * 0.45;
        let z = terrain.height as f32 * 0.35;
        let waypoint_z = (z + 100.0).min(terrain.height as f32 * 0.7);
        Self {
            x,
            z,
            altitude: terrain.sample(x, z).0 + LANDED_CLEARANCE,
            yaw: 0.0,
            waypoint_x: x,
            waypoint_z,
            complete: false,
            speed: 0.0,
            clearance: LANDED_CLEARANCE,
        }
    }

    pub fn step(&mut self, terrain: &Terrain, controls: Controls, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);
        let turn = axis(controls.right, controls.left);
        let lift = axis(controls.climb, controls.descend);
        self.yaw = (self.yaw + turn * 1.4 * dt).rem_euclid(std::f32::consts::TAU);
        self.clearance = (self.clearance + lift * 25.0 * dt).clamp(LANDED_CLEARANCE, MAX_CLEARANCE);

        if self.is_airborne() {
            let throttle = axis(controls.accelerate, controls.brake);
            self.speed = (self.speed + throttle * 22.0 * dt).clamp(0.0, 45.0);
            let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
            self.x = (self.x + sin_yaw * self.speed * dt).rem_euclid(terrain.width as f32);
            self.z = (self.z + cos_yaw * self.speed * dt).rem_euclid(terrain.height as f32);
        } else {
            self.speed = 0.0;
        }
        self.altitude = terrain.sample(self.x, self.z).0 + self.clearance;
    }

    pub(crate) fn is_airborne(&self) -> bool {
        self.clearance > LANDED_CLEARANCE + 1.0
    }
}

fn axis(positive: bool, negative: bool) -> f32 {
    i8::from(positive) as f32 - i8::from(negative) as f32
}
