// Each backend of flate2 against the JDK's answers (bench/survey/oracle.scala's deflate lines); C zlib's by the
// libz-sys the `zlib-c` feature enables with `static`, the bundled sources, its version printed.
use std::io::Write;
fn main() {
    let oracle = std::fs::read_to_string(std::env::args().nth(1).expect("oracle.log")).unwrap();
    let inputs: Vec<(&str, Vec<u8>)> = vec![
        ("short", b"hello hello hello hello".to_vec()),
        ("repeated", b"abcde".repeat(1000)),
        ("ramp", (0..8192u32).map(|i| (i % 251) as u8).collect()),
        ("zeros", vec![0; 10000]),
    ];
    let (mut same, mut differ) = (0, 0);
    for line in oracle.lines().filter(|l| l.starts_with("deflate.") && !l.contains(".bytes=")) {
        let (key, value) = line.split_once('=').unwrap();
        let parts: Vec<&str> = key.split('.').collect();
        let (name, level, wrap) = (parts[1], parts[2].parse::<u32>().unwrap(), parts[3] == "zlib");
        let input = &inputs.iter().find(|(n, _)| *n == name).unwrap().1;
        let out = if wrap {
            let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(level));
            e.write_all(input).unwrap();
            e.finish().unwrap()
        } else {
            let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(level));
            e.write_all(input).unwrap();
            e.finish().unwrap()
        };
        let mut h = crc32fast::Hasher::new();
        h.update(&out);
        let got = format!("{} {:08x}", out.len(), h.finalize());
        if got == value { same += 1 } else { differ += 1; println!("differs {key}: jdk {value}, backend {got}") }
    }
    println!("backend agrees on {same} of {} oracle outputs", same + differ);
    // The zlib linked, by its own answer: libz-sys's bundled sources when `static` holds, a system library otherwise.
    #[cfg(feature = "zlib-c")]
    println!("zlibVersion() = {}", unsafe { std::ffi::CStr::from_ptr(libz_sys::zlibVersion()) }.to_string_lossy());
}
