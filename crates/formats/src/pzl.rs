//! The puzzle file (`WIDGET.PZL`): a Mac resource fork stored as a plain
//! file, read the way the game's own little resource reader
//! (`0x413e78`..`0x414781`) reads it. See `docs/puzzles.md`.
//!
//! - `Puzl` 1..25: each a saved widget (`FORM Gadf`); the resource ID is the
//!   puzzle's number, the resource name its title.
//! - `Done` 1: a (stale, ignored) count word, then one Mac `Boolean` word per
//!   puzzle (`0x0100` solved), as many as the resource's size allows.
//!
//! The Swedish edition (Uppfinnarverkstan) ships the same resources in a
//! flat file of its own instead ([`ResFork::parse_flat`]): one after
//! another, each its type, its ID (`u32`, little-endian), its name in 64
//! bytes (NUL-terminated, Windows-1252) and its length (`u32`), then its
//! data.
//!
//! The game also keeps the solved flags in `WIDGET.INI` (`[Widget Workshop]
//! Done=`) as a hex string ([`parse_done_ini`], [`format_done_ini`]), and ORs
//! the two together when it reads them.

use std::fmt;

/// The 4-byte type codes.
const PUZL: [u8; 4] = *b"Puzl";
const DONE: [u8; 4] = *b"Done";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Something the map points at lies past the file's end.
    OutOfBounds(usize),
    /// There's no `Done` 1 resource.
    NoDone,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::OutOfBounds(at) => write!(f, "resource fork: {at:#x} lies past the end"),
            Error::NoDone => f.write_str("resource fork: no 'Done' resource"),
        }
    }
}

impl std::error::Error for Error {}

fn be16(b: &[u8], at: usize) -> Result<u16, Error> {
    b.get(at..at + 2)
        .map(|s| u16::from_be_bytes([s[0], s[1]]))
        .ok_or(Error::OutOfBounds(at))
}

fn be32(b: &[u8], at: usize) -> Result<u32, Error> {
    b.get(at..at + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(Error::OutOfBounds(at))
}

/// One resource as the reference list describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub id: i16,
    /// The name's bytes (Mac Roman), empty if it has none.
    pub name: Vec<u8>,
    /// Where the data (after its length word) starts in the file.
    pub data_at: usize,
    pub data_len: usize,
}

/// A resource fork's index, read as `0x413e78` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResFork {
    /// Per type: its code and its resources, in reference-list order.
    pub types: Vec<([u8; 4], Vec<Resource>)>,
}

impl ResFork {
    /// Reads the fork's map. Like the game: if the file doesn't start with
    /// a data offset of `0x100` it's taken to be `MacBinary` (fork at `0x80`);
    /// the type list is always at map + `0x1c`, the name list right after the
    /// last type's reference list, and data at fork + `0x100` + offset (the
    /// header's other fields are ignored).
    ///
    /// A flat file ([`ResFork::parse_flat`]) is read as one.
    ///
    /// # Errors
    /// If the map points past the end.
    pub fn parse(file: &[u8]) -> Result<ResFork, Error> {
        if let Some(fork) = Self::parse_flat(file) {
            return Ok(fork);
        }
        let base = if be32(file, 0)? == 0x100 { 0 } else { 0x80 };
        let map = base + be32(file, base + 4)? as usize;
        let type_list = map + 0x1c;
        let n_types = usize::from(be16(file, type_list)?) + 1;
        let mut entries = Vec::with_capacity(n_types);
        for i in 0..n_types {
            let at = type_list + 2 + 8 * i;
            let code = file.get(at..at + 4).ok_or(Error::OutOfBounds(at))?;
            let count = usize::from(be16(file, at + 4)?) + 1;
            let refs = usize::from(be16(file, at + 6)?);
            entries.push(([code[0], code[1], code[2], code[3]], count, refs));
        }
        let name_list = entries
            .last()
            .map_or(type_list, |&(_, count, refs)| type_list + refs + count * 12);
        let mut types = Vec::with_capacity(n_types);
        for (code, count, refs) in entries {
            let mut list = Vec::with_capacity(count);
            for j in 0..count {
                let at = type_list + refs + 12 * j;
                let id = be16(file, at)?.cast_signed();
                let name_off = be16(file, at + 2)?;
                let off = (be32(file, at + 4)? & 0x00ff_ffff) as usize;
                let name = if name_off == 0xffff {
                    Vec::new()
                } else {
                    let p = name_list + usize::from(name_off);
                    let len = usize::from(*file.get(p).ok_or(Error::OutOfBounds(p))?);
                    file.get(p + 1..p + 1 + len).ok_or(Error::OutOfBounds(p))?.to_vec()
                };
                let len_at = base + 0x100 + off;
                let data_len = be32(file, len_at)? as usize;
                let data_at = len_at + 4;
                if file.len() < data_at + data_len {
                    return Err(Error::OutOfBounds(len_at));
                }
                list.push(Resource {
                    id,
                    name,
                    data_at,
                    data_len,
                });
            }
            types.push((code, list));
        }
        Ok(ResFork { types })
    }

