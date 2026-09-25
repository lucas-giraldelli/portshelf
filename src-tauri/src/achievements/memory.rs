//! Reading a running game's memory. N64: Recompiled ports reserve 4 GB of address space and
//! make the first 512 MB readable and writable; N64 address 0x80000000 is the start of that
//! block, kept as native (little-endian) 32-bit words, so a byte at N64 address A is at A ^ 3.
//! Linux only lets a process read the memory of its descendants (Yama ptrace_scope 1), which
//! the ports PortShelf starts are.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

const MEM_SIZE: u64 = 512 * 1024 * 1024;

/// N64 RAM inside a recompiled port's process.
pub struct N64Ram {
    mem: File,
    base: u64,
}

impl N64Ram {
    /// Finds the RAM block of a running port: 512 MB, read/write, anonymous, followed by the
    /// rest of the reservation with no access. None until the port has allocated it.
    pub fn attach(pid: u32) -> Option<Self> {
        let maps = std::fs::read_to_string(format!("/proc/{pid}/maps")).ok()?;
        let lines: Vec<&str> = maps.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let (Some(range), Some(perms)) = (fields.first(), fields.get(1)) else { continue };
            let Some((start, end)) = range.split_once('-') else { continue };
            let (Ok(start), Ok(end)) = (u64::from_str_radix(start, 16), u64::from_str_radix(end, 16)) else { continue };
            let anonymous = fields.len() == 5;
            let next_blocked = lines
                .get(i + 1)
                .is_some_and(|n| n.starts_with(&format!("{end:x}-")) && n.split_whitespace().nth(1) == Some("---p"));
            if end - start == MEM_SIZE && perms.starts_with("rw") && anonymous && next_blocked {
                let mem = File::open(format!("/proc/{pid}/mem")).ok()?;
                return Some(N64Ram { mem, base: start });
            }
        }
        None
    }

    fn raw(&mut self, n64_addr: u32, buf: &mut [u8]) -> std::io::Result<()> {
        let offset = n64_addr.wrapping_sub(0x8000_0000) as u64;
        if offset + buf.len() as u64 > MEM_SIZE {
            return Err(std::io::Error::other("address outside N64 RAM"));
        }
        self.mem.seek(SeekFrom::Start(self.base + offset))?;
        self.mem.read_exact(buf)
    }

    /// A value as the game sees it: 1, 2 or 4 bytes, big-endian on the N64.
    pub fn read(&mut self, n64_addr: u32, size: u32) -> std::io::Result<u32> {
        let mut word = [0u8; 4];
        match size {
            4 => {
                self.raw(n64_addr & !3, &mut word)?;
                Ok(u32::from_le_bytes(word))
            }
            2 => {
                self.raw(n64_addr & !3, &mut word)?;
                let w = u32::from_le_bytes(word);
                Ok(if n64_addr & 2 == 0 { w >> 16 } else { w & 0xFFFF })
            }
            _ => {
                let mut b = [0u8; 1];
                self.raw(n64_addr ^ 3, &mut b)?;
                Ok(b[0] as u32)
            }
        }
    }
}
