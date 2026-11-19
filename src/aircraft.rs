use std::{io, path::Path};

use crate::terrain::{Image, ResourceArchive, decode_image};

type Point = [f32; 3];

#[derive(Clone, Copy)]
struct Vertex {
    position: Point,
    uv: [f32; 2],
}

struct Material {
    texture: Option<Image>,
    transparent: bool,
    base_color: u32,
}

#[derive(Clone, Copy)]
enum Motion {
    Fixed,
    MainRotor,
    TailRotor,
}

struct Face {
    vertices: [Vertex; 3],
    material: usize,
    attachment: Point,
    motion: Motion,
}

pub(crate) struct Aircraft {
    faces: Vec<Face>,
    materials: Vec<Material>,
}

impl Aircraft {
    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        Self::read(path).map_err(|error| format!("Aircraft: {error}"))
    }

    fn read(path: &Path) -> io::Result<Self> {
        let mut archive = ResourceArchive::open(path)?;
        // PLAYER.AI supplies the desert parts. The body stores the attachment
        // positions for the closed gear doors, weapon bay, nose and tail rotor.
        let body = archive.read(b"CMBH_BOD.3DO")?;
        let mut aircraft = Self {
            faces: Vec::new(),
            materials: Vec::new(),
        };
        aircraft.add_part(&mut archive, &body, [0.0; 3], Motion::Fixed)?;
        let child_start = number(&body, 84) as usize;
        for (name, slot, motion) in [
            ("CMAH_BL1.3DO", None, Motion::MainRotor),
            ("CMAH_RTR.3DO", Some(2), Motion::TailRotor),
            ("CMBH_LDR.3DO", Some(3), Motion::Fixed),
            ("CMBH_LDL.3DO", Some(4), Motion::Fixed),
            ("CMBH_LDK.3DO", Some(5), Motion::Fixed),
            ("CMBH_MDR.3DO", Some(6), Motion::Fixed),
            ("CMBH_MDL.3DO", Some(7), Motion::Fixed),
            ("CMBH_LDJ.3DO", Some(11), Motion::Fixed),
            ("CMAH_GUN.3DO", Some(12), Motion::Fixed),
            ("CMBH_BOX.3DO", Some(12), Motion::Fixed),
            ("CMAH_NOS.3DO", Some(17), Motion::Fixed),
        ] {
            let attachment = match slot {
                Some(slot) => {
                    let start = child_start
                        .checked_add(slot * 12)
                        .ok_or_else(|| invalid("invalid 3DO attachment"))?;
                    let bytes = body
                        .get(start..start.saturating_add(12))
                        .ok_or_else(|| invalid("invalid 3DO attachment"))?;
                    model_point(bytes)
                }
                None => [0.0; 3],
            };
            let data = archive.read(name.as_bytes())?;
            aircraft
                .add_part(&mut archive, &data, attachment, motion)
                .map_err(|error| invalid(&format!("{name}: {error}")))?;
        }
        Ok(aircraft)
    }

    fn add_part(
        &mut self,
        archive: &mut ResourceArchive,
        data: &[u8],
        attachment: Point,
        motion: Motion,
    ) -> io::Result<()> {
        if data.len() < 92 || !data.starts_with(b"3DO1") {
            return Err(invalid("expected a 3DO1 model"));
        }
        let [material_start, vertex_start, face_start, normal_start] =
            [68, 72, 76, 80].map(|offset| number(data, offset) as usize);
        let [material_bytes, vertex_bytes, face_bytes] = [
            (material_start, vertex_start, 28),
            (vertex_start, face_start, 44),
            (face_start, normal_start, 32),
        ]
        .map(|(start, end, stride)| {
            data.get(start..end)
                .filter(|bytes| bytes.len().is_multiple_of(stride))
                .ok_or_else(|| invalid("invalid 3DO section"))
        });
        let material_base = self.materials.len();
        for entry in material_bytes?.as_chunks::<28>().0 {
            let name = entry[..12].split(|&b| b == 0).next().unwrap_or_default();
            let texture = if name.is_empty() {
                None
            } else {
                let data = archive.read(name)?;
                Some(if data.starts_with(b"LZP1") {
                    decode_image(&data)?
                } else {
                    decode_pcx(&data)?
                })
            };
            self.materials.push(Material {
                texture,
                transparent: entry[15] == 128,
                base_color: if entry[16..19] == [0, 0, 0] {
                    0x343631
                } else {
                    u32::from_be_bytes([0, entry[16], entry[17], entry[18]])
                },
            });
        }
        let vertices = vertex_bytes?;
        for entry in face_bytes?.as_chunks::<32>().0 {
            let material_offset = number(entry, 4) as usize;
            let material = material_base + material_offset / 28;
            if !material_offset.is_multiple_of(28) || material >= self.materials.len() {
                return Err(invalid("invalid 3DO material reference"));
            }
            let vertex = |offset| {
                let address = number(entry, offset) as usize;
                let bytes = vertices
                    .get(address..address.saturating_add(44))
                    .filter(|_| address.is_multiple_of(44))
                    .ok_or_else(|| invalid("invalid 3DO vertex reference"))?;
                Ok::<_, io::Error>(Vertex {
                    position: model_point(bytes),
                    uv: [32, 36].map(|offset| number(bytes, offset) as f32 / 65535.0),
                })
            };
            self.faces.push(Face {
                vertices: [vertex(8)?, vertex(12)?, vertex(16)?],
                material,
                attachment,
                motion,
            });
        }
        Ok(())
    }

    pub(crate) fn draw(
        &self,
        frame: &mut [u32],
        (width, height): (usize, usize),
        horizon: f32,
        focal: f32,
        camera: Point,
        rotor_angle: f32,
    ) {
        let (sin, cos) = rotor_angle.sin_cos();
        let [view_azimuth, camera_height, camera_distance] = camera;
        let (view_sin, view_cos) = view_azimuth.sin_cos();
        let mut triangles = Vec::with_capacity(self.faces.len());
        for face in &self.faces {
            let points = face.vertices.map(|vertex| {
                let [mut x, mut y, mut z] = vertex.position;
                match face.motion {
                    Motion::Fixed => {}
                    Motion::MainRotor => (x, z) = (x * cos + z * sin, z * cos - x * sin),
                    Motion::TailRotor => (y, z) = (y * cos - z * sin, y * sin + z * cos),
                }
                x += face.attachment[0];
                y += face.attachment[1];
                z += face.attachment[2];
                let side = x * view_cos - z * view_sin;
                let depth = x * view_sin + z * view_cos + camera_distance;
                [
                    width as f32 * 0.5 + focal * side / depth,
                    horizon + focal * (camera_height - y - 1.25) / depth,
                    depth,
                ]
            });
            triangles.push((points, face.vertices.map(|vertex| vertex.uv), face.material));
        }

        let mut depth = DepthBuffer::new(
            (width, height),
            triangles.iter().flat_map(|(points, _, _)| points),
        );
        for (points, uv, material) in triangles {
            fill_triangle(
                frame,
                width,
                points,
                uv,
                &self.materials[material],
                &mut depth,
            );
        }
    }
}

