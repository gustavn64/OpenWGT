//! PNG pictures, as the port's own graphics (`engine::opengfx`) keep their
//! frames: non-interlaced, 8 bits a channel (or 1, 2, 4 or 8 bits for a
//! palette or grey), any colour type. Decoded to RGBA.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NotPng,
    /// A chunk or the pixels end early.
    Truncated,
    /// Interlaced, 16 bits a channel, or an unknown colour type.
    Unsupported(&'static str),
    /// The pixels don't inflate, or a row has an unknown filter.
    Corrupt,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotPng => f.write_str("not a PNG picture"),
            Error::Truncated => f.write_str("the PNG picture is cut short"),
            Error::Unsupported(what) => write!(f, "unsupported PNG picture: {what}"),
            Error::Corrupt => f.write_str("the PNG picture's pixels are corrupt"),
        }
    }
}

impl std::error::Error for Error {}

/// A decoded picture: RGBA, top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>,
}

const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
/// Bigger than any frame the game draws.
const MAX_SIDE: u32 = 4096;

fn be32(d: &[u8], at: usize) -> Result<u32, Error> {
    d.get(at..at + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(Error::Truncated)
}

/// Decodes the PNG picture `d`.
///
/// # Errors
/// If it isn't a PNG, is cut short or corrupt, or is a kind this doesn't
/// read.
pub fn decode(d: &[u8]) -> Result<Rgba, Error> {
    if d.get(..8) != Some(SIGNATURE) {
        return Err(Error::NotPng);
    }
    let mut at = 8;
    let (mut width, mut height, mut depth, mut kind) = (0u32, 0u32, 0u8, 0u8);
    let mut palette: Vec<[u8; 4]> = Vec::new();
    let mut key: Option<[u16; 3]> = None;
    let mut idat = Vec::new();
    loop {
        let len = be32(d, at)? as usize;
        let name = d.get(at + 4..at + 8).ok_or(Error::Truncated)?;
        let end = (at + 8).checked_add(len).ok_or(Error::Truncated)?;
        let body = d.get(at + 8..end).ok_or(Error::Truncated)?;
        match name {
            b"IHDR" => {
                width = be32(body, 0)?;
                height = be32(body, 4)?;
                let b = body.get(8..13).ok_or(Error::Truncated)?;
                depth = b[0];
                kind = b[1];
                if b[4] != 0 {
                    return Err(Error::Unsupported("interlaced"));
                }
            }
            b"PLTE" => {
                palette = body
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|c| [c[0], c[1], c[2], 0xff])
                    .collect();
            }
            b"tRNS" => {
                if kind == 3 {
                    for (p, &a) in palette.iter_mut().zip(body) {
                        p[3] = a;
                    }
                } else {
                    let s = |i: usize| body.get(i..i + 2).map_or(0, |b| u16::from_be_bytes([b[0], b[1]]));
                    key = Some(if kind == 0 { [s(0); 3] } else { [s(0), s(2), s(4)] });
                }
            }
            b"IDAT" => idat.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(Error::Unsupported("size"));
    }
    let channels = match (kind, depth) {
        (0 | 3, 1 | 2 | 4 | 8) => 1,
        (2, 8) => 3,
        (4, 8) => 2,
        (6, 8) => 4,
        (0 | 2 | 3 | 4 | 6, _) => return Err(Error::Unsupported("bit depth")),
        _ => return Err(Error::Unsupported("colour type")),
    };
    let (w, h) = (width as usize, height as usize);
    let bits = channels * usize::from(depth);
    let stride = (w * bits).div_ceil(8);
    let bpp = bits.div_ceil(8);
    let raw =
        miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&idat, (stride + 1) * h).map_err(|_| Error::Corrupt)?;
    if raw.len() < (stride + 1) * h {
        return Err(Error::Truncated);
    }
    let rows = unfilter(&raw, stride, bpp, h)?;
    let pixels = to_rgba(&rows, (w, h, stride), (kind, depth), &palette, key);
    Ok(Rgba { width, height, pixels })
}

