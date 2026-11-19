use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

// Format references: Sioux's pup/res.c, tools/pcx.c and include/decompress.h.
const ARCHIVE_HEADER_SIZE: usize = 36;
const ARCHIVE_ENTRY_SIZE: usize = 20;
const IMAGE_HEADER_SIZE: usize = 0x710;
const PALETTE_SIZE: usize = 256 * 3;
const MAX_IMAGE_PIXELS: usize = 4096 * 4096;

pub struct Terrain {
    pub width: usize,
    pub height: usize,
    elevations: Vec<u8>,
    color_indices: Vec<u8>,
    palette: [u32; 256],
}

impl Terrain {
    pub fn load(path: &Path) -> Result<Self, String> {
        Self::read(path).map_err(|error| format!("{}: {error}", path.display()))
    }

    fn read(path: &Path) -> io::Result<Self> {
        let mut archive = ResourceArchive::open(path)?;
        let color = decode_image(&archive.read(b"C1M3_C.PCX")?)?;
        let elevation = decode_image(&archive.read(b"C1M3_D.PCX")?)?;
        if (color.width, color.height) != (elevation.width, elevation.height) {
            return Err(invalid("terrain layers have different dimensions"));
        }

        Ok(Self {
            width: color.width,
            height: color.height,
            elevations: elevation.pixels,
            color_indices: color.pixels,
            palette: color.palette,
        })
    }

    pub fn sample(&self, x: f32, z: f32) -> (f32, u32) {
        let column = (x.floor() as isize).rem_euclid(self.width as isize) as usize;
        let row = (z.floor() as isize).rem_euclid(self.height as isize) as usize;
        let index = row * self.width + column;
        let height = f32::from(self.elevations[index]) * (60.0 / 255.0);
        (height, self.palette[usize::from(self.color_indices[index])])
    }
}

pub(crate) struct ResourceArchive {
    file: File,
    file_size: u64,
    directory_end: u64,
}

impl ResourceArchive {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let file_size = file.metadata()?.len();
        let mut header = [0; ARCHIVE_HEADER_SIZE];
        file.read_exact(&mut header)?;
        if &header[..12] != b"RESOURCE2xxx" {
            return Err(invalid("expected a RESOURCE2xxx archive"));
        }
        let count = read_u32(&header[12..16])
            .checked_sub(1)
            .ok_or_else(|| invalid("invalid archive entry count"))?;
        let directory_end =
            ARCHIVE_HEADER_SIZE as u64 + u64::from(count) * ARCHIVE_ENTRY_SIZE as u64;
        if directory_end > file_size {
            return Err(invalid("archive directory extends beyond the file"));
        }
        Ok(Self {
            file,
            file_size,
            directory_end,
        })
    }

    pub(crate) fn mission_names(&mut self) -> io::Result<Vec<String>> {
        self.rewind_directory()?;
        let mut names = Vec::new();
        for _ in 0..self.entry_count() {
            let entry = self.read_directory_entry()?;
            if entry
                .name()
                .rsplit_once('.')
                .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("mis"))
            {
                names.push(entry.name().to_ascii_lowercase());
            }
        }
        names.sort_unstable();
        if names.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(invalid("duplicate mission names in the archive"));
        }
        Ok(names)
    }

    pub(crate) fn read(&mut self, target: &[u8]) -> io::Result<Vec<u8>> {
        self.rewind_directory()?;
        for _ in 0..self.entry_count() {
            let entry = self.read_directory_entry()?;
            if !entry.name().as_bytes().eq_ignore_ascii_case(target) {
                continue;
            }
            let offset = entry.offset;
            let size = entry.size;
            if offset < self.directory_end
                || offset + size as u64 > self.file_size
                || size > 2 * MAX_IMAGE_PIXELS
            {
                return Err(invalid("invalid archive entry range"));
            }
            self.file.seek(SeekFrom::Start(offset))?;
            let mut data = vec![0; size];
            self.file.read_exact(&mut data)?;
            return Ok(data);
        }
        Err(invalid(format!(
            "{} is missing from the archive",
            String::from_utf8_lossy(target)
        )))
    }

    fn entry_count(&self) -> u64 {
        (self.directory_end - ARCHIVE_HEADER_SIZE as u64) / ARCHIVE_ENTRY_SIZE as u64
    }

    fn rewind_directory(&mut self) -> io::Result<()> {
        self.file
            .seek(SeekFrom::Start(ARCHIVE_HEADER_SIZE as u64))?;
        Ok(())
    }

    fn read_directory_entry(&mut self) -> io::Result<ArchiveEntry> {
        let mut bytes = [0; ARCHIVE_ENTRY_SIZE];
        self.file.read_exact(&mut bytes)?;
        let name = std::array::from_fn(|index| bytes[index] ^ [0xad, 0xde, 0xed, 0xac][index % 4]);
        Ok(ArchiveEntry {
            name,
            offset: u64::from(read_u32(&bytes[12..16])) + ARCHIVE_ENTRY_SIZE as u64,
            size: read_u32(&bytes[16..20]) as usize,
        })
    }
}

