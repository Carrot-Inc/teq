//! The build of an executable from a link's LLVM IR and the C code its class path carries, after
//! `build.NativeLib`, `build.Descriptor` and `build.LLVM` of Scala Native 0.5.12, with zig as the
//! C compiler and the linker. The executable is a static one for x86_64 Linux on musl.

use super::classpath;
use super::codegen::Module;
use super::descriptor;
use super::jar::{self, CODE_DIR, SOURCE_EXTENSIONS};
use super::library_options;
use super::reach::Triple;
use super::tools;
use crate::zip::Zip;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

type Res<T> = Result<T, String>;

pub struct Options {
    pub zig: PathBuf,
    pub zig_cache: PathBuf,
    /// zig's name of the target: `x86_64-linux-musl`.
    pub target: String,
    pub work: PathBuf,
    pub output: PathBuf,
    pub lib_dirs: Vec<PathBuf>,
    /// How the build's caller names one of `lib_dirs` (`--lib-dir=<dir>`): what a refusal of a
    /// library tells the user to give.
    pub lib_dir_option: String,
    pub workers: usize,
}

/// zig's name of the target every executable is built for.
const TARGET: &str = "x86_64-linux-musl";

/// zig's name of a link's target (`x86_64-unknown-linux-musl` is `x86_64-linux-musl`), refused
/// unless it is the one executables are built for.
pub fn target(triple: &Triple) -> Res<String> {
    let target = format!("{}-{}-{}", triple.arch, triple.os, triple.env);
    if target != TARGET {
        return Err(format!("not supported on native: an executable for {}: teq builds static {} executables", target, TARGET));
    }
    Ok(target)
}

/// What of a link's result the C code and the link line depend on.
pub struct Program<'a> {
    pub modules: &'a [Module],
    pub links: Vec<String>,
    pub defines: Vec<String>,
    pub multithreading: bool,
    pub link_cpp_runtime: bool,
    pub gc: &'a str,
}

const DESCRIPTOR: &str = "scala-native.properties";
const CPP: &str = ".cpp";
/// The variables of the caller's environment zig is given: what moves an output on purpose (`__DATE__`, `__TIME__`
/// and `__TIMESTAMP__`, clang/lib/Driver/ToolChains/Clang.cpp). Of the rest none is passed: clang's driver and zig
/// read CPATH, C_INCLUDE_PATH, CPLUS_INCLUDE_PATH, CCC_OVERRIDE_OPTIONS, ZIG_LIB_DIR, ZIG_LIBC and others, which would
/// change a compilation that its command does not name.
const PASSED_ENVIRONMENT: [&str; 1] = ["SOURCE_DATE_EPOCH"];
/// The companion of a fully static musl executable; see the file.
const STATIC_MUSL: &str = include_str!("static_musl.c");
/// The size musl gives a thread's stack by default is the executable's `PT_GNU_STACK` size up to
/// 8 MiB (musl's `__default_stacksize` and its cap), glibc's default; without it, 128 KiB.
const MUSL_THREAD_STACK: u32 = 8 << 20;
/// The archives a static archive's code calls and its bindings do not name.
const ARCHIVE_NEEDS: [(&str, &[&str]); 1] = [("s2n", &["crypto"])];
/// The libcrypto functions `static_musl.c` wraps for the system's trust roots, wrapped where an archive that verifies
/// peers through libcrypto is linked.
const TRUST_ROOT_WRAPS: [(&str, &[&str]); 1] = [("s2n", &["X509_STORE_set_default_paths", "X509_verify_cert"])];
/// The libraries zig links from what it carries, whatever the target's system holds: its C
/// library's (zig's `isLibCLibName` for musl), libc++ and libunwind, compiler-rt.
const ZIG_LIBRARIES: [&str; 19] = ["c", "m", "rt", "pthread", "crypt", "util", "xnet", "resolv", "dl", "c++", "c++abi", "stdc++", "supc++", "unwind", "atomic", "gcc", "gcc_s", "gcc_eh", "compiler_rt"];

struct NativeLib {
    src: PathBuf,
    dest: PathBuf,
    /// Of a jar, what unpacking it comes to (`jar::read_jar`); none of a directory.
    unpacked: Option<jar::Unpacking>,
}

