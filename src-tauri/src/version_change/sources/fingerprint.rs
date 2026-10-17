/// CurseForge's file fingerprint: 32-bit MurmurHash2 (seed 1) of the file without
/// whitespace bytes (tab, newline, carriage return, space)
pub fn cf_fingerprint(data: &[u8]) -> u32 {
    let bytes: Vec<u8> = data
        .iter()
        .copied()
        .filter(|b| !matches!(b, 9 | 10 | 13 | 32))
        .collect();
    const M: u32 = 0x5bd1_e995;
    let mut h: u32 = 1 ^ (bytes.len() as u32);
    let (chunks, tail) = bytes.as_chunks::<4>();
    for chunk in chunks {
        let mut k = u32::from_le_bytes(*chunk);
        k = k.wrapping_mul(M);
        k ^= k >> 24;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M) ^ k;
    }
    if !tail.is_empty() {
        if tail.len() >= 3 {
            h ^= (tail[2] as u32) << 16;
        }
        if tail.len() >= 2 {
            h ^= (tail[1] as u32) << 8;
        }
        h ^= tail[0] as u32;
        h = h.wrapping_mul(M);
    }
    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;
    h
}
