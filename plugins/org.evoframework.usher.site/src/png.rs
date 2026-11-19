//! A PNG file, read as far as it takes to know that it is one, whole and
//! undamaged, and how large its picture is. Nothing of the picture is
//! unpacked: a logo is kept and given as the file it came as.

/// What every PNG file begins with.
pub const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// How large the picture of a PNG file is, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picture {
    /// How wide.
    pub width: u32,
    /// How high.
    pub height: u32,
}

/// Why a file is not taken for a PNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotOne {
    /// It does not begin as a PNG file does.
    NoPng,
    /// It begins as one, and is cut short or damaged after.
    Damaged,
}

/// The checksums of single bytes, by which a part's checksum is made: the
/// CRC-32 a PNG file closes each of its parts with.
const CHECKS: [u32; 256] = {
    let mut checks = [0u32; 256];
    let mut byte = 0;
    while byte < 256 {
        let mut check = byte as u32;
        let mut bit = 0;
        while bit < 8 {
            check = if check & 1 == 1 {
                0xEDB8_8320 ^ (check >> 1)
            } else {
                check >> 1
            };
            bit += 1;
        }
        checks[byte] = check;
        byte += 1;
    }
    checks
};

/// The checksum a PNG file closes a part with, over the part's name and
/// what it holds.
fn checksum(parts: &[&[u8]]) -> u32 {
    let mut check = u32::MAX;
    for part in parts {
        for byte in *part {
            check = CHECKS[((check ^ u32::from(*byte)) & 0xFF) as usize]
                ^ (check >> 8);
        }
    }
    !check
}