struct Descriptor {
    gc_project: bool,
    links: Vec<String>,
    defines: Vec<String>,
    includes: Vec<PathBuf>,
    c_options: Vec<String>,
    cpp_options: Vec<String>,
}

struct Job {
    source: PathBuf,
    object: PathBuf,
    args: Vec<String>,
    /// The library a C file is of: its class path entry and its compile options, as zig is given them.
    given: Option<(PathBuf, String)>,
}

/// Whether zig's cache stands for a job's compilation: an LLVM IR module includes nothing, and zig keys its object by
/// the module's bytes, the command and zig's version (zig 0.16.0's src/Compilation.zig: `hashCSource` and line 2122).
/// Of a C, C++ or assembly source it does not: its manifest holds the files the compilation read and none of the
/// places looked in before them, so a header added to an earlier include directory, or a link there that comes to
/// lead to a file, is not seen; and what an assembler reads (`.include`, `.incbin`) is in no list.
fn through_zig_cache(source: &Path) -> bool {
    text(source).ends_with(".ll")
}

fn io<T>(path: &Path, result: std::io::Result<T>) -> Res<T> {
    result.map_err(|e| format!("{}: {}", path.display(), e))
}

fn is_source(name: &str) -> bool {
    SOURCE_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// `NativeLib.findNativeLibs`: the entries of the class path that carry C code, each with the directory it is
/// unpacked into, named by the entry and its place among them, and of a jar what unpacking comes to (`jar::read_jar`,
/// which judges every jar of the class path, against the central directory teq's class path read took of it). An entry
/// the link would refuse is refused here too (`classpath::entry`).
fn native_libs(class_path: &[String], work: &Path) -> Res<Vec<NativeLib>> {
    let limits = jar::limits(work)?;
    let mut libs = Vec::new();
    for entry in class_path {
        let src = PathBuf::from(entry);
        let kind = classpath::entry(entry).map_err(|why| format!("the class path: {}", why))?;
        let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let name = name.strip_suffix(".jar").unwrap_or(&name);
        let dest = work.join("dependencies").join(format!("{}-{}", name, libs.len()));
        let unpacked = if matches!(kind, classpath::Entry::Jar) {
            let linked = Zip::open(entry)?.directory_place();
            let file = jar::JarFile::open(&src)?;
            match jar::read_jar(&file, Some(linked), &dest, limits).map_err(|why| format!("{}: {}", entry, why))? {
                Some(unpacking) => {
                    // Every entry the JDK's copy reads, read now as it reads it: a jar whose data cannot be read is
                    // refused before anything is unpacked.
                    for e in &unpacking.read {
                        jar::copy_entry(&file, unpacking.locpos, e).map_err(|why| format!("{}: {}", entry, why))?;
                    }
                    Some(unpacking)
                }
                None => continue,
            }
        } else if src.join(CODE_DIR).is_dir() && classpath::files(&src.join(CODE_DIR))?.iter().any(|(name, _)| is_source(name)) {
            None
        } else {
            continue;
        };
        libs.push(NativeLib { src, dest, unpacked });
    }
    if libs.is_empty() {
        return Err(format!("no Scala Native libraries were found on the class path: {}", class_path.join(":")));
    }
    Ok(libs)
}

/// The bytes of a library's descriptor, read from its class path entry, where it has one.
fn descriptor_bytes(lib: &NativeLib) -> Res<Option<Vec<u8>>> {
    let name = format!("{}/{}", CODE_DIR, DESCRIPTOR);
    match &lib.unpacked {
        Some(unpacking) => match unpacking.code.iter().find(|(file, _)| *file == name) {
            Some((_, e)) => jar::copy_entry(&jar::JarFile::open(&lib.src)?, unpacking.locpos, e).map(Some).map_err(|why| format!("{}: {}", text(&lib.src), why)),
            None => Ok(None),
        },
        None => {
            let path = lib.src.join(&name);
            path.is_file().then(|| io(&path, std::fs::read(&path))).transpose()
        }
    }
}

/// The work directory held for the length of a build (`<work>/lock`, locked before anything of it is read or written,
/// released when the build ends), as two builds into it would write, and link, the same objects: a second build into
/// it says so once and waits for the first. Every compiler the build starts holds
/// the lock's descriptor too, not closed when it starts (`inherited`): a build killed while its compilers still run
/// leaves the directory locked until the last of them has ended, so that no later build meets an object one of them
/// is writing. (zig 0.16.0's `cc` leaves no process of its own behind once a compilation or a link has ended.)
fn locked(work: &Path) -> Res<std::fs::File> {
    let path = work.join("lock");
    let file = io(&path, std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(&path))?;
    match file.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => {
            eprintln!("{}: another native build holds this work directory; waiting for it", work.display());
            io(&path, file.lock())?;
        }
        // A file system that holds no lock: the build stops here, as without the lock it could link another build's
        // objects.
        Err(std::fs::TryLockError::Error(e)) => return Err(format!("{}: {}", path.display(), e)),
    }
    io(&path, inherited(&file))?;
    Ok(file)
}

