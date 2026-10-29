//! The system's memory as Activity Monitor's Memory footer has it (`p19a-monitor.md`
//! choice 5).

use std::{fs, path::Path};

use serde::Serialize;

/// Bytes, but for pressure.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Memory {
    pub physical: u64,
    /// Physical less what is available.
    pub used: u64,
    /// Anonymous and shared memory: what processes hold.
    pub apps: u64,
    /// Slab, kernel stacks, page tables and vmalloc.
    pub kernel: u64,
    /// The page cache less shared memory, which `Cached` also counts.
    pub cached: u64,
    pub swap_used: u64,
    /// What zram's compressed pages take, where there is zram.
    pub compressed: u64,
    /// The share of the last ten seconds some task waited on memory, 0 to 1.
    pub pressure: f32,
}

pub fn read(root: &Path) -> Memory {
    let meminfo = fs::read_to_string(root.join("proc/meminfo")).unwrap_or_default();
    let field = |name: &str| {
        meminfo
            .lines()
            .find_map(|l| l.strip_prefix(name))
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
            * 1024
    };
    let physical = field("MemTotal:");
    let shmem = field("Shmem:");
    Memory {
        physical,
        used: physical.saturating_sub(field("MemAvailable:")),
        apps: field("AnonPages:") + shmem,
        kernel: field("Slab:") + field("KernelStack:") + field("PageTables:") + field("VmallocUsed:"),
        cached: (field("Cached:") + field("Buffers:")).saturating_sub(shmem),
        swap_used: field("SwapTotal:").saturating_sub(field("SwapFree:")),
        compressed: zram(root),
        pressure: pressure(root),
    }
}

/// `mm_stat`'s third field, the memory zram uses, over every zram device.
fn zram(root: &Path) -> u64 {
    let Ok(dir) = fs::read_dir(root.join("sys/block")) else {
        return 0;
    };
    dir.flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("zram"))
        .filter_map(|e| fs::read_to_string(e.path().join("mm_stat")).ok())
        .filter_map(|s| s.split_whitespace().nth(2).and_then(|v| v.parse::<u64>().ok()))
        .sum()
}

fn pressure(root: &Path) -> f32 {
    fs::read_to_string(root.join("proc/pressure/memory"))
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with("some "))
        .and_then(|l| l.split_whitespace().find_map(|f| f.strip_prefix("avg10=")))
        .and_then(|v| v.parse::<f32>().ok())
        .map_or(0.0, |p| p / 100.0)
}

#[cfg(test)]
mod tests {
    use isle_fakes::tree::Tree;

    #[test]
    fn the_footer_splits_used_memory_and_reads_zram_and_pressure() {
        let tree = Tree::new("memory");
        tree.write(
            "proc/meminfo",
            "MemTotal: 1000 kB\nMemAvailable: 400 kB\nBuffers: 10 kB\nCached: 300 kB\nSwapTotal: 500 kB\n\
             SwapFree: 450 kB\nAnonPages: 200 kB\nShmem: 50 kB\nSlab: 40 kB\nKernelStack: 5 kB\n\
             PageTables: 3 kB\nVmallocUsed: 2 kB\n",
        )
        .write("sys/block/zram0/mm_stat", "4907630592 1482390784 1515364352 0 3646869504 20662 2021469 101006 906245\n")
        .write("proc/pressure/memory", "some avg10=12.50 avg60=0.02 avg300=0.00 total=1\nfull avg10=1.00 avg60=0 avg300=0 total=1\n");
        let m = super::read(tree.path());
        assert_eq!(m.physical, 1000 * 1024);
        assert_eq!(m.used, 600 * 1024);
        assert_eq!(m.apps, 250 * 1024);
        assert_eq!(m.kernel, 50 * 1024);
        assert_eq!(m.cached, 260 * 1024);
        assert_eq!(m.swap_used, 50 * 1024);
        assert_eq!(m.compressed, 1515364352);
        assert!((m.pressure - 0.125).abs() < 1e-6);
    }
}