fn number(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Read a file as a PNG: its beginning, its header, and every part of
/// it up to its last, each against its own checksum. A file that passes
/// is whole as it was written; whether its picture unpacks is the
/// browser's to find.
pub fn read(file: &[u8]) -> Result<Picture, NotOne> {
    let Some(mut rest) = file.strip_prefix(&SIGNATURE) else {
        return Err(NotOne::NoPng);
    };
    let mut picture = None;
    let mut has_picture_data = false;
    loop {
        // A part: how long it is, its name, what it holds, its checksum.
        if rest.len() < 12 {
            return Err(NotOne::Damaged);
        }
        let length = number(rest) as usize;
        let name = &rest[4..8];
        let Some(end) = length.checked_add(12).filter(|end| *end <= rest.len())
        else {
            return Err(NotOne::Damaged);
        };
        let held = &rest[8..8 + length];
        if number(&rest[8 + length..]) != checksum(&[name, held]) {
            return Err(NotOne::Damaged);
        }
        rest = &rest[end..];
        match (name, picture) {
            // The header comes first, and once.
            (b"IHDR", None) => picture = Some(header(held)?),
            (_, None) | (b"IHDR", Some(_)) => return Err(NotOne::Damaged),
            (b"IDAT", Some(_)) => has_picture_data = true,
            (b"IEND", Some(picture)) => {
                // The last part holds nothing, and nothing follows it.
                let whole = length == 0 && rest.is_empty() && has_picture_data;
                return whole.then_some(picture).ok_or(NotOne::Damaged);
            }
            _ => {}
        }
    }
}

/// The header of a PNG file: how large the picture is, and how it is
/// held, which is one of the ways a PNG has.
fn header(held: &[u8]) -> Result<Picture, NotOne> {
    if held.len() != 13 {
        return Err(NotOne::Damaged);
    }
    let picture = Picture {
        width: number(held),
        height: number(&held[4..]),
    };
    let (depth, colour) = (held[8], held[9]);
    let held_one_way = match colour {
        0 => matches!(depth, 1 | 2 | 4 | 8 | 16),
        3 => matches!(depth, 1 | 2 | 4 | 8),
        2 | 4 | 6 => matches!(depth, 8 | 16),
        _ => false,
    };
    let as_a_png_is = held_one_way
        && held[10] == 0
        && held[11] == 0
        && held[12] <= 1
        && (1..=i32::MAX as u32).contains(&picture.width)
        && (1..=i32::MAX as u32).contains(&picture.height);
    as_a_png_is.then_some(picture).ok_or(NotOne::Damaged)
}

/// A PNG file made here, for a test that needs one: a picture of one
/// colour, each pixel red, green, blue and how opaque.
#[cfg(any(test, feature = "samples"))]
pub fn sample(width: u32, height: u32, colour: [u8; 4]) -> Vec<u8> {
    drawn(width, height, |_, _| colour)
}

/// A PNG file made here, for a test that needs one: a picture whose
/// every pixel is what `pixel` says it is at so far across and so far
/// down, as red, green, blue and how opaque. It is packed as stored
/// bytes with nothing squeezed.
#[cfg(any(test, feature = "samples"))]
pub fn drawn(
    width: u32,
    height: u32,
    pixel: impl Fn(u32, u32) -> [u8; 4],
) -> Vec<u8> {
    fn part(file: &mut Vec<u8>, name: &[u8; 4], held: &[u8]) {
        file.extend_from_slice(&(held.len() as u32).to_be_bytes());
        file.extend_from_slice(name);
        file.extend_from_slice(held);
        file.extend_from_slice(&checksum(&[name, held]).to_be_bytes());
    }
    // The picture: each row opens with the filter it was written under,
    // which is none.
    let mut rows =
        Vec::with_capacity((width as usize * 4 + 1) * height as usize);
    for down in 0..height {
        rows.extend_from_slice(&[0]);
        for across in 0..width {
            rows.extend_from_slice(&pixel(across, down));
        }
    }
    // Packed as zlib packs, in stored blocks, with zlib's own checksum
    // of what was packed after them.
    let mut packed = vec![0x78, 0x01];
    let mut blocks = rows.chunks(0xFFFF).peekable();
    while let Some(block) = blocks.next() {
        packed.push(u8::from(blocks.peek().is_none()));
        packed.extend_from_slice(&(block.len() as u16).to_le_bytes());
        packed.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        packed.extend_from_slice(block);
    }
    let (mut low, mut high) = (1u32, 0u32);
    for byte in &rows {
        low = (low + u32::from(*byte)) % 65521;
        high = (high + low) % 65521;
    }
    packed.extend_from_slice(&((high << 16) | low).to_be_bytes());

    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    // Eight bits a channel, in colour with how opaque; packed, filtered
    // and ordered the one way a PNG has.
    header.extend_from_slice(&[8, 6, 0, 0, 0]);

    let mut file = SIGNATURE.to_vec();
    part(&mut file, b"IHDR", &header);
    part(&mut file, b"IDAT", &packed);
    part(&mut file, b"IEND", &[]);
    file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checksum_is_the_one_a_png_closes_a_part_with() {
        // The last part of every PNG file: its name over nothing held.
        assert_eq!(checksum(&[b"IEND"]), 0xAE42_6082);
        assert_eq!(checksum(&[b"123456789"]), 0xCBF4_3926);
        assert_eq!(checksum(&[b"1234", b"56789"]), 0xCBF4_3926);
    }

    #[test]
    fn a_png_is_read_for_how_large_its_picture_is() {
        let file = sample(300, 84, [10, 20, 30, 255]);
        assert_eq!(
            read(&file),
            Ok(Picture {
                width: 300,
                height: 84
            })
        );
        // One whose picture takes more than one stored block.
        let file = sample(1024, 512, [0, 0, 0, 0]);
        assert_eq!(
            read(&file),
            Ok(Picture {
                width: 1024,
                height: 512
            })
        );
    }

    #[test]
    fn a_file_that_does_not_begin_as_a_png_is_none() {
        assert_eq!(read(b""), Err(NotOne::NoPng));
        assert_eq!(read(b"GIF89a, and so on"), Err(NotOne::NoPng));
        assert_eq!(read(b"<svg xmlns='...'/>"), Err(NotOne::NoPng));
        assert_eq!(read(&SIGNATURE[..7]), Err(NotOne::NoPng));
        // A JPEG.
        assert_eq!(read(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 16]), Err(NotOne::NoPng));
    }

    #[test]
    fn a_png_cut_short_or_changed_is_damaged() {
        let file = sample(4, 4, [1, 2, 3, 4]);
        assert!(read(&file).is_ok());
        // Cut anywhere after its beginning.
        for length in SIGNATURE.len()..file.len() {
            assert_eq!(read(&file[..length]), Err(NotOne::Damaged), "{length}");
        }
        // Any byte after its beginning changed: a part's checksum, its
        // length or its name no longer holds.
        for at in SIGNATURE.len()..file.len() {
            let mut changed = file.clone();
            changed[at] ^= 0x01;
            assert_eq!(read(&changed), Err(NotOne::Damaged), "{at}");
        }
        // And anything after its last part.
        let mut longer = file.clone();
        longer.push(0);
        assert_eq!(read(&longer), Err(NotOne::Damaged));
    }

    #[test]
    fn a_png_with_no_picture_or_a_header_no_png_has_is_damaged() {
        fn part(name: &[u8; 4], held: &[u8]) -> Vec<u8> {
            let mut part = (held.len() as u32).to_be_bytes().to_vec();
            part.extend_from_slice(name);
            part.extend_from_slice(held);
            part.extend_from_slice(&checksum(&[name, held]).to_be_bytes());
            part
        }
        let file = |header: &[u8], parts: &[Vec<u8>]| {
            let mut file = SIGNATURE.to_vec();
            file.extend(part(b"IHDR", header));
            for part in parts {
                file.extend_from_slice(part);
            }
            file
        };
        let good = [0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0, 0, 0];
        let data = part(b"IDAT", &[0x78, 0x01]);
        let end = part(b"IEND", &[]);
        assert!(read(&file(&good, &[data.clone(), end.clone()])).is_ok());
        // No picture data at all.
        assert_eq!(
            read(&file(&good, std::slice::from_ref(&end))),
            Err(NotOne::Damaged)
        );
        // A picture of no width, and one held in a way no PNG is.
        for header in [
            [0, 0, 0, 0, 0, 0, 0, 2, 8, 6, 0, 0, 0],
            [0, 0, 0, 2, 0, 0, 0, 0, 8, 6, 0, 0, 0],
            [0, 0, 0, 2, 0, 0, 0, 2, 3, 6, 0, 0, 0],
            [0, 0, 0, 2, 0, 0, 0, 2, 8, 5, 0, 0, 0],
            [0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 1, 0, 0],
            [0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0, 0, 2],
        ] {
            assert_eq!(
                read(&file(&header, &[data.clone(), end.clone()])),
                Err(NotOne::Damaged),
                "{header:?}"
            );
        }
        // A second header, and a part before the first.
        assert_eq!(
            read(&file(&good, &[part(b"IHDR", &good), data.clone(), end])),
            Err(NotOne::Damaged)
        );
        let mut headless = SIGNATURE.to_vec();
        headless.extend(data);
        assert_eq!(read(&headless), Err(NotOne::Damaged));
    }
}