#[cfg(unix)]
fn inherited(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::io::AsRawFd;
    extern "C" {
        fn fcntl(fd: i32, cmd: i32, ...) -> i32;
    }
    // F_SETFD with no flag: the descriptor stays open across exec (POSIX's FD_CLOEXEC cleared).
    const F_SETFD: i32 = 2;
    if unsafe { fcntl(file.as_raw_fd(), F_SETFD, 0) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(unix))]
fn inherited(_: &std::fs::File) -> std::io::Result<()> {
    Ok(())
}

/// The C code of an entry in its directory, unpacked again only when the entry changed: of a jar, the files
/// `jar::read_jar` gives, its stamp the jar's digest (as Scala Native's `jarhash`) and their names.
fn unpack(lib: &NativeLib) -> Res<()> {
    let stamp_path = lib.dest.join("stamp");
    if let Some(unpacking) = &lib.unpacked {
        let mut stamp = Zip::open(&text(&lib.src))?.sha1()?;
        for (name, _) in &unpacking.code {
            stamp.push('\n');
            stamp.push_str(name);
        }
        if std::fs::read_to_string(&stamp_path).is_ok_and(|s| s == stamp) {
            return Ok(());
        }
        if lib.dest.exists() {
            io(&lib.dest, std::fs::remove_dir_all(&lib.dest))?;
        }
        let file = jar::JarFile::open(&lib.src)?;
        for (name, e) in &unpacking.code {
            let to = lib.dest.join(name);
            if let Some(parent) = to.parent() {
                io(parent, std::fs::create_dir_all(parent))?;
            }
            let data = jar::copy_entry(&file, unpacking.locpos, e).map_err(|why| format!("{}: {}", text(&lib.src), why))?;
            io(&to, std::fs::write(&to, data))?;
        }
        io(&lib.dest, std::fs::create_dir_all(&lib.dest))?;
        io(&stamp_path, std::fs::write(&stamp_path, stamp))
    } else {
        let mut read = Vec::new();
        for (name, file) in classpath::files(&lib.src.join(CODE_DIR))? {
            let bytes = io(&file, std::fs::read(&file))?;
            read.push((name, bytes));
        }
        let stamp = crate::products::digest(read.iter().map(|(name, bytes)| (name.as_str(), bytes.as_slice())));
        if std::fs::read_to_string(&stamp_path).is_ok_and(|s| s == stamp) {
            return Ok(());
        }
        if lib.dest.exists() {
            io(&lib.dest, std::fs::remove_dir_all(&lib.dest))?;
        }
        for (name, bytes) in &read {
            let to = lib.dest.join(CODE_DIR).join(name);
            if let Some(parent) = to.parent() {
                io(parent, std::fs::create_dir_all(parent))?;
            }
            io(&to, std::fs::write(&to, bytes))?;
        }
        io(&lib.dest, std::fs::create_dir_all(&lib.dest))?;
        io(&stamp_path, std::fs::write(&stamp_path, stamp))
    }
}

/// A refusal of a library's descriptor, naming the library by its class path entry.
fn refused(entry: &Path, what: &str) -> String {
    format!("{}: {}: not supported on native: {}", entry.display(), DESCRIPTOR, what)
}

