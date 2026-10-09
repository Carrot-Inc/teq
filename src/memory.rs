//! The machine's memory as the typer's automatic worker count reads it: one worker per GiB
//! above the first, so that a machine too small for the cap's peak
//! types with fewer workers or with one.

const GIB: u64 = 1 << 30;

/// The workers the machine's memory allows: one per GiB of what a process may use above the
/// first, at least one; no bound where the memory cannot be read.
pub fn workers() -> usize {
    static N: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *N.get_or_init(|| available().map_or(usize::MAX, workers_for))
}

/// The resident bytes of the calling thread's stack (`mincore` over its mapping), for
/// `TEQ_SESSION_INVENTORY`: what a thread kept between builds would keep of its deepest
/// recursion. Zero where the stack cannot be found.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn stack_resident() -> usize {
    extern "C" {
        fn pthread_self() -> usize;
        fn mincore(addr: *mut u8, len: usize, vec: *mut u8) -> std::os::raw::c_int;
        fn sysconf(name: std::os::raw::c_int) -> std::os::raw::c_long;
        #[cfg(target_os = "macos")]
        fn pthread_get_stackaddr_np(thread: usize) -> *mut u8;
        #[cfg(target_os = "macos")]
        fn pthread_get_stacksize_np(thread: usize) -> usize;
        #[cfg(target_os = "linux")]
        fn pthread_getattr_np(thread: usize, attr: *mut u8) -> std::os::raw::c_int;
        #[cfg(target_os = "linux")]
        fn pthread_attr_getstack(attr: *const u8, addr: *mut *mut u8, size: *mut usize) -> std::os::raw::c_int;
        #[cfg(target_os = "linux")]
        fn pthread_attr_destroy(attr: *mut u8) -> std::os::raw::c_int;
    }
    #[cfg(target_os = "linux")]
    const PAGE_SIZE: std::os::raw::c_int = 30;
    #[cfg(target_os = "macos")]
    const PAGE_SIZE: std::os::raw::c_int = 29;
    unsafe {
        let page = sysconf(PAGE_SIZE).max(1) as usize;
        #[cfg(target_os = "macos")]
        let (low, len) = {
            let len = pthread_get_stacksize_np(pthread_self());
            ((pthread_get_stackaddr_np(pthread_self()) as usize).saturating_sub(len), len)
        };
        #[cfg(target_os = "linux")]
        let (low, len) = {
            let mut attr = [0u64; 16];
            if pthread_getattr_np(pthread_self(), attr.as_mut_ptr() as *mut u8) != 0 {
                return 0;
            }
            let (mut addr, mut len) = (std::ptr::null_mut(), 0usize);
            let got = pthread_attr_getstack(attr.as_ptr() as *const u8, &mut addr, &mut len);
            pthread_attr_destroy(attr.as_mut_ptr() as *mut u8);
            if got != 0 {
                return 0;
            }
            (addr as usize, len)
        };
        let low = low.next_multiple_of(page);
        let len = len / page * page;
        let mut vec = vec![0u8; len / page];
        if len == 0 || mincore(low as *mut u8, len, vec.as_mut_ptr()) != 0 {
            return 0;
        }
        vec.iter().filter(|&&v| v & 1 != 0).count() * page
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn stack_resident() -> usize {
    0
}

fn workers_for(memory: u64) -> usize {
    (memory.saturating_sub(GIB) / GIB).max(1) as usize
}

/// What a process of this machine may use: the physical memory, and on Linux the cgroup's
/// limit where it is lower (a container's `sysconf` reports the host's memory, where
/// `available_parallelism` already reads the cgroup's CPU quota). A Windows job object's limit,
/// that system's container, is not read.
fn available() -> Option<u64> {
    match (physical(), cgroup_limit()) {
        (Some(p), Some(l)) => Some(p.min(l)),
        (p, l) => p.or(l),
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn physical() -> Option<u64> {
    extern "C" {
        fn sysconf(name: std::os::raw::c_int) -> std::os::raw::c_long;
    }
    #[cfg(target_os = "linux")]
    const PHYS_PAGES_AND_PAGE_SIZE: (std::os::raw::c_int, std::os::raw::c_int) = (85, 30);
    #[cfg(target_os = "macos")]
    const PHYS_PAGES_AND_PAGE_SIZE: (std::os::raw::c_int, std::os::raw::c_int) = (200, 29);
    let (pages, size) = unsafe { (sysconf(PHYS_PAGES_AND_PAGE_SIZE.0), sysconf(PHYS_PAGES_AND_PAGE_SIZE.1)) };
    (pages > 0 && size > 0).then(|| pages as u64 * size as u64)
}

#[cfg(windows)]
fn physical() -> Option<u64> {
    /// `MEMORYSTATUSEX`: its length, the load, then the physical memory in bytes and the rest.
    #[repr(C)]
    struct MemoryStatus {
        length: u32,
        load: u32,
        total_phys: u64,
        rest: [u64; 6],
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatus) -> i32;
    }
    let mut status = MemoryStatus { length: std::mem::size_of::<MemoryStatus>() as u32, load: 0, total_phys: 0, rest: [0; 6] };
    // SAFETY: a structure of the length it declares, written by the call.
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) } != 0;
    (ok && status.total_phys > 0).then_some(status.total_phys)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn physical() -> Option<u64> {
    None
}

#[cfg(target_os = "linux")]
fn cgroup_limit() -> Option<u64> {
    let own = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let mounts = std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default();
    limit_of(&own, &mounts, |path| std::fs::read_to_string(path).ok())
}

#[cfg(not(target_os = "linux"))]
fn cgroup_limit() -> Option<u64> {
    None
}

/// The lowest memory limit of the process's cgroups and their ancestors, from `/proc/self/cgroup`'s
/// lines (`0::<path>` for the unified hierarchy, `<n>:<controllers>:<path>` for version 1's) and
/// `/proc/self/mountinfo` (`mounts`), which says where each hierarchy is mounted and which of its
/// cgroups the mount shows: each directory's `memory.max`, or `memory.limit_in_bytes` under the
/// memory controller, from the cgroup's own up through the ancestors the mount shows. Without a
/// mount of the hierarchy, its usual place, `/sys/fs/cgroup` or `/sys/fs/cgroup/memory`, showing the
/// whole of it. `max` is no limit.
#[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
fn limit_of(own: &str, mounts: &str, read: impl Fn(&str) -> Option<String>) -> Option<u64> {
    own.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, ':');
            let (_, controllers, path) = (parts.next()?, parts.next()?, parts.next()?);
            let (unified, file, usual) = if controllers.is_empty() {
                (true, "memory.max", "/sys/fs/cgroup")
            } else if controllers.split(',').any(|c| c == "memory") {
                (false, "memory.limit_in_bytes", "/sys/fs/cgroup/memory")
            } else {
                return None;
            };
            let mut shown = mounts_of(mounts, unified);
            if shown.is_empty() {
                shown.push(("/".to_string(), usual.to_string()));
            }
            shown.iter().filter_map(|(root, point)| shown_limit(path, root, point, file, &read)).min()
        })
        .min()
}