    /// The Swedish edition's flat file: records of a type code, an ID
    /// (`u32` LE), a 64-byte NUL-terminated name and a length (`u32` LE),
    /// each followed by its data, to the file's end. `None` if `file` isn't
    /// one (a record that doesn't fit, a type that isn't four letters).
    #[must_use]
    pub fn parse_flat(file: &[u8]) -> Option<ResFork> {
        const HEAD: usize = 4 + 4 + 64 + 4;
        let le32 = |at: usize| -> Option<u32> { Some(u32::from_le_bytes(file.get(at..at + 4)?.try_into().ok()?)) };
        let mut types: Vec<([u8; 4], Vec<Resource>)> = Vec::new();
        let mut at = 0;
        while at < file.len() {
            let code: [u8; 4] = file.get(at..at + 4)?.try_into().ok()?;
            if !code.iter().all(u8::is_ascii_alphanumeric) {
                return None;
            }
            let id = i16::try_from(le32(at + 4)?).ok()?;
            let name_field = file.get(at + 8..at + 72)?;
            let name = name_field.split(|&b| b == 0).next()?.to_vec();
            let data_len = le32(at + 72)? as usize;
            let data_at = at + HEAD;
            if file.len() < data_at + data_len {
                return None;
            }
            let r = Resource {
                id,
                name,
                data_at,
                data_len,
            };
            match types.iter_mut().find(|(c, _)| *c == code) {
                Some((_, list)) => list.push(r),
                None => types.push((code, vec![r])),
            }
            at = data_at + data_len;
        }
        (!types.is_empty()).then_some(ResFork { types })
    }

    /// The resource `id` of type `code` (`0x414102` looks up by ID).
    #[must_use]
    pub fn find(&self, code: [u8; 4], id: i16) -> Option<&Resource> {
        self.types
            .iter()
            .find(|(c, _)| *c == code)?
            .1
            .iter()
            .find(|r| r.id == id)
    }
}

/// One puzzle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    /// Its number: the `Puzl` resource ID.
    pub id: i16,
    /// The title as stored (Mac Roman).
    pub name: Vec<u8>,
    /// The saved widget (`FORM Gadf`).
    pub widget: Vec<u8>,
}

impl Puzzle {
    /// The title as the Load Puzzle list shows it: `0x42ced8` turns the Mac
    /// apostrophe (`0xD5`) into `'`; other bytes are shown as Windows text,
    /// Windows-1252 (only ASCII occurs in the English file; the Swedish
    /// edition's names are Windows text, the port's French ones may have
    /// œ or ’).
    #[must_use]
    pub fn title(&self) -> String {
        self.name
            .iter()
            .map(|&b| if b == 0xd5 { '\'' } else { crate::cp1252::char_of(b) })
            .collect()
    }
}

/// The puzzle file's contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzles {
    /// Sorted by number.
    pub puzzles: Vec<Puzzle>,
    /// The `Done` resource's flags: `done[n - 1]` is puzzle `n`.
    pub done: Vec<bool>,
}

impl Puzzles {
    /// Reads `WIDGET.PZL`.
    ///
    /// # Errors
    /// If the fork is damaged or has no `Done` resource.
    pub fn parse(file: &[u8]) -> Result<Puzzles, Error> {
        let fork = ResFork::parse(file)?;
        let mut puzzles: Vec<Puzzle> = fork
            .types
            .iter()
            .filter(|(c, _)| *c == PUZL)
            .flat_map(|(_, list)| list)
            .map(|r| Puzzle {
                id: r.id,
                name: r.name.clone(),
                widget: file[r.data_at..r.data_at + r.data_len].to_vec(),
            })
            .collect();
        puzzles.sort_by_key(|p| p.id);
        let done = fork.find(DONE, 1).ok_or(Error::NoDone)?;
        let data = &file[done.data_at..done.data_at + done.data_len];
        Ok(Puzzles {
            puzzles,
            done: read_done(data),
        })
    }
}

