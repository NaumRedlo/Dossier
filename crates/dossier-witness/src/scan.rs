use crate::memory::Memory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    bytes: Vec<Option<u8>>,
}

impl Pattern {
    pub fn parse(said: &str) -> Option<Pattern> {
        let bytes: Option<Vec<Option<u8>>> = said
            .split_whitespace()
            .map(|part| match part {
                "??" | "?" => Some(None),
                _ => u8::from_str_radix(part, 16).ok().map(Some),
            })
            .collect();
        let bytes = bytes?;
        (!bytes.is_empty() && bytes[0].is_some()).then_some(Pattern { bytes })
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn fits(&self, hay: &[u8]) -> bool {
        self.bytes.iter().zip(hay).all(|(want, have)| want.is_none_or(|want| want == *have))
    }

    pub fn find_in(&self, hay: &[u8]) -> Option<usize> {
        self.all_in(hay).next()
    }

    pub fn all_in<'a>(&'a self, hay: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
        let first = self.bytes[0].unwrap_or(0);
        let last = hay.len().saturating_sub(self.len().saturating_sub(1));
        (0..last).filter(move |at| hay[*at] == first && self.fits(&hay[*at..]))
    }
}

pub const CHUNK: usize = 1 << 20;

pub fn find_all(memory: &dyn Memory, pattern: &Pattern, executable_only: bool, most: usize) -> Vec<u64> {
    let mut found = Vec::new();
    let mut buffer = vec![0u8; CHUNK];
    let overlap = pattern.len().saturating_sub(1);
    for region in memory.regions() {
        if (executable_only && !region.executable) || (region.size as usize) < pattern.len() {
            continue;
        }
        let size = region.size as usize;
        let mut from = 0;
        while from < size {
            let len = CHUNK.min(size - from);
            let at = region.base + from as u64;
            if len >= pattern.len() && memory.read(at, &mut buffer[..len]) {
                for hit in pattern.all_in(&buffer[..len]) {
                    let address = at + hit as u64;
                    if !found.contains(&address) {
                        found.push(address);
                    }
                    if found.len() >= most {
                        return found;
                    }
                }
            }
            if len < CHUNK {
                break;
            }
            from += CHUNK - overlap;
        }
    }
    found
}

pub fn find(memory: &dyn Memory, pattern: &Pattern) -> Option<u64> {
    find_all(memory, pattern, true, 1).first().copied().or_else(|| find_all(memory, pattern, false, 1).first().copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::Fake;

    #[test]
    fn a_pattern_reads_bytes_and_wildcards_and_refuses_nonsense() {
        let pattern = Pattern::parse("F8 01 ?? 04").unwrap();
        assert_eq!(pattern.len(), 4);
        assert_eq!(pattern.find_in(&[0, 0xF8, 0x01, 0x77, 0x04, 9]), Some(1));
        assert_eq!(pattern.find_in(&[0xF8, 0x01, 0x77, 0x05]), None);
        assert_eq!(pattern.find_in(&[0xF8, 0x01, 0x77]), None);
        assert!(Pattern::parse("").is_none() && Pattern::parse("ZZ 01").is_none() && Pattern::parse("?? 01").is_none());
    }

    #[test]
    fn a_pattern_is_found_across_the_seam_of_two_reads_and_only_once() {
        let mut fake = Fake::default();
        let mut code = vec![0u8; CHUNK + 64];
        code[CHUNK - 2..CHUNK + 2].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        fake.put(0x40_0000, &code, true);
        let pattern = Pattern::parse("AA BB ?? DD").unwrap();
        assert_eq!(find_all(&fake, &pattern, true, 8), vec![0x40_0000 + CHUNK as u64 - 2]);
    }

    #[test]
    fn code_is_searched_first_and_data_only_when_code_has_nothing() {
        let mut fake = Fake::default();
        fake.put(0x10_0000, &[1, 2, 3, 4], false);
        fake.put(0x20_0000, &[9, 1, 2, 3, 4], true);
        let pattern = Pattern::parse("01 02 03").unwrap();
        assert_eq!(find(&fake, &pattern), Some(0x20_0001));
        let only_data = Pattern::parse("01 02 03 04").unwrap();
        let mut plain = Fake::default();
        plain.put(0x10_0000, &[1, 2, 3, 4], false);
        assert_eq!(find(&plain, &only_data), Some(0x10_0000));
        assert_eq!(find(&plain, &Pattern::parse("05 06").unwrap()), None);
    }
}