/// The mounts of the unified hierarchy (`cgroup2`), or of version 1's with the memory controller,
/// from mountinfo's lines (`<id> <parent> <dev> <root> <mount point> <options> [<optional>...] -
/// <type> <source> <super options>`): each mount's root, the cgroup it shows at its mount point,
/// and the mount point.
fn mounts_of(mounts: &str, unified: bool) -> Vec<(String, String)> {
    mounts
        .lines()
        .filter_map(|line| {
            let (mount, filesystem) = line.split_once(" - ")?;
            let mut fields = mount.split(' ');
            let (root, point) = (fields.nth(3)?, fields.next()?);
            let mut filesystem = filesystem.split(' ');
            let (kind, options) = (filesystem.next()?, filesystem.nth(1)?);
            let ours = if unified { kind == "cgroup2" } else { kind == "cgroup" && options.split(',').any(|o| o == "memory") };
            ours.then(|| (unescape(root), unescape(point)))
        })
        .collect()
}

/// mountinfo's escapes of a path's space, tab, newline and backslash (`\040`).
fn unescape(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let octal = bytes.get(i + 1..i + 4).filter(|d| bytes[i] == b'\\' && d.iter().all(|b| (b'0'..=b'7').contains(b)));
        match octal {
            Some(d) => {
                out.push(d.iter().fold(0u8, |n, b| n.wrapping_mul(8).wrapping_add(b - b'0')));
                i += 4;
            }
            None => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The lowest limit of cgroup `path` and its ancestors that a mount of `root` at `point` shows; none
/// where the mount does not show the cgroup.
fn shown_limit(path: &str, root: &str, point: &str, file: &str, read: &impl Fn(&str) -> Option<String>) -> Option<u64> {
    let below = path.trim_end_matches('/').strip_prefix(root.trim_end_matches('/'))?;
    if !below.is_empty() && !below.starts_with('/') {
        return None;
    }
    let point = point.trim_end_matches('/');
    let mut dir = below;
    let mut lowest: Option<u64> = None;
    loop {
        if let Some(limit) = read(&format!("{}{}/{}", point, dir, file)).and_then(|text| text.trim().parse::<u64>().ok()) {
            lowest = Some(lowest.map_or(limit, |l| l.min(limit)));
        }
        match dir.rfind('/') {
            Some(i) => dir = &dir[..i],
            None => break,
        }
    }
    lowest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_worker_per_gib_above_the_first() {
        assert_eq!(workers_for(GIB / 2), 1);
        assert_eq!(workers_for(2 * GIB), 1);
        assert_eq!(workers_for(4 * GIB), 3);
        assert_eq!(workers_for(8 * GIB), 7);
        assert_eq!(workers_for(9 * GIB), 8);
        assert_eq!(workers_for(16 * GIB - 1), 14);
    }

    #[test]
    fn the_cgroup_limits() {
        let files = |entries: &'static [(&'static str, &'static str)]| move |p: &str| entries.iter().find(|(f, _)| *f == p).map(|(_, t)| t.to_string());
        assert_eq!(limit_of("0::/\n", "", files(&[("/sys/fs/cgroup/memory.max", "4294967296\n")])), Some(4 * GIB));
        assert_eq!(limit_of("0::/\n", "", files(&[("/sys/fs/cgroup/memory.max", "max\n")])), None);
        assert_eq!(limit_of("0::/ci/job\n", "", files(&[("/sys/fs/cgroup/ci/job/memory.max", "2147483648"), ("/sys/fs/cgroup/memory.max", "max")])), Some(2 * GIB));
        assert_eq!(limit_of("0::/ci/job\n", "", files(&[("/sys/fs/cgroup/memory.max", "3221225472")])), Some(3 * GIB));
        assert_eq!(limit_of("0::/ci/job\n", "", files(&[("/sys/fs/cgroup/ci/job/memory.max", "max"), ("/sys/fs/cgroup/ci/memory.max", "2147483648")])), Some(2 * GIB));
        assert_eq!(limit_of("0::/ci/job/\n", "", files(&[("/sys/fs/cgroup/ci/job/memory.max", "4294967296"), ("/sys/fs/cgroup/ci/memory.max", "max")])), Some(4 * GIB));
        let v1 = "12:cpu,cpuacct:/docker/x\n9:memory:/docker/x\n";
        assert_eq!(limit_of(v1, "", files(&[("/sys/fs/cgroup/memory/docker/x/memory.limit_in_bytes", "1073741824")])), Some(GIB));
        assert_eq!(limit_of(v1, "", files(&[("/sys/fs/cgroup/memory/memory.limit_in_bytes", "9223372036854771712")])), Some(9223372036854771712));
        assert_eq!(limit_of(v1, "", files(&[("/sys/fs/cgroup/memory/docker/memory.limit_in_bytes", "2147483648"), ("/sys/fs/cgroup/memory/docker/x/memory.limit_in_bytes", "9223372036854771712")])), Some(2 * GIB));
        assert_eq!(limit_of("12:cpu:/x\n", "", files(&[])), None);
    }

    #[test]
    fn the_cgroup_mounts() {
        let files = |entries: &'static [(&'static str, &'static str)]| move |p: &str| entries.iter().find(|(f, _)| *f == p).map(|(_, t)| t.to_string());
        let v1_elsewhere = "30 25 0:27 / /sys/fs/cgroup/rg1 rw,nosuid shared:13 - cgroup cgroup rw,cpuset,memory\n";
        assert_eq!(limit_of("9:cpuset,memory:/ci/job\n", v1_elsewhere, files(&[("/sys/fs/cgroup/rg1/ci/job/memory.limit_in_bytes", "2147483648")])), Some(2 * GIB));
        let v1_combined = "31 25 0:28 / /sys/fs/cgroup/cpu,memory rw shared:14 - cgroup cgroup rw,cpu,memory\n";
        assert_eq!(limit_of("9:cpu,memory:/ci/job\n", v1_combined, files(&[("/sys/fs/cgroup/cpu,memory/ci/job/memory.limit_in_bytes", "2147483648")])), Some(2 * GIB));
        let v2_relocated = "29 25 0:26 / /sys/fs/cgroup/unified rw,nosuid shared:4 - cgroup2 cgroup2 rw,nsdelegate\n";
        assert_eq!(limit_of("0::/ci/job\n", v2_relocated, files(&[("/sys/fs/cgroup/unified/ci/job/memory.max", "2147483648")])), Some(2 * GIB));
        let v2_subtree = "29 25 0:26 /docker/abc /sys/fs/cgroup ro,nosuid - cgroup2 cgroup2 rw\n";
        assert_eq!(limit_of("0::/docker/abc/job\n", v2_subtree, files(&[("/sys/fs/cgroup/job/memory.max", "max"), ("/sys/fs/cgroup/memory.max", "2147483648")])), Some(2 * GIB));
        assert_eq!(limit_of("0::/docker/abc/job\n", v2_subtree, files(&[("/sys/fs/cgroup/job/memory.max", "max"), ("/sys/fs/cgroup/memory.max", "max")])), None);
        assert_eq!(limit_of("0::/elsewhere/job\n", v2_subtree, files(&[("/sys/fs/cgroup/job/memory.max", "2147483648")])), None);
        let nested = "29 25 0:26 / /sys/fs/cgroup rw - cgroup2 cgroup2 rw\n";
        assert_eq!(limit_of("0::/a/b/c\n", nested, files(&[("/sys/fs/cgroup/a/b/c/memory.max", "max"), ("/sys/fs/cgroup/a/b/memory.max", "4294967296"), ("/sys/fs/cgroup/a/memory.max", "3221225472")])), Some(3 * GIB));
        let spaced = "29 25 0:26 / /mnt/cgroup\\040two rw - cgroup2 cgroup2 rw\n";
        assert_eq!(limit_of("0::/job\n", spaced, files(&[("/mnt/cgroup two/job/memory.max", "2147483648")])), Some(2 * GIB));
        let other_kinds = "22 1 8:1 / / rw - ext4 /dev/sda1 rw\n31 25 0:28 / /sys/fs/cgroup/cpu rw - cgroup cgroup rw,cpu\n";
        assert_eq!(limit_of("0::/ci\n", other_kinds, files(&[("/sys/fs/cgroup/ci/memory.max", "2147483648")])), Some(2 * GIB));
    }
}