/// A puzzle file with `puzzles` (number, title in Mac Roman, the saved
/// widget) and none of them solved, laid out as the game's reader
/// (`0x413e78`) expects: data from 0x100, the type list at map + 0x1c,
/// the name list right after the last type's reference list.
#[must_use]
pub fn write(puzzles: &[(i16, &[u8], &[u8])]) -> Vec<u8> {
    let mut data = Vec::new();
    let mut puzl_refs = Vec::new();
    let mut names = Vec::new();
    for (id, name, widget) in puzzles {
        let name_at = u16::try_from(names.len()).unwrap_or(0xffff);
        names.push(u8::try_from(name.len().min(255)).unwrap_or(255));
        names.extend_from_slice(&name[..name.len().min(255)]);
        puzl_refs.push((*id, name_at, u32::try_from(data.len()).unwrap_or(0)));
        data.extend_from_slice(&u32::try_from(widget.len()).unwrap_or(0).to_be_bytes());
        data.extend_from_slice(widget);
    }
    // `Done` 1: the count word, then a flag per puzzle, all unsolved.
    let done_at = u32::try_from(data.len()).unwrap_or(0);
    let done_len = 2 + 2 * puzzles.len();
    data.extend_from_slice(&u32::try_from(done_len).unwrap_or(0).to_be_bytes());
    data.extend_from_slice(&1u16.to_be_bytes());
    data.extend(std::iter::repeat_n(0u8, 2 * puzzles.len()));
    // The map: header copy, reserved, attributes, the two offsets, the
    // type list (`Puzl`, `Done`), their reference lists, the names.
    let types = 2usize;
    let type_list = 2 + types * 8;
    let puzl_ref_at = type_list;
    let done_ref_at = puzl_ref_at + puzl_refs.len() * 12;
    let names_at = 0x1c + done_ref_at + 12;
    let mut map = vec![0u8; 0x1c];
    map[0x18..0x1a].copy_from_slice(&0x1cu16.to_be_bytes());
    map[0x1a..0x1c].copy_from_slice(&u16::try_from(names_at).unwrap_or(0).to_be_bytes());
    map.extend_from_slice(&u16::try_from(types - 1).unwrap_or(0).to_be_bytes());
    for (code, n, at) in [(PUZL, puzl_refs.len(), puzl_ref_at), (DONE, 1, done_ref_at)] {
        map.extend_from_slice(&code);
        map.extend_from_slice(&u16::try_from(n.saturating_sub(1)).unwrap_or(0).to_be_bytes());
        map.extend_from_slice(&u16::try_from(at).unwrap_or(0).to_be_bytes());
    }
    let entry = |map: &mut Vec<u8>, id: i16, name: u16, at: u32| {
        map.extend_from_slice(&id.to_be_bytes());
        map.extend_from_slice(&name.to_be_bytes());
        map.push(0);
        map.extend_from_slice(&at.to_be_bytes()[1..]);
        map.extend_from_slice(&[0; 4]);
    };
    for (id, name, at) in &puzl_refs {
        entry(&mut map, *id, *name, *at);
    }
    entry(&mut map, 1, 0xffff, done_at);
    map.extend_from_slice(&names);
    let map_at = 0x100 + data.len();
    let mut header = Vec::new();
    for v in [0x100, map_at, data.len(), map.len()] {
        header.extend_from_slice(&u32::try_from(v).unwrap_or(0).to_be_bytes());
    }
    map[..16].copy_from_slice(&header);
    let mut out = header;
    out.resize(0x100, 0);
    out.extend_from_slice(&data);
    out.extend_from_slice(&map);
    out
}

/// The flags of a `Done` resource's data: `size / 2 - 1` words after the
/// count word, any non-zero word solved (`0x42d82d`, `0x42cef8`).
#[must_use]
pub fn read_done(data: &[u8]) -> Vec<bool> {
    let n = (data.len() / 2).saturating_sub(1);
    (0..n).map(|i| data[2 + 2 * i] != 0 || data[3 + 2 * i] != 0).collect()
}

