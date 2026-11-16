//! Reading a font file's family name, so the bridges can hand bundled fonts to the engines under
//! the name the file already carries (never another one: the files stay unmodified).

/// The typographic family of a font file (`name` ID 16, else 1; Windows Unicode records), or
/// `None` for a file that is not a TrueType/OpenType font or names no family.
pub fn family(data: &[u8]) -> Option<String> {
    let n = be16(data, 4)? as usize;
    let r = (0..n).map(|i| 12 + i * 16).find(|r| data.get(*r..*r + 4) == Some(b"name"))?;
    let (off, len) = (be32(data, r + 8)? as usize, be32(data, r + 12)? as usize);
    let t = data.get(off..off.checked_add(len)?)?;
    let (count, strings) = (be16(t, 2)? as usize, be16(t, 4)? as usize);
    let get = |want: u16| {
        (0..count).find_map(|i| {
            let r = 6 + i * 12;
            let (platform, encoding, id) = (be16(t, r)?, be16(t, r + 2)?, be16(t, r + 6)?);
            if platform != 3 || !matches!(encoding, 1 | 10) || id != want {
                return None;
            }
            let (l, o) = (be16(t, r + 8)? as usize, be16(t, r + 10)? as usize);
            let units: Vec<u16> = t.get(strings + o..strings + o + l)?.as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes(*c)).collect();
            Some(String::from_utf16_lossy(&units)).filter(|s| !s.is_empty())
        })
    };
    get(16).or_else(|| get(1))
}

fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    #[test]
    fn not_a_font_has_no_family() {
        assert_eq!(super::family(b""), None);
        assert_eq!(super::family(b"\x00\x01\x00\x00\x00\x01"), None);
        assert_eq!(super::family(&[0xff; 64]), None);
    }
}
