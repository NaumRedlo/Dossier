use crate::memory::{Memory, Region};

#[cfg(not(target_pointer_width = "64"))]
compile_error!("Witness is built for 64-bit Windows and reads 32-bit and 64-bit clients alike");

const PROCESS_VM_READ: u32 = 0x0010;
const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
const SNAP_PROCESSES: u32 = 0x0000_0002;
const MEM_COMMIT: u32 = 0x1000;
const PAGE_NOACCESS: u32 = 0x01;
const PAGE_GUARD: u32 = 0x100;
const EXECUTABLE: u32 = 0x10 | 0x20 | 0x40 | 0x80;
const INVALID: isize = -1;
const TOP: usize = 0x1_0000_0000;

#[repr(C)]
struct Info {
    base: usize,
    allocation_base: usize,
    allocation_protect: u32,
    partition: u16,
    size: usize,
    state: u32,
    protect: u32,
    kind: u32,
}

#[repr(C)]
struct Entry {
    size: u32,
    usage: u32,
    pid: u32,
    heap: usize,
    module: u32,
    threads: u32,
    parent: u32,
    priority: i32,
    flags: u32,
    name: [u16; 260],
}

#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
    fn CloseHandle(handle: isize) -> i32;
    fn ReadProcessMemory(process: isize, base: usize, buffer: *mut u8, size: usize, read: *mut usize) -> i32;
    fn VirtualQueryEx(process: isize, address: usize, info: *mut Info, length: usize) -> usize;
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> isize;
    fn Process32FirstW(snapshot: isize, entry: *mut Entry) -> i32;
    fn Process32NextW(snapshot: isize, entry: *mut Entry) -> i32;
    fn QueryFullProcessImageNameW(process: isize, flags: u32, name: *mut u16, size: *mut u32) -> i32;
}

const PATH_MOST: usize = 1024;

pub fn processes_named(name: &str) -> Vec<u32> {
    let mut found = Vec::new();
    let snapshot = unsafe { CreateToolhelp32Snapshot(SNAP_PROCESSES, 0) };
    if snapshot == INVALID || snapshot == 0 {
        return found;
    }
    let mut entry: Entry = unsafe { std::mem::zeroed() };
    entry.size = std::mem::size_of::<Entry>() as u32;
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while more {
        let len = entry.name.iter().position(|unit| *unit == 0).unwrap_or(entry.name.len());
        if String::from_utf16_lossy(&entry.name[..len]).eq_ignore_ascii_case(name) {
            found.push(entry.pid);
        }
        more = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot) };
    found
}

pub struct Process {
    handle: isize,
    pub pid: u32,
}

impl Process {
    pub fn open(pid: u32) -> Option<Process> {
        let handle = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, 0, pid) };
        (handle != 0 && handle != INVALID).then_some(Process { handle, pid })
    }
}

impl Process {
    pub fn folder(&self) -> Option<std::path::PathBuf> {
        let mut name = [0u16; PATH_MOST];
        let mut size = PATH_MOST as u32;
        let found = unsafe { QueryFullProcessImageNameW(self.handle, 0, name.as_mut_ptr(), &mut size) };
        if found == 0 || size == 0 {
            return None;
        }
        let path = std::path::PathBuf::from(String::from_utf16_lossy(&name[..size as usize]));
        path.parent().map(std::path::Path::to_path_buf)
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}

impl Memory for Process {
    fn read(&self, at: u64, into: &mut [u8]) -> bool {
        let mut read = 0usize;
        let done = unsafe { ReadProcessMemory(self.handle, at as usize, into.as_mut_ptr(), into.len(), &mut read) };
        done != 0 && read == into.len()
    }

    fn regions(&self) -> Vec<Region> {
        let mut found = Vec::new();
        let mut at = 0usize;
        while at < TOP {
            let mut info: Info = unsafe { std::mem::zeroed() };
            if unsafe { VirtualQueryEx(self.handle, at, &mut info, std::mem::size_of::<Info>()) } == 0 || info.size == 0 {
                break;
            }
            if info.state == MEM_COMMIT && info.protect & (PAGE_NOACCESS | PAGE_GUARD) == 0 && info.protect != 0 {
                found.push(Region { base: info.base as u64, size: info.size as u64, executable: info.protect & EXECUTABLE != 0, protect: info.protect, kind: info.kind });
            }
            at = info.base.saturating_add(info.size);
        }
        found
    }
}

#[link(name = "user32")]
extern "system" {
    fn EnumWindows(each: extern "system" fn(isize, isize) -> i32, given: isize) -> i32;
    fn GetWindowThreadProcessId(window: isize, pid: *mut u32) -> u32;
    fn GetWindowTextW(window: isize, into: *mut u16, most: i32) -> i32;
    fn GetClassNameW(window: isize, into: *mut u16, most: i32) -> i32;
    fn IsWindowVisible(window: isize) -> i32;
    fn EnumChildWindows(parent: isize, each: extern "system" fn(isize, isize) -> i32, given: isize) -> i32;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub pid: u32,
    pub title: String,
    pub class: String,
    pub visible: bool,
}

extern "system" fn gather(window: isize, given: isize) -> i32 {
    let found = unsafe { &mut *(given as *mut Vec<Window>) };
    let mut pid = 0u32;
    let mut title = [0u16; 256];
    let mut class = [0u16; 128];
    let (titled, classed, visible) = unsafe {
        GetWindowThreadProcessId(window, &mut pid);
        (GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32), GetClassNameW(window, class.as_mut_ptr(), class.len() as i32), IsWindowVisible(window) != 0)
    };
    let top = found.len();
    found.push(Window { pid, title: String::from_utf16_lossy(&title[..titled.max(0) as usize]), class: String::from_utf16_lossy(&class[..classed.max(0) as usize]), visible });
    if found[top].class == "#32770" {
        unsafe { EnumChildWindows(window, gather_child, given) };
    }
    1
}

extern "system" fn gather_child(window: isize, given: isize) -> i32 {
    let found = unsafe { &mut *(given as *mut Vec<Window>) };
    let mut pid = 0u32;
    let mut title = [0u16; 1024];
    let mut class = [0u16; 128];
    let (titled, classed) = unsafe {
        GetWindowThreadProcessId(window, &mut pid);
        (GetWindowTextW(window, title.as_mut_ptr(), title.len() as i32), GetClassNameW(window, class.as_mut_ptr(), class.len() as i32))
    };
    found.push(Window { pid, title: String::from_utf16_lossy(&title[..titled.max(0) as usize]), class: format!("child {}", String::from_utf16_lossy(&class[..classed.max(0) as usize])), visible: true });
    1
}

pub fn windows_of(pid: u32) -> Vec<Window> {
    let mut found: Vec<Window> = Vec::new();
    unsafe { EnumWindows(gather, &mut found as *mut Vec<Window> as isize) };
    found.retain(|window| window.pid == pid);
    found
}
