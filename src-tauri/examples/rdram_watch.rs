//! Proof of concept for achievements: starts an N64: Recompiled port, finds the N64 RAM inside
//! its process and prints Banjo-Kazooie's jiggy and item counters whenever they change.
//! cargo run --example rdram_watch -- <program> <log file> [program arguments]
//!
//! The recompilation runtime reserves 4 GB of address space and makes the first 512 MB readable
//! and writable; N64 address 0x80000000 is the start of that block. Memory is kept as native
//! (little-endian) 32-bit words, so a byte at N64 address A is at offset (A ^ 3).
//! Linux only lets a process read the memory of its own children (Yama ptrace_scope 1), which is
//! why this tool starts the game itself, as PortShelf does.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::Command;
use std::time::{Duration, Instant};

const MEM_SIZE: u64 = 512 * 1024 * 1024;
const JIGGY_FLAGS: u32 = 0x8038_32C0; // u8[13], one bit per jiggy (decomp: jiggyscore)
const ITEMS: u32 = 0x8038_5F30; // s32[0x2C] (decomp: item_getCount)
const OS_MEM_SIZE: u32 = 0x8000_0318; // written by the boot code: 4 or 8 MB
const LEVEL: u32 = 0x8038_3301; // u8 (decomp: D_80383300.level)

/// Start of the port's N64 RAM: a 512 MB read/write anonymous block followed by the rest of
/// the 4 GB reservation with no access.
fn find_rdram(pid: u32) -> Option<u64> {
    let maps = std::fs::read_to_string(format!("/proc/{pid}/maps")).ok()?;
    let lines: Vec<&str> = maps.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let mut parts = line.split_whitespace();
        let range = parts.next()?;
        let perms = parts.next()?;
        let (start, end) = range.split_once('-')?;
        let (start, end) = (u64::from_str_radix(start, 16).ok()?, u64::from_str_radix(end, 16).ok()?);
        let anonymous = line.split_whitespace().count() == 5;
        let next_blocked = lines.get(i + 1).is_some_and(|n| n.starts_with(&format!("{end:x}-")) && n.split_whitespace().nth(1) == Some("---p"));
        if end - start == MEM_SIZE && perms.starts_with("rw") && anonymous && next_blocked {
            return Some(start);
        }
    }
    None
}

struct Ram {
    mem: File,
    base: u64,
}

impl Ram {
    fn read(&mut self, n64_addr: u32, len: usize) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0; len];
        self.mem.seek(SeekFrom::Start(self.base + (n64_addr - 0x8000_0000) as u64))?;
        self.mem.read_exact(&mut buf)?;
        Ok(buf)
    }
    fn word(&mut self, n64_addr: u32) -> std::io::Result<u32> {
        let b = self.read(n64_addr, 4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    /// Bytes as the game sees them (big-endian order), from native words.
    fn bytes(&mut self, n64_addr: u32, len: usize) -> std::io::Result<Vec<u8>> {
        let start = n64_addr & !3;
        let raw = self.read(start, ((n64_addr - start) as usize + len + 3) & !3)?;
        Ok((0..len).map(|i| raw[((n64_addr - start) as usize + i) ^ 3]).collect())
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let program = std::path::PathBuf::from(&args[1]);
    let mut log: Box<dyn Write> = match args.get(2) {
        Some(path) => Box::new(File::create(path).unwrap()),
        None => Box::new(std::io::stdout()),
    };
    let mut child = Command::new(&program).args(&args[3..]).current_dir(program.parent().unwrap()).spawn().expect("start the port");
    let started = Instant::now();
    writeln!(log, "started pid {}", child.id()).unwrap();

    let base = loop {
        if let Some(base) = find_rdram(child.id()) {
            break base;
        }
        if child.try_wait().unwrap().is_some() {
            writeln!(log, "the port exited before its RAM appeared").unwrap();
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    let mut ram = Ram { mem: File::open(format!("/proc/{}/mem", child.id())).expect("open /proc/pid/mem"), base };
    writeln!(log, "N64 RAM at {base:#x} after {:?}", started.elapsed()).unwrap();

    let mut last = (Vec::new(), Vec::new(), 0);
    loop {
        if child.try_wait().unwrap().is_some() {
            writeln!(log, "the port exited").unwrap();
            return;
        }
        let snapshot = (|| -> std::io::Result<_> {
            let mem_size = ram.word(OS_MEM_SIZE)?;
            let mut flags = ram.bytes(JIGGY_FLAGS, 13)?;
            flags.push(ram.bytes(LEVEL, 1)?[0]);
            let items: Vec<u32> = (0..0x2C).map(|i| ram.word(ITEMS + i * 4)).collect::<Result<_, _>>()?;
            Ok((flags, items, mem_size))
        })();
        match snapshot {
            Ok(now) if now != last => {
                let (flags, items, mem_size) = &now;
                let collected: Vec<usize> = (1..=100).filter(|id| flags[(id - 1) / 8] & (1 << (id & 7)) != 0).collect();
                let level = flags[13];
                writeln!(
                    log,
                    "[{:>6.1}s] level={level} osMemSize={mem_size:#x} notes={} jiggies_held={} jiggies_total={} mumbo_total={} collected={collected:?}",
                    started.elapsed().as_secs_f32(),
                    items[0xC] as i32,
                    items[0xE] as i32,
                    items[0x26] as i32,
                    items[0x25] as i32,
                )
                .unwrap();
                log.flush().unwrap();
                last = now;
            }
            Ok(_) => {}
            Err(e) => writeln!(log, "read failed: {e}").unwrap(),
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
