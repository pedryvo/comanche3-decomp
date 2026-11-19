use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

pub(crate) fn check(directory: &Path) -> Result<(), String> {
    let mut videos = 0;
    for name in ["HINTRO.KDV", "LINTRO.KDV", "HOVRVIEW.KDV", "LOVRVIEW.KDV"] {
        let path = directory.join(name);
        if !path.is_file() {
            continue;
        }
        match inspect(&path).map_err(|error| format!("{name}: {error}"))? {
            Some(segments) => {
                videos += 1;
                for (index, segment) in segments.iter().enumerate() {
                    println!(
                        "{name} segment {}: {}x{}  {}",
                        index + 1,
                        segment.width,
                        segment.height,
                        segment
                            .tags
                            .iter()
                            .map(|(tag, count)| format!("{}={count}", String::from_utf8_lossy(tag)))
                            .collect::<Vec<_>>()
                            .join("  ")
                    );
                    if !segment.audio_layouts.is_empty() {
                        let description = segment
                            .audio_layouts
                            .iter()
                            .map(|(planes, samples)| {
                                format!("{planes} plane(s) of {samples} samples")
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        println!("  SAMP: {description}");
                    }
                }
            }
            None => println!("{name}: empty placeholder"),
        }
    }
    if videos == 0 {
        return Err("Nenhum arquivo KDV com video encontrado".into());
    }
    Ok(())
}

struct Segment {
    width: u32,
    height: u32,
    tags: BTreeMap<[u8; 4], u64>,
    audio_layouts: BTreeSet<(usize, u32)>,
}

fn inspect(path: &Path) -> Result<Option<Vec<Segment>>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let size = file.metadata().map_err(|error| error.to_string())?.len();
    let mut reader = BufReader::new(file);
    if size <= 2 {
        let mut placeholder = Vec::new();
        reader
            .read_to_end(&mut placeholder)
            .map_err(|error| error.to_string())?;
        if placeholder.is_empty() || placeholder == b"\r\n" {
            return Ok(None);
        }
    }

    let mut offset = 0;
    let mut segments = Vec::<Segment>::new();
    while offset < size {
        let mut header = [0; 8];
        reader
            .read_exact(&mut header)
            .map_err(|error| error.to_string())?;
        let tag: [u8; 4] = header[..4].try_into().expect("four-byte tag");
        let length = u64::from(number(&header[4..]));
        if length < 8 || length > size - offset {
            return Err(format!("invalid record length at byte {offset}"));
        }
        if tag == *b"xVDK" {
            if length < 16 {
                return Err(format!("invalid KDV segment header at byte {offset}"));
            }
            let mut dimensions = [0; 8];
            reader
                .read_exact(&mut dimensions)
                .map_err(|error| error.to_string())?;
            let width = number(&dimensions[..4]);
            let height = number(&dimensions[4..]);
            if width == 0 || height == 0 {
                return Err(format!("invalid KDV dimensions at byte {offset}"));
            }
            segments.push(Segment {
                width,
                height,
                tags: BTreeMap::new(),
                audio_layouts: BTreeSet::new(),
            });
        }
        let segment = segments.last_mut().ok_or("missing KDV header")?;
        *segment.tags.entry(tag).or_default() += 1;
        if matches!(&tag, b"aVDK" | b"sVDK") {
            let planes = if tag == *b"sVDK" { 2 } else { 1 };
            let count = read_sample_planes(&mut reader, offset + 8, offset + length, planes)
                .map_err(|error| format!("audio record at byte {offset}: {error}"))?;
            segment.audio_layouts.insert((planes, count));
        }
        offset += length;
        reader
            .seek(SeekFrom::Start(offset))
            .map_err(|error| error.to_string())?;
    }
    Ok(Some(segments))
}

fn read_sample_planes(
    reader: &mut (impl Read + Seek),
    mut position: u64,
    end: u64,
    planes: usize,
) -> Result<u32, String> {
    let mut sample_count = 0;
    for plane in 0..planes {
        if end - position < 16 {
            return Err("truncated SAMP header".into());
        }
        let mut header = [0; 16];
        reader
            .read_exact(&mut header)
            .map_err(|error| error.to_string())?;
        let count = number(&header[4..8]);
        if &header[..4] != b"SAMP"
            || count == 0
            || u64::from(count) > end - position - 16
            || (plane > 0 && count != sample_count)
        {
            return Err("invalid SAMP plane".into());
        }
        sample_count = count;
        position += 16 + u64::from(count);
        reader
            .seek(SeekFrom::Start(position))
            .map_err(|error| error.to_string())?;
    }
    if end - position > 3 {
        return Err("unexpected data after SAMP planes".into());
    }
    Ok(sample_count)
}

fn number(data: &[u8]) -> u32 {
    u32::from_le_bytes(data.try_into().expect("four-byte field"))
}
