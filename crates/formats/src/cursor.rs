//! Windows cursors (`RT_GROUP_CURSOR`, `RT_CURSOR`), as `WIDGET.EXE`
//! keeps the editor's pointers (groups 5000 to 5013, `0x439b88`).

/// The `RT_CURSOR` id of a group's first image: a `NEWHEADER` (6 bytes)
/// then 14-byte entries ending in the image's id.
#[must_use]
pub fn group_first(d: &[u8]) -> Option<u16> {
    let n = u16::from_le_bytes([*d.get(4)?, *d.get(5)?]);
    if n == 0 {
        return None;
    }
    Some(u16::from_le_bytes([*d.get(6 + 12)?, *d.get(6 + 13)?]))
}

/// A cursor image in true colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub width: u16,
    pub height: u16,
    /// The hot spot, from the top left.
    pub hot: (u16, u16),
    /// RGBA, top row first; the AND mask's pixels are transparent.
    pub rgba: Vec<u8>,
}

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(at)?, *d.get(at + 1)?]))
}

fn u32_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *d.get(at)?,
        *d.get(at + 1)?,
        *d.get(at + 2)?,
        *d.get(at + 3)?,
    ]))
}

impl Cursor {
    /// Reads an `RT_CURSOR`: the hot spot, then a DIB twice as high as the
    /// cursor (the XOR image, then the AND mask), 1, 4 or 8 bits a pixel.
    #[must_use]
    pub fn parse(d: &[u8]) -> Option<Cursor> {
        let hot = (u16_at(d, 0)?, u16_at(d, 2)?);
        let b = d.get(4..)?;
        let hdr = u32_at(b, 0)? as usize;
        let w = u32_at(b, 4)? as usize;
        let h = (u32_at(b, 8)? / 2) as usize;
        let bpp = usize::from(u16_at(b, 14)?);
        if !matches!(bpp, 1 | 4 | 8) || w == 0 || h == 0 || w > 256 || h > 256 {
            return None;
        }
        let used = u32_at(b, 32)? as usize;
        let colors = if used == 0 { 1 << bpp } else { used };
        let pal = hdr;
        let xor = pal + colors * 4;
        let stride = (w * bpp).div_ceil(32) * 4;
        let and = xor + stride * h;
        let and_stride = w.div_ceil(32) * 4;
        let mut rgba = vec![0; w * h * 4];
        for y in 0..h {
            let row = h - 1 - y;
            for x in 0..w {
                let bit = x * bpp;
                let byte = *b.get(xor + row * stride + bit / 8)?;
                let i = usize::from(byte >> (8 - bpp - bit % 8)) & ((1 << bpp) - 1);
                let masked = b.get(and + row * and_stride + x / 8)? & (0x80 >> (x % 8)) != 0;
                let at = (y * w + x) * 4;
                if !masked {
                    let c = pal + i * 4;
                    rgba[at] = *b.get(c + 2)?;
                    rgba[at + 1] = *b.get(c + 1)?;
                    rgba[at + 2] = *b.get(c)?;
                    rgba[at + 3] = 0xff;
                }
            }
        }
        Some(Cursor {
            width: u16::try_from(w).ok()?,
            height: u16::try_from(h).ok()?,
            hot,
            rgba,
        })
    }
}

/// An `RT_CURSOR` that [`Cursor::parse`] reads: `w` × `h` pixels, top row
/// first, each a palette index or transparent, 8 bits a pixel with
/// `palette` as its colours.
#[must_use]
pub fn encode(w: u16, h: u16, hot: (u16, u16), pixels: &[Option<u8>], palette: &[[u8; 3]; 256]) -> Vec<u8> {
    let (wu, hu) = (usize::from(w), usize::from(h));
    let mut d = Vec::new();
    d.extend_from_slice(&hot.0.to_le_bytes());
    d.extend_from_slice(&hot.1.to_le_bytes());
    let stride = wu.div_ceil(4) * 4;
    let and_stride = wu.div_ceil(32) * 4;
    let mut hdr = [0u8; 40];
    hdr[0] = 40;
    hdr[4..8].copy_from_slice(&u32::from(w).to_le_bytes());
    hdr[8..12].copy_from_slice(&(u32::from(h) * 2).to_le_bytes());
    hdr[12] = 1;
    hdr[14] = 8;
    let size = u32::try_from((stride + and_stride) * hu).unwrap_or(0);
    hdr[20..24].copy_from_slice(&size.to_le_bytes());
    d.extend_from_slice(&hdr);
    for c in palette {
        d.extend_from_slice(&[c[2], c[1], c[0], 0]);
    }
    for y in (0..hu).rev() {
        let mut row = vec![0u8; stride];
        for (x, px) in row.iter_mut().take(wu).enumerate() {
            *px = pixels.get(y * wu + x).copied().flatten().unwrap_or(0);
        }
        d.extend_from_slice(&row);
    }
    for y in (0..hu).rev() {
        let mut row = vec![0u8; and_stride];
        for x in 0..wu {
            if pixels.get(y * wu + x).copied().flatten().is_none() {
                row[x / 8] |= 0x80 >> (x % 8);
            }
        }
        d.extend_from_slice(&row);
    }
    d
}

/// A one-image `RT_GROUP_CURSOR` whose image is `RT_CURSOR` `id`, of
/// `bytes` bytes, as [`group_first`] reads it.
#[must_use]
pub fn group(w: u16, h: u16, id: u16, bytes: u32) -> Vec<u8> {
    let mut d = vec![0, 0, 2, 0, 1, 0];
    d.extend_from_slice(&w.to_le_bytes());
    d.extend_from_slice(&(h * 2).to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&8u16.to_le_bytes());
    d.extend_from_slice(&bytes.to_le_bytes());
    d.extend_from_slice(&id.to_le_bytes());
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_one_bit_cursor() {
        // 2×2 (stride 4), 1 bpp: XOR rows then AND rows, bottom up.
        let mut d = vec![1, 0, 0, 0];
        let mut h = vec![0u8; 40];
        h[0] = 40;
        h[4] = 2;
        h[8] = 4;
        h[12] = 1;
        h[14] = 1;
        d.extend(h);
        d.extend([0, 0, 0, 0, 255, 255, 255, 0]);
        // XOR: bottom row white|black, top row black|black.
        d.extend([0x80, 0, 0, 0, 0, 0, 0, 0]);
        // AND: bottom row opaque, top row: right pixel transparent.
        d.extend([0, 0, 0, 0, 0x40, 0, 0, 0]);
        let c = Cursor::parse(&d).unwrap();
        assert_eq!(c.hot, (1, 0));
        assert_eq!((c.width, c.height), (2, 2));
        assert_eq!(&c.rgba[..8], [0, 0, 0, 255, 0, 0, 0, 0]);
        assert_eq!(&c.rgba[8..], [255, 255, 255, 255, 0, 0, 0, 255]);
    }

    #[test]
    fn encodes_what_it_reads() {
        let mut pal = [[0u8; 3]; 256];
        pal[5] = [10, 20, 30];
        let px = [Some(5), None, None, Some(5), Some(0), Some(5)];
        let d = encode(3, 2, (2, 1), &px, &pal);
        let c = Cursor::parse(&d).unwrap();
        assert_eq!((c.width, c.height, c.hot), (3, 2, (2, 1)));
        assert_eq!(&c.rgba[..8], [10, 20, 30, 255, 0, 0, 0, 0]);
        assert_eq!(&c.rgba[12..16], [10, 20, 30, 255]);
        assert_eq!(group_first(&group(3, 2, 31, 9)), Some(31));
    }
}