/// The unfiltered `rows` (`w` × `h`, `stride` bytes a row) of colour type
/// `kind` at `depth` bits as RGBA.
fn to_rgba(
    rows: &[u8],
    (w, h, stride): (usize, usize, usize),
    (kind, depth): (u8, u8),
    palette: &[[u8; 4]],
    key: Option<[u16; 3]>,
) -> Vec<[u8; 4]> {
    let sample = |row: &[u8], x: usize| -> u8 {
        let per = 8 / usize::from(depth);
        let byte = row[x / per];
        let shift = 8 - usize::from(depth) * (x % per + 1);
        (byte >> shift) & (0xff >> (8 - depth))
    };
    let mut pixels = Vec::with_capacity(w * h);
    for y in 0..h {
        let row = &rows[y * stride..(y + 1) * stride];
        for x in 0..w {
            let px = match kind {
                3 => palette.get(usize::from(sample(row, x))).copied().unwrap_or([0; 4]),
                0 => {
                    let v = sample(row, x);
                    let full = u8::try_from(u32::from(v) * 255 / ((1u32 << depth) - 1)).unwrap_or(0);
                    let clear = key.is_some_and(|k| k[0] == u16::from(v));
                    [full, full, full, if clear { 0 } else { 0xff }]
                }
                4 => [row[x * 2], row[x * 2], row[x * 2], row[x * 2 + 1]],
                2 => {
                    let p = &row[x * 3..x * 3 + 3];
                    let clear = key.is_some_and(|k| k == [p[0], p[1], p[2]].map(u16::from));
                    [p[0], p[1], p[2], if clear { 0 } else { 0xff }]
                }
                _ => {
                    let p = &row[x * 4..x * 4 + 4];
                    [p[0], p[1], p[2], p[3]]
                }
            };
            pixels.push(px);
        }
    }
    pixels
}

/// Undoes the rows' filters: `h` rows of `stride` bytes, each after its
/// filter byte in `raw`.
fn unfilter(raw: &[u8], stride: usize, bpp: usize, h: usize) -> Result<Vec<u8>, Error> {
    let mut rows = vec![0u8; stride * h];
    for y in 0..h {
        let filter = raw[y * (stride + 1)];
        let src = &raw[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        let (done, rest) = rows.split_at_mut(y * stride);
        let prev = if y == 0 { None } else { Some(&done[(y - 1) * stride..]) };
        let row = &mut rest[..stride];
        for x in 0..stride {
            let a = if x >= bpp { row[x - bpp] } else { 0 };
            let b = prev.map_or(0, |p| p[x]);
            let c = if x >= bpp { prev.map_or(0, |p| p[x - bpp]) } else { 0 };
            let add = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => u8::midpoint(a, b),
                4 => paeth(a, b, c),
                _ => return Err(Error::Corrupt),
            };
            row[x] = src[x].wrapping_add(add);
        }
    }
    Ok(rows)
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (ia, ib, ic) = (i16::from(a), i16::from(b), i16::from(c));
    let p = ia + ib - ic;
    let (pa, pb, pc) = ((p - ia).abs(), (p - ib).abs(), (p - ic).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Encodes `img` as an RGBA PNG (for tools and tests).
#[must_use]
pub fn encode(img: &Rgba) -> Vec<u8> {
    let w = img.width as usize;
    let mut raw = Vec::with_capacity((w * 4 + 1) * img.height as usize);
    for row in img.pixels.chunks(w.max(1)) {
        raw.push(0);
        raw.extend(row.iter().flatten());
    }
    let mut out = SIGNATURE.to_vec();
    let mut chunk = |name: &[u8; 4], body: &[u8]| {
        out.extend_from_slice(&u32::try_from(body.len()).unwrap_or(0).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(name);
        out.extend_from_slice(body);
        let crc = crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&img.width.to_be_bytes());
    ihdr.extend_from_slice(&img.height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 9));
    chunk(b"IEND", &[]);
    out
}

fn crc32(d: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in d {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_rgba() {
        let img = Rgba {
            width: 3,
            height: 2,
            pixels: vec![
                [1, 2, 3, 255],
                [4, 5, 6, 0],
                [7, 8, 9, 128],
                [10, 11, 12, 255],
                [0; 4],
                [255; 4],
            ],
        };
        assert_eq!(decode(&encode(&img)).unwrap(), img);
    }

    #[test]
    fn rejects_other_files() {
        assert_eq!(decode(b"GIF89a"), Err(Error::NotPng));
    }

    #[test]
    fn filters_undo() {
        // A 2×2 grey picture, row 0 with Sub, row 1 with Paeth.
        let raw = [1, 10, 5, 4, 3, 7];
        let mut d = SIGNATURE.to_vec();
        let mut chunk = |name: &[u8; 4], body: &[u8]| {
            d.extend_from_slice(&u32::try_from(body.len()).unwrap().to_be_bytes());
            let start = d.len();
            d.extend_from_slice(name);
            d.extend_from_slice(body);
            let crc = crc32(&d[start..]);
            d.extend_from_slice(&crc.to_be_bytes());
        };
        chunk(b"IHDR", &[0, 0, 0, 2, 0, 0, 0, 2, 8, 0, 0, 0, 0]);
        chunk(b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6));
        chunk(b"IEND", &[]);
        let img = decode(&d).unwrap();
        let grey: Vec<u8> = img.pixels.iter().map(|p| p[0]).collect();
        // Row 0: 10, 10+5. Row 1: paeth(0,10,0)=10 → 13; paeth(13,15,10)=15 → 22.
        assert_eq!(grey, [10, 15, 13, 22]);
    }
}