/// Writes `done` into the `Done` resource of the puzzle file `file`, in
/// place, as `0x414492` does (the count word kept, each flag `0x0100` or
/// `0`). Flags past the resource's room are dropped; puzzles past `done`'s
/// end are left as they are.
///
/// # Errors
/// If the fork is damaged or has no `Done` resource.
pub fn write_done(file: &mut [u8], done: &[bool]) -> Result<(), Error> {
    let fork = ResFork::parse(file)?;
    let r = fork.find(DONE, 1).ok_or(Error::NoDone)?;
    let n = (r.data_len / 2).saturating_sub(1);
    for (i, &d) in done.iter().take(n).enumerate() {
        let at = r.data_at + 2 + 2 * i;
        file[at] = u8::from(d);
        file[at + 1] = 0;
    }
    Ok(())
}

/// A hex digit's value, 0 for anything else (`0x42d7ed`).
fn hex_value(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'A'..=b'F' => c - b'A' + 10,
        b'a'..=b'f' => c - b'a' + 10,
        _ => 0,
    }
}

/// Adds the INI's `Done=` string `s` to `done` (`0x42d82d`): digit `i / 4`,
/// bit `i % 4` is `done[i]`. Only sets flags, never clears them; stops at the
/// string's end.
pub fn parse_done_ini(s: &str, done: &mut [bool]) {
    let s = s.as_bytes();
    for (i, d) in done.iter_mut().enumerate() {
        let Some(&c) = s.get(i >> 2) else { break };
        if c == 0 {
            break;
        }
        if hex_value(c) & (1 << (i & 3)) != 0 {
            *d = true;
        }
    }
}

/// The INI's `Done=` string for `done` (`0x42d915`): `n / 4 + 1` uppercase
/// hex digits (so 25 puzzles give 7, all solved `"FFFFFF1"`). `None` where
/// the game writes nothing (256 digits or more).
#[must_use]
pub fn format_done_ini(done: &[bool]) -> Option<String> {
    let len = (done.len() >> 2) + 1;
    if len >= 0x100 {
        return None;
    }
    let mut digits = vec![0u8; len];
    for (i, _) in done.iter().enumerate().filter(|&(_, &d)| d) {
        digits[i >> 2] |= 1 << (i & 3);
    }
    Some(
        digits
            .iter()
            .map(|&v| char::from(b"0123456789ABCDEF"[usize::from(v)]))
            .collect(),
    )
}

#[cfg(test)]
mod tests {

    /// The Swedish edition's flat file: records one after another.
    #[test]
    fn reads_the_flat_file() {
        let record = |code: &[u8; 4], id: u32, name: &[u8], data: &[u8]| {
            let mut r = code.to_vec();
            r.extend_from_slice(&id.to_le_bytes());
            let mut n = name.to_vec();
            n.push(0);
            n.resize(64, 0xfc);
            r.extend_from_slice(&n);
            r.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
            r.extend_from_slice(data);
            r
        };
        let mut file = record(b"Done", 1, b"Done", &[0, 1, 0, 0, 1, 0]);
        file.extend(record(b"Puzl", 2, b"T\xe4nd lampan!", b"FORM2"));
        file.extend(record(b"Puzl", 1, b"R\xe4kna", b"FORM1"));
        let p = Puzzles::parse(&file).unwrap();
        assert_eq!(p.puzzles.len(), 2);
        assert_eq!(p.puzzles[0].id, 1);
        assert_eq!(p.puzzles[0].widget, b"FORM1");
        assert_eq!(p.puzzles[1].title(), "Tänd lampan!");
        assert_eq!(p.done, vec![false, true]);
        write_done(&mut file, &[true, false]).unwrap();
        assert_eq!(Puzzles::parse(&file).unwrap().done, vec![true, false]);
        // A Mac resource fork isn't taken for one.
        assert!(ResFork::parse_flat(&write(&[(1, b"x", b"FORM")])).is_none());
    }

    use super::*;