/// The string of a property's piece as zig is to be given it: one a process can be given.
fn argument(piece: &[u16], property: &str, entry: &Path) -> Res<String> {
    let given = String::from_utf16(piece).map_err(|_| refused(entry, &format!("{} with a UTF-16 surrogate that pairs with nothing", property)))?;
    if given.contains('\0') {
        return Err(refused(entry, &format!("{} with a NUL character", property)));
    }
    Ok(given)
}

/// `Descriptor.load` (src/native/descriptor.rs) and `NativeLib.resolveDescriptorFlags`: what a library's descriptor
/// adds to the compilation of its C code under `code`, its compile options those the build takes
/// (`library_options`); `entry` names the library in a refusal or a failure.
fn descriptor(bytes: &[u8], entry: &Path, code: &Path) -> Res<Descriptor> {
    let failed = |why: String| format!("{}: {}: {}", entry.display(), DESCRIPTOR, why);
    let properties = descriptor::load(&descriptor::decode_utf8(bytes)).map_err(failed)?;
    let read = descriptor::read(&properties);
    let gc_project = read.gc_project.map_err(|value| failed(format!("project.gcProject is {}, which is neither true nor false", String::from_utf16_lossy(&value))))?;
    let strings = |pieces: &[descriptor::JString], property: &str| pieces.iter().map(|piece| argument(piece, property, entry)).collect::<Res<Vec<_>>>();
    // A link name that is no string is no program's.
    let links = read.links.iter().filter_map(|name| String::from_utf16(name).ok()).collect();
    let defines = strings(&read.defines, "preprocessor.defines")?;
    if defines.iter().any(String::is_empty) {
        return Err(refused(entry, "an empty define in preprocessor.defines"));
    }
    let code_dir: descriptor::JString = text(code).encode_utf16().collect();
    let includes = read.includes.iter().map(|path| {
        let resolved = descriptor::include_path(path, &code_dir).ok_or_else(|| refused(entry, "compile.include.paths with a NUL character"))?;
        argument(&resolved, "compile.include.paths", entry).map(PathBuf::from)
    }).collect::<Res<Vec<_>>>()?;
    let options = |pieces: &[descriptor::JString], property: &str| -> Res<Vec<String>> {
        library_options::taken(&strings(pieces, property)?).map_err(|option| refused(entry, &library_options::refusal(&option)))
    };
    Ok(Descriptor {
        gc_project,
        links,
        defines,
        includes,
        c_options: options(&read.c_options, "compile.c.options")?,
        cpp_options: options(&read.cpp_options, "compile.cpp.options")?,
    })
}

fn is_std_flag(option: &str) -> bool {
    option.starts_with("-std=") || option.starts_with("--std=")
}

/// `LLVM.compileFile`'s flags, for the file kinds Linux builds have and the modes teq builds in:
/// no optimizer, Scala Native's own exceptions.
fn compile_args(options: &Options, program: &Program, source: &Path, object: &Path, descriptor: Option<&Descriptor>, lib_options: &[String]) -> Vec<String> {
    let name = text(source);
    let is_cpp = name.ends_with(CPP);
    let is_ll = name.ends_with(".ll");
    let mut args = vec![if is_cpp { "c++" } else { "cc" }.to_string(), "-c".into(), name, "-o".into(), text(object), "-target".into(), options.target.clone()];
    let exceptions: &[&str] = match (program.link_cpp_runtime, is_cpp) {
        (true, _) => &["-fexceptions", "-funwind-tables"],
        (false, true) => &["-fno-rtti", "-fno-exceptions", "-funwind-tables"],
        (false, false) => &[],
    };
    if !is_ll {
        let (given, default) = match (descriptor, is_cpp) {
            (Some(d), true) => (&d.cpp_options[..], "-std=c++11"),
            (Some(d), false) => (&d.c_options[..], "-std=c11"),
            (None, true) => (&[][..], "-std=c++11"),
            (None, false) => (&[][..], "-std=c11"),
        };
        if !given.iter().any(|o| is_std_flag(o)) {
            args.push(default.into());
        }
        args.extend(given.iter().cloned());
        args.extend(exceptions.iter().map(|s| s.to_string()));
    }
    if program.multithreading {
        args.push("-DSCALANATIVE_MULTITHREADING_ENABLED".into());
    }
    args.extend(["-Wno-override-module", "-fvisibility=hidden", "-O0", "-fomit-frame-pointer"].map(String::from));
    // zig's addition to a compilation with no optimizer, and its runtime in the link.
    args.push("-fno-sanitize=undefined".into());
    args.extend(lib_options.iter().cloned());
    args
}