fn decode_pcx(data: &[u8]) -> io::Result<Image> {
    if data.len() < 897 || !data.starts_with(&[10, 5, 1, 8]) || data[65] != 1 {
        return Err(invalid("expected an indexed PCX texture"));
    }
    let word = |offset| usize::from(u16::from_le_bytes([data[offset], data[offset + 1]]));
    let width = (word(8) + 1).saturating_sub(word(4));
    let height = (word(10) + 1).saturating_sub(word(6));
    let stride = word(66);
    let palette_start = data.len() - 768;
    if width == 0
        || height == 0
        || width > stride
        || stride > 2048
        || height > 2048
        || data[palette_start - 1] != 12
    {
        return Err(invalid("invalid PCX dimensions or palette"));
    }
    let mut input = data[128..palette_start - 1].iter().copied();
    let mut decoded = Vec::with_capacity(stride * height);
    while decoded.len() < stride * height {
        let token = input
            .next()
            .ok_or_else(|| invalid("truncated PCX texture"))?;
        let (length, value) = if token >= 0xc0 {
            (
                usize::from(token & 0x3f),
                input.next().ok_or_else(|| invalid("truncated PCX run"))?,
            )
        } else {
            (1, token)
        };
        if length == 0 || decoded.len() + length > stride * height {
            return Err(invalid("invalid PCX run"));
        }
        decoded.resize(decoded.len() + length, value);
    }
    let palette = std::array::from_fn(|index| {
        let offset = palette_start + index * 3;
        u32::from_be_bytes([0, data[offset], data[offset + 1], data[offset + 2]])
    });
    let pixels = decoded
        .chunks_exact(stride)
        .flat_map(|row| row[..width].iter().copied())
        .collect();
    Ok(Image {
        width,
        height,
        pixels,
        palette,
    })
}