    /// A resource fork with the given resources (`type, id, name, data`), in
    /// type order as given, laid out as `ResEdit` does.
    fn build(res: &[([u8; 4], i16, Option<&[u8]>, &[u8])]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut offsets = Vec::new();
        for r in res {
            offsets.push(data.len());
            data.extend_from_slice(&u32::try_from(r.3.len()).unwrap().to_be_bytes());
            data.extend_from_slice(r.3);
        }
        let mut codes: Vec<[u8; 4]> = Vec::new();
        for r in res {
            if !codes.contains(&r.0) {
                codes.push(r.0);
            }
        }
        let mut type_list = Vec::new();
        type_list.extend_from_slice(&u16::try_from(codes.len() - 1).unwrap().to_be_bytes());
        let mut ref_lists = Vec::new();
        let mut names = Vec::new();
        let ref_base = 2 + 8 * codes.len();
        for code in &codes {
            let mine: Vec<usize> = (0..res.len()).filter(|&i| res[i].0 == *code).collect();
            type_list.extend_from_slice(code);
            type_list.extend_from_slice(&u16::try_from(mine.len() - 1).unwrap().to_be_bytes());
            type_list.extend_from_slice(&u16::try_from(ref_base + ref_lists.len()).unwrap().to_be_bytes());
            for i in mine {
                ref_lists.extend_from_slice(&res[i].1.to_be_bytes());
                match res[i].2 {
                    Some(n) => {
                        ref_lists.extend_from_slice(&u16::try_from(names.len()).unwrap().to_be_bytes());
                        names.push(u8::try_from(n.len()).unwrap());
                        names.extend_from_slice(n);
                    }
                    None => ref_lists.extend_from_slice(&[0xff, 0xff]),
                }
                ref_lists.push(0);
                ref_lists.extend_from_slice(&u32::try_from(offsets[i]).unwrap().to_be_bytes()[1..]);
                ref_lists.extend_from_slice(&[0; 4]);
            }
        }
        let map_off = 0x100 + data.len();
        let mut file = vec![0u8; 0x100];
        let mut header = Vec::new();
        header.extend_from_slice(&0x100u32.to_be_bytes());
        header.extend_from_slice(&u32::try_from(map_off).unwrap().to_be_bytes());
        header.extend_from_slice(&u32::try_from(data.len()).unwrap().to_be_bytes());
        let map_len = 0x1c + type_list.len() + ref_lists.len() + names.len();
        header.extend_from_slice(&u32::try_from(map_len).unwrap().to_be_bytes());
        file[..16].copy_from_slice(&header);
        file.extend_from_slice(&data);
        file.extend_from_slice(&header);
        file.extend_from_slice(&[0; 6]);
        file.extend_from_slice(&[0, 0]);
        file.extend_from_slice(&0x1cu16.to_be_bytes());
        file.extend_from_slice(
            &u16::try_from(0x1c + type_list.len() + ref_lists.len())
                .unwrap()
                .to_be_bytes(),
        );
        file.extend_from_slice(&type_list);
        file.extend_from_slice(&ref_lists);
        file.extend_from_slice(&names);
        file
    }

    fn sample() -> Vec<u8> {
        // Done: count word 1 (stale, as shipped), 3 flags, puzzle 2 solved.
        let done: &[u8] = &[0, 1, 0, 0, 1, 0, 0, 0];
        build(&[
            (*b"TMPL", 128, Some(b"Done"), b"tmpl"),
            (PUZL, 2, Some(b"Doc\xd5s Two"), b"FORM-two"),
            (PUZL, 1, Some(b"One"), b"FORM-one"),
            (DONE, 1, None, done),
        ])
    }

    #[test]
    fn reads_a_fork() {
        let p = Puzzles::parse(&sample()).unwrap();
        assert_eq!(p.puzzles.len(), 2);
        assert_eq!(p.puzzles[0].id, 1);
        assert_eq!(p.puzzles[0].title(), "One");
        assert_eq!(p.puzzles[0].widget, b"FORM-one");
        assert_eq!(p.puzzles[1].id, 2);
        assert_eq!(p.puzzles[1].name, b"Doc\xd5s Two");
        assert_eq!(p.puzzles[1].title(), "Doc's Two");
        assert_eq!(p.puzzles[1].widget, b"FORM-two");
        assert_eq!(p.done, vec![false, true, false]);
    }

    #[test]
    fn reads_macbinary() {
        let mut file = vec![0u8; 0x80];
        file.extend_from_slice(&sample());
        let p = Puzzles::parse(&file).unwrap();
        assert_eq!(p.puzzles[1].widget, b"FORM-two");
        assert_eq!(p.done, vec![false, true, false]);
    }

    #[test]
    fn writes_done_in_place() {
        let mut file = sample();
        let before = file.len();
        write_done(&mut file, &[true, false, true, true]).unwrap();
        assert_eq!(file.len(), before);
        let p = Puzzles::parse(&file).unwrap();
        assert_eq!(p.done, vec![true, false, true]);
        // The count word is kept.
        let r = ResFork::parse(&file).unwrap().find(DONE, 1).unwrap().clone();
        assert_eq!(&file[r.data_at..r.data_at + 4], &[0, 1, 1, 0]);
    }

