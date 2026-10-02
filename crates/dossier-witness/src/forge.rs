use crate::beatmaps::{ENTRY_SIZED_BEFORE, STARS_FROM};

pub struct Bytes(Vec<u8>);

impl Bytes {
    fn text(&mut self, said: &str) {
        if said.is_empty() {
            self.0.push(0);
            return;
        }
        self.0.push(0x0b);
        let mut length = said.len();
        loop {
            let low = (length & 0x7f) as u8;
            length >>= 7;
            if length == 0 {
                self.0.push(low);
                break;
            }
            self.0.push(low | 0x80);
        }
        self.0.extend_from_slice(said.as_bytes());
    }

    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn i64(&mut self, v: i64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn i16(&mut self, v: i16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
}

pub struct Spec {
    pub md5: &'static str,
    pub difficulty: &'static str,
    pub status: u8,
    pub mode: u8,
    pub id: i32,
    pub set: i32,
    pub stars: Vec<(i32, f32)>,
    pub points: Vec<(f64, f64, bool)>,
    pub total: i32,
}

pub fn spec(md5: &'static str) -> Spec {
    Spec { md5, difficulty: "Insane", status: 4, mode: 0, id: 11, set: 5, stars: vec![(0, 5.0), (64, 7.5), (16, 5.6), (1024, 6.0)], points: vec![(500.0, 0.0, true)], total: 120_000 }
}

pub fn entry_bytes(version: i32, spec: &Spec, padding: usize) -> Vec<u8> {
    let mut body = Bytes(Vec::new());
    for field in ["Artist", "", "Title", "", "Mapper"] {
        body.text(field);
    }
    body.text(spec.difficulty);
    body.text("audio.mp3");
    body.text(spec.md5);
    body.text("map.osu");
    body.0.push(spec.status);
    body.u16(100);
    body.u16(50);
    body.u16(2);
    body.i64(0);
    if version >= STARS_FROM {
        for v in [9.0f32, 4.0, 6.0, 8.0] {
            body.f32(v);
        }
    } else {
        body.0.extend_from_slice(&[9, 4, 6, 8]);
    }
    body.f64(1.4);
    if version >= STARS_FROM {
        for mode in 0..4 {
            if mode == 0 {
                body.i32(spec.stars.len() as i32);
                for (mods, stars) in &spec.stars {
                    body.0.push(0x08);
                    body.i32(*mods);
                    if version >= 20_250_107 {
                        body.0.push(0x0c);
                        body.f32(*stars);
                    } else {
                        body.0.push(0x0d);
                        body.f64(f64::from(*stars));
                    }
                }
            } else {
                body.i32(0);
            }
        }
    }
    body.i32(90);
    body.i32(spec.total);
    body.i32(1000);
    body.i32(spec.points.len() as i32);
    for (length, offset, uninherited) in &spec.points {
        body.f64(*length);
        body.f64(*offset);
        body.0.push(u8::from(*uninherited));
    }
    body.i32(spec.id);
    body.i32(spec.set);
    body.i32(0);
    body.0.extend_from_slice(&[9, 9, 9, 9]);
    body.i16(0);
    body.f32(0.7);
    body.0.push(spec.mode);
    body.text("");
    body.text("tag");
    body.i16(0);
    body.text("");
    body.0.push(1);
    body.i64(0);
    body.0.push(0);
    body.text("folder");
    body.i64(0);
    body.0.extend_from_slice(&[0, 0, 0, 0, 0]);
    if version < STARS_FROM {
        body.i16(0);
    }
    body.i32(0);
    body.0.push(0);
    body.0.extend(std::iter::repeat_n(0xAA, padding));
    if version < ENTRY_SIZED_BEFORE {
        let mut sized = Bytes(Vec::new());
        sized.i32(body.0.len() as i32);
        sized.0.extend(body.0);
        return sized.0;
    }
    body.0
}

pub fn library_bytes(version: i32, entries: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Bytes(Vec::new());
    out.i32(version);
    out.i32(3);
    out.0.push(1);
    out.i64(0);
    out.text("Player");
    out.i32(entries.len() as i32);
    for entry in entries {
        out.0.extend_from_slice(entry);
    }
    out.i32(0);
    out.0
}

