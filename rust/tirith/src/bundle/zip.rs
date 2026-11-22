//! A zip with its files stored as they are: the teach extension, downloaded from the page.

/// CRC-32 as zip wants it.
fn crc(data: &[u8]) -> u32 {
    !data.iter().fold(!0u32, |sum, byte| {
        (0..8).fold(sum ^ u32::from(*byte), |c, _| {
            (c >> 1) ^ (0xEDB8_8320 & (c & 1).wrapping_neg())
        })
    })
}

/// The fields a file's local and central headers share: version, flags (UTF-8), no compression, 1 Jan 2026, sums and lengths.
fn fields(name: &str, data: &[u8]) -> Vec<u8> {
    let mut out = vec![];
    for n in [20u16, 0x0800, 0, 0, (46 << 9) | (1 << 5) | 1] {
        out.extend(n.to_le_bytes());
    }
    for n in [crc(data), data.len() as u32, data.len() as u32] {
        out.extend(n.to_le_bytes());
    }
    out.extend((name.len() as u16).to_le_bytes());
    out.extend(0u16.to_le_bytes()); // no extra field
    out
}

/// The files as one zip, each below `folder`.
pub fn zip(folder: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
    let (mut body, mut index) = (vec![], vec![]);
    for (name, data) in files {
        let name = format!("{folder}/{name}");
        let (at, shared) = (body.len() as u32, fields(&name, data));
        body.extend(0x0403_4b50u32.to_le_bytes());
        body.extend(&shared);
        body.extend(name.as_bytes());
        body.extend(*data);
        index.extend(0x0201_4b50u32.to_le_bytes());
        index.extend(20u16.to_le_bytes());
        index.extend(&shared);
        index.extend([0u8; 10]); // no comment, disk 0, no attributes
        index.extend(at.to_le_bytes());
        index.extend(name.as_bytes());
    }
    let (count, size, at) = (files.len() as u16, index.len() as u32, body.len() as u32);
    body.extend(index);
    body.extend(0x0605_4b50u32.to_le_bytes());
    body.extend([0u8; 4]);
    body.extend(count.to_le_bytes());
    body.extend(count.to_le_bytes());
    body.extend(size.to_le_bytes());
    body.extend(at.to_le_bytes());
    body.extend(0u16.to_le_bytes());
    body
}