    #[test]
    fn ini_strings() {
        assert_eq!(format_done_ini(&[true; 25]).as_deref(), Some("FFFFFF1"));
        assert_eq!(format_done_ini(&[false; 25]).as_deref(), Some("0000000"));
        let mut only8 = [false; 25];
        only8[7] = true;
        assert_eq!(format_done_ini(&only8).as_deref(), Some("0800000"));
        assert_eq!(format_done_ini(&[false; 1024]), None);

        let mut back = [false; 25];
        parse_done_ini("FFFFFF1", &mut back);
        assert_eq!(back, [true; 25]);
        let mut back = [false; 25];
        parse_done_ini("0800000", &mut back);
        assert_eq!(back, only8);
        // Lowercase digits count; junk reads as 0; a short string stops early;
        // flags already set stay set.
        let mut d = [false; 25];
        d[24] = true;
        parse_done_ini("fz", &mut d);
        assert_eq!(&d[..8], &[true, true, true, true, false, false, false, false]);
        assert!(d[24]);
    }

    /// The shipped `WIDGET.PZL` (extracted to `orig`; skipped without).
    fn shipped() -> Option<Vec<u8>> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../orig/WIN95/WIDGET/WIDGET.PZL");
        std::fs::read(p).ok()
    }

    #[test]
    fn reads_the_shipped_file() {
        let Some(file) = shipped() else {
            eprintln!("no orig: skipped");
            return;
        };
        assert_eq!(file.len(), 242_671);
        let p = Puzzles::parse(&file).unwrap();
        let titles: Vec<String> = p.puzzles.iter().map(Puzzle::title).collect();
        assert_eq!(
            titles,
            [
                "White Color",
                "Turn It On",
                "The Millionaire",
                "Lightbulbs",
                "Guess the Note",
                "More Lightbulbs",
                "Doc's Diabolical Adder",
                "Count To Ten!",
                "Barking Dogs",
                "Wake the Chickens",
                "Clues",
                "Still More Lightbulbs",
                "Moon Baseball",
                "Marquee",
                "Mad Multiplier",
                "Heads or Tails",
                "Gravity Chamber Test",
                "Fast Counter",
                "Elephant Fall",
                "Beat the Clock",
                "Bank Vault",
                "Projector Puzzle",
                "How Many Earths",
                "Doc's Animal Clock",
                "A Rose is a Rose",
            ]
        );
        for (i, z) in p.puzzles.iter().enumerate() {
            assert_eq!(usize::try_from(z.id).unwrap(), i + 1);
            let form = crate::wgt::Form::parse(&z.widget).unwrap();
            assert_eq!(form.version(), Some(4), "puzzle {}", z.id);
            // Exactly one Win / Lose Checker, and a 64-entry `numG`.
            assert_eq!(form.chunks.iter().filter(|c| &c.tag == b"Stop").count(), 1);
            let numg = form.chunks.iter().find(|c| &c.tag == b"numG").unwrap();
            assert_eq!(crate::wgt::be16(&numg.data, 0).ok(), Some(64));
        }
        // The `Done` resource: 25 flags, none set; the stale count word 1.
        assert_eq!(p.done, vec![false; 25]);
        let r = ResFork::parse(&file).unwrap().find(DONE, 1).unwrap().clone();
        assert_eq!(r.data_len, 52);
        assert_eq!(&file[r.data_at..r.data_at + 2], &[0, 1]);
    }

    #[test]
    fn writes_what_it_reads() {
        let a = b"FORM-a".to_vec();
        let b = b"FORM-bb".to_vec();
        let file = write(&[(1, b"First One", &a), (2, b"Second", &b)]);
        let p = Puzzles::parse(&file).unwrap();
        assert_eq!(p.puzzles.len(), 2);
        assert_eq!(
            (p.puzzles[0].title(), &p.puzzles[0].widget),
            ("First One".to_owned(), &a)
        );
        assert_eq!((p.puzzles[1].id, &p.puzzles[1].widget), (2, &b));
        assert_eq!(p.done, [false, false]);
        let mut file = file;
        write_done(&mut file, &[false, true]).unwrap();
        assert_eq!(Puzzles::parse(&file).unwrap().done, [false, true]);
    }
}