/// `NativeLib.configureNativeLibrary`: what the link's result and the library's descriptor add.
fn lib_options(program: &Program, descriptor: &Descriptor) -> Vec<String> {
    let mut out: Vec<String> = program.defines.iter().map(|d| format!("-D{}", d)).collect();
    out.extend(descriptor.links.iter().filter(|name| program.links.contains(name)).map(|name| format!("-DSCALANATIVE_LINK_{}", name.to_uppercase())));
    out.extend(descriptor.defines.iter().map(|d| format!("-D{}", d)));
    if descriptor.gc_project {
        out.push(format!("-DSCALANATIVE_GC_{}", program.gc.to_uppercase()));
    }
    out.extend(descriptor.includes.iter().map(|dir| format!("-I{}", text(dir))));
    out
}

/// The environment zig runs in: its caches, and the variables of `PASSED_ENVIRONMENT` the caller's has.
fn environment(options: &Options) -> Vec<(String, std::ffi::OsString)> {
    let mut vars: Vec<(String, std::ffi::OsString)> = PASSED_ENVIRONMENT.iter().filter_map(|name| Some((name.to_string(), std::env::var_os(name)?))).collect();
    vars.extend(["ZIG_GLOBAL_CACHE_DIR", "ZIG_LOCAL_CACHE_DIR"].map(|name| (name.to_string(), options.zig_cache.clone().into_os_string())));
    vars
}

fn run(zig: &Path, options: &Options, args: &[String], what: &Path) -> Res<()> {
    let mut command = Command::new(zig);
    command.env_clear().envs(environment(options));
    let output = command.args(args).current_dir(&options.work).output().map_err(|e| format!("{}: {}", zig.display(), e))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!("{}: zig {} failed\n{}", what.display(), args[0], String::from_utf8_lossy(&output.stderr).trim_end()))
}

// The starts of LLVM bitcode, bare and in its wrapper, and the fields of an ELF header the link's input is told by.
const BITCODE: &[u8] = b"BC\xc0\xde";
const BITCODE_WRAPPER: &[u8] = b"\xde\xc0\x17\x0b";
const ELF64_LITTLE: &[u8] = b"\x7fELF\x02\x01";
const E_TYPE: usize = 16;
const E_MACHINE: usize = 18;
const ET_REL: u16 = 1;

/// Whether a compilation's output is what the static link takes: an ELF relocatable object of the
/// target's machine (EM_X86_64, EM_AARCH64), or LLVM bitcode, which zig's link compiles itself.
fn is_object(path: &Path, target: &str) -> bool {
    let mut head = Vec::new();
    let read = std::fs::File::open(path).and_then(|file| std::io::Read::read_to_end(&mut std::io::Read::take(file, 64), &mut head));
    if read.is_err() {
        return false;
    }
    if head.starts_with(BITCODE) || head.starts_with(BITCODE_WRAPPER) {
        return true;
    }
    let field = |at: usize| head.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let machine = match target.split('-').next() {
        Some("x86_64") => 62,
        Some("aarch64") => 183,
        _ => return false,
    };
    head.starts_with(ELF64_LITTLE) && field(E_TYPE) == Some(ET_REL) && field(E_MACHINE) == Some(machine)
}

/// The dependency file a C, C++ or assembly source's compilation is given (`compile_all`).
fn dependency_path(object: &Path) -> PathBuf {
    PathBuf::from(format!("{}.d", text(object)))
}

