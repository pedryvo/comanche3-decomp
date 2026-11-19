use crate::{aircraft::Aircraft, flight::Flight, terrain::Terrain};

const SKY_TOP: u32 = 0x2b5278;
const SKY_HORIZON: u32 = 0xb8b5aa;
const MARKER: u32 = 0xc6ff84;
const MARKER_SHADOW: u32 = 0x142129;
const DRAW_DISTANCE: f32 = 720.0;
pub(crate) const FIRST_PERSON_HORIZON: f32 = 0.43;

const ORBIT_DISTANCE: f32 = 24.0;

pub(crate) struct Orbit {
    azimuth: f32,
    elevation: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            azimuth: -0.2,
            elevation: 0.16,
        }
    }
}

impl Orbit {
    pub(crate) fn rotate(&mut self, azimuth: f32, elevation: f32) {
        self.azimuth = (self.azimuth + azimuth).rem_euclid(std::f32::consts::TAU);
        self.elevation = (self.elevation + elevation).clamp(0.03, 0.65);
    }
}

pub(crate) struct ExternalView<'a> {
    pub aircraft: &'a Aircraft,
    pub orbit: &'a Orbit,
    pub rotor_angle: f32,
}

/// Draw a perspective heightfield from front to back, filling the visible spans
/// of each column. Color is packed as 0x00RRGGBB.
pub fn draw(
    frame: &mut [u32],
    (width, height): (usize, usize),
    terrain: &Terrain,
    flight: &Flight,
    chase: Option<ExternalView<'_>>,
) {
    if width < 32 || height < 32 {
        frame.fill(SKY_TOP);
        return;
    }

    let focal = height as f32 * 0.93;
    let (azimuth, elevation) = chase.as_ref().map_or((0.0, 0.0), |view| {
        (view.orbit.azimuth, view.orbit.elevation)
    });
    let (sin, cos) = (flight.yaw + azimuth).sin_cos();
    let rear_offset = if chase.is_some() { ORBIT_DISTANCE } else { 0.0 };
    let camera_x = flight.x - rear_offset * sin;
    let camera_z = flight.z - rear_offset * cos;
    let camera_altitude = if chase.is_some() {
        (flight.altitude + 1.25 + ORBIT_DISTANCE * elevation.tan())
            .max(terrain.sample(camera_x, camera_z).0 + 2.5)
    } else {
        flight.altitude
    };
    // Shift the projection so the aircraft stays centered as elevation changes.
    let horizon = if chase.is_some() {
        height as f32 * 0.52 - focal * (camera_altitude - flight.altitude - 1.25) / rear_offset
    } else {
        height as f32 * FIRST_PERSON_HORIZON
    };
    for (y, row) in frame.chunks_exact_mut(width).enumerate() {
        let haze = (y as f32 / horizon.max(1.0)).clamp(0.0, 1.0);
        row.fill(blend(SKY_TOP, SKY_HORIZON, haze));
    }

    for x in 0..width {
        let side = (x as f32 + 0.5 - width as f32 * 0.5) / focal;
        let ray_x = sin + side * cos;
        let ray_z = cos - side * sin;
        let mut span_bottom = height;
        let mut depth: f32 = 1.0;
        while depth < DRAW_DISTANCE && span_bottom > 0 {
            let world_x = camera_x + depth * ray_x;
            let world_z = camera_z + depth * ray_z;
            let (elevation, color) = terrain.sample(world_x, world_z);
            let top =
                project_y(horizon, focal, camera_altitude - elevation, depth).max(0.0) as usize;
            if top < span_bottom {
                let haze = ((depth - 80.0) / (DRAW_DISTANCE - 80.0)).clamp(0.0, 1.0);
                let color = blend(color, SKY_HORIZON, haze * haze);
                for y in top..span_bottom {
                    frame[y * width + x] = color;
                }
                span_bottom = top;
            }
            // Increasing spacing saves work as perspective shrinks the far terrain.
            depth += 0.8 + depth * 0.024;
        }
    }

    if let Some(view) = chase {
        view.aircraft.draw(
            frame,
            (width, height),
            horizon,
            focal,
            [azimuth, camera_altitude - flight.altitude, rear_offset],
            view.rotor_angle,
        );
    }

    if !flight.complete {
        let dx = flight.waypoint_x - camera_x;
        let dz = flight.waypoint_z - camera_z;
        let depth = dx * sin + dz * cos;
        let side = dx * cos - dz * sin;
        let elevation = terrain.sample(flight.waypoint_x, flight.waypoint_z).0 + 6.0;
        let projected_x = width as f32 * 0.5 + focal * side / depth.max(1.0);
        let size = (height / 94).clamp(5, 22) as isize;
        let margin = (size + 4) as f32;
        let right_edge = (width as f32 - margin).max(margin);
        let bottom_edge = (height as f32 - margin).max(margin);

        let (x, y) = if depth < 1.0 || projected_x < margin || projected_x > right_edge {
            // Keep the waypoint bearing visible while the target is outside the view.
            let x = if side < 0.0 { margin } else { right_edge };
            (x, horizon.clamp(margin, bottom_edge))
        } else {
            let y = project_y(horizon, focal, camera_altitude - elevation, depth);
            (projected_x, y.clamp(margin, bottom_edge))
        };
        draw_marker(frame, width, height, x as isize, y as isize, size);
    }
}

fn project_y(horizon: f32, focal: f32, delta_height: f32, depth: f32) -> f32 {
    horizon + focal * delta_height / depth
}

fn blend(from: u32, to: u32, amount: f32) -> u32 {
    let alpha = (amount * 256.0) as u32;
    let red_blue = ((from & 0xff00ff) * (256 - alpha) + (to & 0xff00ff) * alpha) >> 8;
    let green = ((from & 0x00ff00) * (256 - alpha) + (to & 0x00ff00) * alpha) >> 8;
    (red_blue & 0xff00ff) | (green & 0x00ff00)
}

fn draw_marker(frame: &mut [u32], width: usize, height: usize, x: isize, y: isize, size: isize) {
    for dy in -size - 2..=size + 2 {
        for dx in -size - 2..=size + 2 {
            let edge = (dx.abs() + dy.abs() - size).abs();
            if edge > 2 {
                continue;
            }
            let px = x + dx;
            let py = y + dy;
            if (0..width as isize).contains(&px) && (0..height as isize).contains(&py) {
                frame[py as usize * width + px as usize] =
                    if edge <= 1 { MARKER } else { MARKER_SHADOW };
            }
        }
    }
}
