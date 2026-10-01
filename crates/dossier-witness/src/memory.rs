#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub base: u64,
    pub size: u64,
    pub executable: bool,
    pub protect: u32,
    pub kind: u32,
}

impl Region {
    pub fn end(&self) -> u64 {
        self.base + self.size
    }
}

pub trait Memory {
    fn read(&self, at: u64, into: &mut [u8]) -> bool;
    fn regions(&self) -> Vec<Region>;
}

pub const STRING_MOST: usize = 4096;
pub const LIST_MOST: usize = 4_000_000;

pub trait Reads: Memory {
    fn bytes(&self, at: u64, len: usize) -> Option<Vec<u8>> {
        let mut out = vec![0; len];
        self.read(at, &mut out).then_some(out)
    }

    fn array<const N: usize>(&self, at: u64) -> Option<[u8; N]> {
        let mut out = [0; N];
        self.read(at, &mut out).then_some(out)
    }

    fn u8(&self, at: u64) -> Option<u8> {
        self.array::<1>(at).map(|b| b[0])
    }

    fn u16(&self, at: u64) -> Option<u16> {
        self.array(at).map(u16::from_le_bytes)
    }

    fn i32(&self, at: u64) -> Option<i32> {
        self.array(at).map(i32::from_le_bytes)
    }

    fn u32(&self, at: u64) -> Option<u32> {
        self.array(at).map(u32::from_le_bytes)
    }

    fn i64(&self, at: u64) -> Option<i64> {
        self.array(at).map(i64::from_le_bytes)
    }

    fn f32(&self, at: u64) -> Option<f32> {
        self.array(at).map(f32::from_le_bytes)
    }

    fn f64(&self, at: u64) -> Option<f64> {
        self.array(at).map(f64::from_le_bytes)
    }

    fn pointer(&self, at: u64) -> Option<u64> {
        self.u32(at).filter(|to| *to != 0).map(u64::from)
    }

    fn string(&self, object: u64) -> Option<String> {
        let len = self.i32(object + 4)?;
        if len < 0 || len as usize > STRING_MOST {
            return None;
        }
        let raw = self.bytes(object + 8, len as usize * 2)?;
        let units: Vec<u16> = raw.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
        String::from_utf16(&units).ok()
    }

    fn string_at(&self, field: u64) -> Option<String> {
        self.string(self.pointer(field)?)
    }

    fn list(&self, object: u64) -> Option<(u64, usize)> {
        let size = self.i32(object + 0xC)?;
        if size < 0 || size as usize > LIST_MOST {
            return None;
        }
        if size == 0 {
            return Some((0, 0));
        }
        let items = self.pointer(object + 4)?;
        let room = self.i32(items + 4)?;
        (room >= size).then_some((items + 8, size as usize))
    }
}

impl<M: Memory + ?Sized> Reads for M {}