struct ArchiveEntry {
    name: [u8; 12],
    offset: u64,
    size: usize,
}

impl ArchiveEntry {
    fn name(&self) -> &str {
        let bytes = self
            .name
            .split(|&byte| byte == 0)
            .next()
            .unwrap_or_default();
        std::str::from_utf8(bytes).unwrap_or_default()
    }
}

pub(crate) struct Image {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: Vec<u8>,
    pub(crate) palette: [u32; 256],
}

pub(crate) fn decode_image(data: &[u8]) -> io::Result<Image> {
    if data.len() < IMAGE_HEADER_SIZE || &data[..4] != b"LZP1" {
        return Err(invalid("expected an LZP1 image"));
    }
    let width = read_u32(&data[4..8]) as usize;
    let height = read_u32(&data[8..12]) as usize;
    let pixel_count = width
        .checked_mul(height)
        .filter(|&count| count != 0 && count <= MAX_IMAGE_PIXELS)
        .ok_or_else(|| invalid("invalid image dimensions"))?;

    let mut decoded = decode_lzp1(&data[IMAGE_HEADER_SIZE..], pixel_count + PALETTE_SIZE + 24)?;
    if decoded.len() < pixel_count {
        return Err(invalid("pixel data ended early"));
    }

    // Some images include a replacement palette after the pixel stream.
    let footer = &decoded[pixel_count..];
    let embedded_palette = footer
        .starts_with(b"LZP1")
        .then(|| {
            (12..=20).find_map(|offset| {
                let end = offset + PALETTE_SIZE;
                (footer.get(end..end + 4) == Some(b"CerP")).then(|| &footer[offset..end])
            })
        })
        .flatten();
    let rgb = embedded_palette.unwrap_or(&data[12..12 + PALETTE_SIZE]);
    let palette = std::array::from_fn(|index| {
        let offset = index * 3;
        u32::from_be_bytes([0, rgb[offset], rgb[offset + 1], rgb[offset + 2]])
    });
    decoded.truncate(pixel_count);
    Ok(Image {
        width,
        height,
        pixels: decoded,
        palette,
    })
}

fn decode_lzp1(compressed: &[u8], limit: usize) -> io::Result<Vec<u8>> {
    const CLEAR: u16 = 256;
    const END: u16 = 257;
    const FIRST_ENTRY: u16 = 258;
    const DICTIONARY_SIZE: usize = 8192;

    let mut dictionary = [(0_u16, 0_u8); DICTIONARY_SIZE];
    let mut next_entry = FIRST_ENTRY;
    let mut code_width = 9;
    let mut prior_code = None;
    let mut bits = 0_u32;
    let mut available_bits = 0;
    let mut bytes = compressed.iter().copied();
    let mut output = Vec::with_capacity(limit);
    let mut suffixes = Vec::new();

    while output.len() < limit {
        while available_bits < code_width {
            let byte = bytes
                .next()
                .ok_or_else(|| invalid("truncated LZP1 stream"))?;
            bits |= u32::from(byte) << available_bits;
            available_bits += 8;
        }
        let code = (bits & ((1 << code_width) - 1)) as u16;
        bits >>= code_width;
        available_bits -= code_width;

        match code {
            CLEAR => {
                next_entry = FIRST_ENTRY;
                code_width = 9;
                prior_code = None;
                continue;
            }
            END => break,
            _ => {}
        }

        let mut prefix = code;
        suffixes.clear();
        if code == next_entry {
            let (previous, first) =
                prior_code.ok_or_else(|| invalid("LZP1 code refers to an empty dictionary"))?;
            suffixes.push(first);
            prefix = previous;
        } else if code > next_entry {
            return Err(invalid("LZP1 code exceeds the dictionary"));
        }
        while prefix >= FIRST_ENTRY {
            let (parent, suffix) = dictionary[usize::from(prefix)];
            if parent >= prefix {
                return Err(invalid("invalid LZP1 dictionary chain"));
            }
            suffixes.push(suffix);
            prefix = parent;
        }
        let first = u8::try_from(prefix).map_err(|_| invalid("invalid LZP1 literal"))?;
        output.push(first);
        output.extend(suffixes.iter().rev().copied().take(limit - output.len()));

        if let Some((previous, _)) = prior_code
            && usize::from(next_entry) < DICTIONARY_SIZE
        {
            dictionary[usize::from(next_entry)] = (previous, first);
            next_entry += 1;
            if next_entry == 1 << code_width && code_width < 13 {
                code_width += 1;
            }
        }
        prior_code = Some((code, first));
    }
    Ok(output)
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("four-byte field"))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