/// Every job compiled, its object written again. An LLVM IR module through zig's cache (`through_zig_cache`); a C,
/// C++ or assembly source given a dependency file of its own, with which zig 0.16.0 looks no object up in its cache:
/// it takes `-MD` as the caller's dependency file (src/clang_options.zon, `MD`: `.dep_file`; src/main.zig 2427-2429,
/// `disable_c_depfile`) and then compiles (src/Compilation.zig 5682), as "the user is presumably doing their own
/// caching by using dep file flags" (5721-5725). The build does none: what such a source reads is not all in any list.
fn compile_all(zig: &Path, options: &Options, jobs: &[Job]) -> Res<()> {
    let runs: Vec<Job> = jobs
        .iter()
        .map(|job| {
            let _ = std::fs::remove_file(&job.object);
            let mut args = job.args.clone();
            if !through_zig_cache(&job.source) {
                args.extend(["-MD".to_string(), "-MF".to_string(), text(&dependency_path(&job.object))]);
            }
            Job { source: job.source.clone(), object: job.object.clone(), args, given: None }
        })
        .collect();
    let compiled = compile(zig, options, &runs);
    for job in jobs {
        let _ = std::fs::remove_file(dependency_path(&job.object));
    }
    compiled?;
    // Whatever the options were, the compilation has to have written an object the link takes.
    for job in jobs {
        if !is_object(&job.object, &options.target) {
            let _ = std::fs::remove_file(&job.object);
            return Err(match &job.given {
                Some((entry, given)) => format!(
                    "{}: {}: not supported on native: compile options with which the compiler writes no object for {} ({})",
                    entry.display(),
                    DESCRIPTOR,
                    job.source.display(),
                    given
                ),
                None => format!("{}: zig wrote no object of it", job.source.display()),
            });
        }
    }
    Ok(())
}

fn compile(zig: &Path, options: &Options, jobs: &[Job]) -> Res<()> {
    let next = AtomicUsize::new(0);
    let failure: Mutex<Option<String>> = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..options.workers.clamp(1, jobs.len().max(1)) {
            crate::alloc::spawn_in(scope, || loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(job) = jobs.get(index) else { break };
                if failure.lock().unwrap().is_some() {
                    break;
                }
                if let Err(e) = run(zig, options, &job.args, &job.source) {
                    failure.lock().unwrap().get_or_insert(e);
                }
            });
        }
    });
    match failure.into_inner().unwrap() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// The libraries a link names: `pthread`, `dl` and `m`, those the program reached and what their
/// archives need.
fn link_names<'a>(program: &'a Program) -> Vec<&'a str> {
    let mut names: Vec<&str> = vec!["pthread", "dl", "m"];
    for name in &program.links {
        let needed = ARCHIVE_NEEDS.iter().filter(|(archive, _)| archive == name).flat_map(|(_, needed)| needed.iter().copied());
        for name in std::iter::once(name.as_str()).chain(needed) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// The libcrypto functions a link wraps for the system's trust roots (`static_musl.c`): those of
/// an archive among the link's names that verifies its peers through libcrypto.
fn trust_root_wraps(names: &[&str]) -> Vec<&'static str> {
    TRUST_ROOT_WRAPS.iter().filter(|(archive, _)| names.contains(archive)).flat_map(|(_, symbols)| symbols.iter().copied()).collect()
}

/// teq's own source of a library compiled in the work directory, where its headers are then.
fn own_object(zig: &Path, options: &Options, name: &str, own: &tools::Own) -> Res<(PathBuf, PathBuf)> {
    let dir = options.work.join("own").join(name);
    io(&dir, std::fs::create_dir_all(&dir))?;
    for (file, contents) in own.files {
        let path = dir.join(file);
        if !std::fs::read(&path).is_ok_and(|old| old == contents.as_bytes()) {
            io(&path, std::fs::write(&path, contents))?;
        }
    }
    let source = dir.join(own.compiled);
    let object = dir.join(format!("{}.o", own.compiled));
    let mut args = vec!["cc".to_string(), "-target".into(), options.target.clone()];
    args.extend(own.flags.iter().map(|flag| flag.to_string()));
    args.extend(["-c".to_string(), text(&source), "-o".into(), text(&object)]);
    compile_all(zig, options, &[Job { source, object: object.clone(), args, given: None }])?;
    Ok((object, dir))
}