fn model_point(bytes: &[u8]) -> Point {
    let [x, y, z] = [0, 4, 8].map(|offset| number(bytes, offset) as f32 * 0.000004);
    [-x, z, -y]
}

struct DepthBuffer {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
    values: Vec<f32>,
}

impl DepthBuffer {
    fn new<'a>((width, height): (usize, usize), points: impl Iterator<Item = &'a Point>) -> Self {
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        for point in points {
            left = left.min(point[0].floor() as usize);
            top = top.min(point[1].floor() as usize);
            right = right.max(point[0].ceil() as usize).min(width);
            bottom = bottom.max(point[1].ceil() as usize).min(height);
        }
        let count = right.saturating_sub(left) * bottom.saturating_sub(top);
        Self {
            left,
            top,
            right,
            bottom,
            values: vec![0.0; count],
        }
    }

    fn at(&mut self, x: usize, y: usize) -> &mut f32 {
        &mut self.values[(y - self.top) * (self.right - self.left) + x - self.left]
    }
}

fn number(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(data[offset..offset + 4].try_into().expect("32-bit field"))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn edge(a: Point, b: Point, x: f32, y: f32) -> f32 {
    (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
}

fn fill_triangle(
    frame: &mut [u32],
    width: usize,
    [a, b, c]: [Point; 3],
    uv: [[f32; 2]; 3],
    material: &Material,
    depth: &mut DepthBuffer,
) {
    let area = edge(a, b, c[0], c[1]);
    if area.abs() < 0.01 {
        return;
    }
    let left = (a[0].min(b[0]).min(c[0]).floor() as usize).max(depth.left);
    let right = (a[0].max(b[0]).max(c[0]).ceil() as usize).min(depth.right);
    let top = (a[1].min(b[1]).min(c[1]).floor() as usize).max(depth.top);
    let bottom = (a[1].max(b[1]).max(c[1]).ceil() as usize).min(depth.bottom);
    for y in top..bottom {
        for x in left..right {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let weights = [edge(b, c, px, py), edge(c, a, px, py), edge(a, b, px, py)]
                .map(|value| value / area);
            if weights.iter().any(|&weight| weight < 0.0) {
                continue;
            }
            let weights = [weights[0] / a[2], weights[1] / b[2], weights[2] / c[2]];
            let sum = weights.iter().sum::<f32>();
            let nearest = depth.at(x, y);
            if sum <= *nearest {
                continue;
            }
            let color = if let Some(texture) = &material.texture {
                let interpolate = |axis: usize| {
                    ((uv[0][axis] * weights[0]
                        + uv[1][axis] * weights[1]
                        + uv[2][axis] * weights[2])
                        / sum)
                        .clamp(0.0, 1.0)
                };
                let u = (interpolate(0) * (texture.width - 1) as f32).round() as usize;
                let v = (interpolate(1) * (texture.height - 1) as f32).round() as usize;
                let index = texture.pixels[v * texture.width + u];
                if index == 0 && (material.transparent || texture.palette[0] == 0x0000ff) {
                    continue;
                }
                texture.palette[usize::from(index)]
            } else {
                material.base_color
            };
            *nearest = sum;
            frame[y * width + x] = color;
        }
    }
}
