const SHIFTS: [u32; 64] = [7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21];

pub fn digest(data: &[u8]) -> [u8; 16] {
    let sines: Vec<u32> = (0..64).map(|n| ((f64::from(n) + 1.0).sin().abs() * 4_294_967_296.0) as u32).collect();
    let mut state: [u32; 4] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476];
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_le_bytes());
    for block in padded.chunks_exact(64) {
        let words: Vec<u32> = block.chunks_exact(4).map(|word| u32::from_le_bytes([word[0], word[1], word[2], word[3]])).collect();
        let [mut a, mut b, mut c, mut d] = state;
        for n in 0..64 {
            let (mixed, at) = match n / 16 {
                0 => ((b & c) | (!b & d), n),
                1 => ((d & b) | (!d & c), (5 * n + 1) % 16),
                2 => (b ^ c ^ d, (3 * n + 5) % 16),
                _ => (c ^ (b | !d), (7 * n) % 16),
            };
            let turned = a.wrapping_add(mixed).wrapping_add(sines[n]).wrapping_add(words[at]).rotate_left(SHIFTS[n]);
            (a, d, c, b) = (d, c, b, b.wrapping_add(turned));
        }
        state = [state[0].wrapping_add(a), state[1].wrapping_add(b), state[2].wrapping_add(c), state[3].wrapping_add(d)];
    }
    let mut out = [0u8; 16];
    for (n, word) in state.iter().enumerate() {
        out[n * 4..n * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    out
}

pub fn hex(data: &[u8]) -> String {
    digest(data).iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_digest_matches_the_published_examples() {
        assert_eq!(hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(hex(b"The quick brown fox jumps over the lazy dog"), "9e107d9d372bb6826bd81d3542a419d6");
        assert_eq!(hex(&[b'a'; 1000]), "cabe45dcc9ae5b66ba86600cca6b8ba8");
    }
}