/// The executable at `options.output`: the class path's C code and the program's modules compiled (`compile_all`)
/// and linked statically with what the program links.
pub fn build(class_path: &[String], program: &Program, options: &Options) -> Res<()> {
    let zig = &options.zig;
    io(&options.work, std::fs::create_dir_all(&options.work))?;
    let _held = locked(&options.work)?;
    // Every jar's entries and every library's descriptor are read and judged before anything is unpacked or compiled: a
    // refusal leaves the work directory as it was.
    let libs = native_libs(class_path, &options.work)?;
    let described = libs
        .iter()
        .map(|lib| descriptor_bytes(lib)?.map(|bytes| descriptor(&bytes, &lib.src, &lib.dest.join(CODE_DIR))).transpose())
        .collect::<Res<Vec<Option<Descriptor>>>>()?;
    let names = link_names(program);
    // The libraries zig does not carry are found or compiled before the class path's C code,
    // which includes their headers: the directories the build names first.
    let mut includes = tools::includes(&options.lib_dirs);
    let mut libraries = Vec::new();
    for name in names.iter().filter(|name| !ZIG_LIBRARIES.contains(name)) {
        match tools::library(name, &options.lib_dirs, &options.target, &options.lib_dir_option)? {
            tools::Library::Archive(archive) => libraries.push(archive),
            tools::Library::Own(own) => {
                let (object, dir) = own_object(zig, options, name, own)?;
                includes.push(dir);
                libraries.push(object);
            }
        }
    }
    let includes: Vec<String> = includes.iter().map(|dir| format!("-I{}", text(dir))).collect();
    let mut library_objects = Vec::new();
    for (lib, described) in libs.iter().zip(described) {
        unpack(lib)?;
        let code = lib.dest.join(CODE_DIR);
        let mut lib_options = match &described {
            Some(d) => lib_options(program, d),
            None => program.defines.iter().map(|d| format!("-D{}", d)).collect(),
        };
        lib_options.extend(includes.iter().cloned());
        // In the names' order. Scala Native takes the order the system lists its fresh copy in (IO.getAll's
        // `Files.walkFileTree`), which on a hashing file system differs from one copy to the next.
        let jobs: Vec<Job> = classpath::files(&code)?
            .into_iter()
            .filter(|(name, _)| is_source(name))
            .map(|(_, source)| {
                let object = PathBuf::from(format!("{}.o", text(&source)));
                let given = described.as_ref().map_or_else(String::new, |d| if text(&source).ends_with(CPP) { &d.cpp_options } else { &d.c_options }.join(","));
                let args = compile_args(options, program, &source, &object, described.as_ref(), &lib_options);
                Job { args, source, object, given: Some((lib.src.clone(), given)) }
            })
            .collect();
        library_objects.extend(jobs.iter().map(|job| job.object.clone()));
        compile_all(zig, options, &jobs)?;
    }

    let generated = options.work.join("generated");
    io(&generated, std::fs::create_dir_all(&generated))?;
    let mut jobs = Vec::new();
    for module in program.modules {
        let source = generated.join(format!("{}.ll", module.name));
        if !std::fs::read(&source).is_ok_and(|old| old == module.text) {
            io(&source, std::fs::write(&source, &module.text))?;
        }
        let object = PathBuf::from(format!("{}.o", text(&source)));
        jobs.push(Job { args: compile_args(options, program, &source, &object, None, &[]), source, object, given: None });
    }
    compile_all(zig, options, &jobs)?;
    // Linked as Scala Native links them: the program's modules, then the libraries' objects in the class path's order
    // (tools/.../build/Build.scala `compile`, the generated IR's objects before the libraries'; LLVM.scala
    // `prepareLinkCommand`, the objects in that order).
    let mut objects: Vec<PathBuf> = jobs.iter().map(|job| job.object.clone()).collect();
    objects.extend(library_objects);

    let musl = options.work.join("static");
    io(&musl, std::fs::create_dir_all(&musl))?;
    let source = musl.join("static_musl.c");
    let object = musl.join("static_musl.c.o");
    if !std::fs::read(&source).is_ok_and(|old| old == STATIC_MUSL.as_bytes()) {
        io(&source, std::fs::write(&source, STATIC_MUSL))?;
    }
    let args = ["cc", "-c", &text(&source), "-o", &text(&object), "-target", &options.target, "-O2", "-funwind-tables", "-fno-sanitize=undefined"].map(String::from).to_vec();
    compile_all(zig, options, &[Job { source, object: object.clone(), args, given: None }])?;
    let mut link = vec![if program.link_cpp_runtime { "c++" } else { "cc" }.to_string(), "-target".into(), options.target.clone(), "-static".into()];
    link.extend(["-o".to_string(), text(&options.output)]);
    link.extend(objects.iter().map(|o| text(o)));
    link.extend([text(&object), "-Wl,-wrap=pthread_create".into(), "-Wl,-wrap=main".into(), format!("-Wl,-z,stack-size={}", MUSL_THREAD_STACK)]);
    link.push("-Wl,--as-needed".into());
    link.extend(trust_root_wraps(&names).iter().map(|symbol| format!("-Wl,-wrap={}", symbol)));
    // Each library zig does not carry is the file found above.
    link.extend(libraries.iter().map(|file| text(file)));
    link.extend(names.iter().filter(|name| ZIG_LIBRARIES.contains(name)).map(|name| format!("-l{}", name)));
    run(zig, options, &link, &options.output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the link takes of a compilation's output, and planted outputs it does not.
    #[test]
    fn takes_an_object_of_the_target_or_bitcode_alone() {
        let dir = std::env::temp_dir().join(format!("teq-is-object-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let elf = |kind: u16, machine: u16| {
            let mut b = b"\x7fELF\x02\x01".to_vec();
            b.resize(E_TYPE, 0);
            b.extend(kind.to_le_bytes().into_iter().chain(machine.to_le_bytes()));
            b.resize(64, 0);
            b
        };
        let cases: [(&str, Vec<u8>, bool); 8] = [
            ("x86_64 object", elf(1, 62), true),
            ("bitcode", b"BC\xc0\xde\x35\x14".to_vec(), true),
            ("bitcode in its wrapper", b"\xde\xc0\x17\x0b\x00\x00".to_vec(), true),
            ("aarch64 object", elf(1, 183), false),
            ("executable", elf(2, 62), false),
            ("preprocessed text", b"# 1 \"a.c\"\nint x;\n".to_vec(), false),
            ("empty", Vec::new(), false),
            ("cut short", elf(1, 62).into_iter().take(E_TYPE + 1).collect(), false),
        ];
        for (what, bytes, taken) in cases {
            let path = dir.join(what.replace(' ', "-"));
            std::fs::write(&path, bytes).unwrap();
            assert_eq!(is_object(&path, "x86_64-linux-musl"), taken, "{what}");
        }
        assert!(is_object(&dir.join("aarch64-object"), "aarch64-linux-musl"));
        assert!(!is_object(&dir.join("absent"), "x86_64-linux-musl"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refuses_an_entry_the_link_refuses() {
        let work = std::env::temp_dir().join(format!("teq-native-libs-{}", std::process::id()));
        let absent = work.join("missing.jar").to_string_lossy().into_owned();
        let refused = native_libs(std::slice::from_ref(&absent), &work).err();
        assert_eq!(refused, Some(format!("the class path: {}: no such jar", absent)));
    }

    fn linking(links: &[&str]) -> Program<'static> {
        Program { modules: &[], links: links.iter().map(|name| name.to_string()).collect(), defines: Vec::new(), multithreading: false, link_cpp_runtime: false, gc: "immix" }
    }

    #[test]
    fn links_what_an_archive_needs_after_it() {
        assert_eq!(link_names(&linking(&[])), ["pthread", "dl", "m"]);
        assert_eq!(link_names(&linking(&["m", "idn2"])), ["pthread", "dl", "m", "idn2"]);
        assert_eq!(link_names(&linking(&["s2n", "idn2"])), ["pthread", "dl", "m", "s2n", "crypto", "idn2"]);
        assert_eq!(link_names(&linking(&["crypto", "s2n"])), ["pthread", "dl", "m", "crypto", "s2n"]);
    }

    #[test]
    fn wraps_the_trust_roots_where_s2n_is_linked_alone() {
        assert!(trust_root_wraps(&link_names(&linking(&["crypto"]))).is_empty());
        assert_eq!(trust_root_wraps(&link_names(&linking(&["s2n"]))), ["X509_STORE_set_default_paths", "X509_verify_cert"]);
    }
}
