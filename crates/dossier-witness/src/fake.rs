use crate::memory::{Memory, Region};

#[derive(Debug, Default, Clone)]
pub struct Fake {
    blocks: Vec<(u64, Vec<u8>, bool)>,
}

impl Fake {
    pub fn put(&mut self, at: u64, bytes: &[u8], executable: bool) {
        self.blocks.push((at, bytes.to_vec(), executable));
        self.blocks.sort_by_key(|block| block.0);
    }

    pub fn room(&mut self, at: u64, size: usize) {
        self.put(at, &vec![0; size], false);
    }

    pub fn write(&mut self, at: u64, bytes: &[u8]) {
        let block = self.blocks.iter_mut().find(|(base, held, _)| at >= *base && at + bytes.len() as u64 <= *base + held.len() as u64).expect("room for the write");
        let from = (at - block.0) as usize;
        block.1[from..from + bytes.len()].copy_from_slice(bytes);
    }

    pub fn set_u32(&mut self, at: u64, value: u32) {
        self.write(at, &value.to_le_bytes());
    }

    pub fn set_string(&mut self, object: u64, said: &str) {
        let units: Vec<u16> = said.encode_utf16().collect();
        self.set_u32(object + 4, units.len() as u32);
        let raw: Vec<u8> = units.iter().flat_map(|unit| unit.to_le_bytes()).collect();
        self.write(object + 8, &raw);
    }
}

impl Memory for Fake {
    fn read(&self, at: u64, into: &mut [u8]) -> bool {
        match self.blocks.iter().find(|(base, held, _)| at >= *base && at + into.len() as u64 <= *base + held.len() as u64) {
            Some((base, held, _)) => {
                let from = (at - base) as usize;
                into.copy_from_slice(&held[from..from + into.len()]);
                true
            }
            None => false,
        }
    }

    fn regions(&self) -> Vec<Region> {
        self.blocks.iter().map(|(base, held, executable)| Region { base: *base, size: held.len() as u64, executable: *executable, protect: if *executable { 0x40 } else { 0x04 }, kind: 0x20000 }).collect()
    }
}
