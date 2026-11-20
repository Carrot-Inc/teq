//> using file ../tools/script
// The static archives of the C libraries a native build links and zig's C library does not carry, as teq's releases
// publish them under the tag native-libs-<n>: a zip per library and target holding
// lib/lib<name>.a, built with no debug information (-g0), the public headers a binding's C code includes under
// include/ and the library's licence texts under licenses/, nothing else. Each library is built from its
// source tarball, pinned below by its URL and SHA-256 and verified before it is unpacked, by its own build (zlib's
// configure, OpenSSL's Configure under perl, s2n-tls's cmake, all through make) with the pinned zig as compiler and
// archiver. Run from the repository's root:
//
//   ./teq interp bench/native-libs.scala -- --zig <zig 0.16.0> --out <dir> [--sources <dir>] [--jobs <n>]
//   ./teq interp bench/native-libs.scala -- --self-test
//
// The zips go to <dir>/assets, and a row of the pin table is printed for each. --sources names a directory that holds
// the tarballs already (verified all the same); the others are fetched with curl. What it needs besides zig: curl,
// tar, gzip, perl, make and cmake. The same sources give the same bytes in any directory, on any machine, in any time
// zone: the compiler's paths under <dir> are written relative to it and zig's own as under `zig` (the debug
// information names zig's headers; -ffile-prefix-map in the compiler's wrapper, out of what OpenSSL records of its
// flags), OpenSSL's prefix is fixed and its build date is the release's
// (SOURCE_DATE_EPOCH), zig's ar writes no time, uid or gid, and every entry of a zip carries the time 0, a DOS time
// before 1980, written as 1980-01-01 whatever the time zone. Each phase has its bound; the whole takes a few minutes
// on four cores. Status 1 when a phase fails, its log's last lines printed. Before every build, and alone with
// --self-test (status 1 where they do not), the wrappers are written under a directory whose name holds a space, a
// quote, a dollar sign and a backquote and held to pass every word to zig whole.
import java.nio.file.{Files, Path, Paths}

/// A library's source: its tarball's URL, SHA-256 and top directory, and the licence files it holds there.
final case class Source(url: String, sha256: String, dir: String, licenses: Vector[String]):
  def file: String = url.substring(url.lastIndexOf('/') + 1)

final case class Library(name: String, version: String, source: Source)

/// The tag the archives are published under: a new set of archives is a new tag.
val tag = "native-libs-1"
val zigVersion = "0.16.0"
/// zig's name of the target and OpenSSL's and cmake's of the machine.
val target = "x86_64-linux-musl"
val opensslTarget = "linux-x86_64"
val cmakeProcessor = "x86_64"
/// OpenSSL 3.5.9's release, the build date it records.
val sourceDateEpoch = "1790691008"
val commonFlags = "-ffunction-sections -fdata-sections"

val z = Library("z", "1.3.1", Source("https://github.com/madler/zlib/releases/download/v1.3.1/zlib-1.3.1.tar.gz",
  "9a93b2b7dfdac77ceba5a558a580e74667dd6fede4585b91eefb60f03b72df23", "zlib-1.3.1", Vector("LICENSE")))
val crypto = Library("crypto", "3.5.9", Source("https://github.com/openssl/openssl/releases/download/openssl-3.5.9/openssl-3.5.9.tar.gz",
  "603f5602e2eef00d77fbd429d34dcd5822bb301757a1bc9cdb24c670f1eb859a", "openssl-3.5.9", Vector("LICENSE.txt")))
val s2n = Library("s2n", "1.5.27", Source("https://github.com/aws/s2n-tls/archive/refs/tags/v1.5.27.tar.gz",
  "95d6e5ada2d66108653e91de3ceea800987fde9eb190ef219034ab4fd06b114c", "s2n-tls-1.5.27", Vector("LICENSE", "NOTICE")))

def sha256(bytes: Array[Byte]): String =
  java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(bytes))

/// A word as a POSIX shell reads it back whatever it holds: in single quotes, each quote of it ended, escaped and begun
/// again. Every word of a script this one writes that is not a constant goes through it.
def shQuoted(word: String): String = "'" + word.replace("'", "'\\''") + "'"

/// What the compiler's wrapper runs zig with. zig turns UBSan on for a compilation with no -O flag (configure's probes
/// among them), and links its runtime; the paths under `out` are written relative to it and zig's own as under `zig`.
def ccWords(zig: Path, out: Path): Vector[String] =
  Vector("cc", "-target", target, "-fno-sanitize=undefined", "-g0", s"-ffile-prefix-map=$out=.", s"-ffile-prefix-map=${zig.getParent}=zig")

/// The tools the builds call by name, written into `bin` and put on the PATH, so that what a build records of the
/// compiler is its name: each runs `zig` with its cache under `out`.
def writeWrappers(zig: Path, out: Path, bin: Path): Unit =
  val cache = shQuoted(out.resolve("zig-cache").toString)
  for (name, words) <- Vector("teq-cc" -> ccWords(zig, out), "teq-ar" -> Vector("ar"), "teq-ranlib" -> Vector("ranlib")) do
    val file = bin.resolve(name)
    val command = (zig.toString +: words).map(shQuoted).mkString(" ")
    Files.writeString(file, "#!/bin/sh\nexport ZIG_GLOBAL_CACHE_DIR=" + cache + " ZIG_LOCAL_CACHE_DIR=" + cache + "\nexec " + command + " \"$@\"\n")
    Sh("chmod", "755", file.toString).run()

/// Whether the wrappers pass every word whole: written under a directory whose name holds a space, a quote, a dollar
/// sign and a backquote, around a zig that prints the cache and the words it is given, the compiler's run with a word
/// of its own; what it prints has to be the cache and the words intended, one a line. Run before every build, and
/// alone by --self-test.
def wrappersHold(): Boolean =
  val base = Files.createTempDirectory("native-libs.")
  try
    val dir = Files.createDirectories(base.resolve("a dir's $HOME `id`"))
    val (zig, bin) = (dir.resolve("zig"), Files.createDirectories(dir.resolve("bin")))
    Files.writeString(zig, "#!/bin/sh\nprintf '%s\\n' \"$ZIG_GLOBAL_CACHE_DIR\" \"$@\"\n")
    Sh("chmod", "755", zig.toString).run()
    writeWrappers(zig, dir, bin)
    val ran = Sh(bin.resolve("teq-cc").toString, "-c", "an input's $name.c").timeout(30).check(false).run()
    ran.ok && ran.lines == (dir.resolve("zig-cache").toString +: ccWords(zig, dir)) ++ Vector("-c", "an input's $name.c")
  finally Sh.remove(base)

@main def nativeLibs(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/native-libs.scala --zig <zig 0.16.0> --out <dir> [--sources <dir>] [--jobs <n>] | --self-test")
  if args.flag("self-test") then
    args.exactly(0)
    val held = wrappersHold()
    println(s"native-libs: the wrappers ${if held then "pass every word whole" else "do NOT pass every word whole"}")
    Script.exit(if held then 0 else 1)
  if !wrappersHold() then Log.die("the compiler's wrappers do not pass every word whole (--self-test)")
  val zig = Paths.get(args.option("zig").getOrElse(args.fail("name the zig with --zig"))).toAbsolutePath
  val out = Paths.get(args.option("out").getOrElse(args.fail("name the output directory with --out"))).toAbsolutePath.normalize
  val sources = args.option("sources").map(s => Paths.get(s).toAbsolutePath)
  val jobs = args.option("jobs").map(n => n.toIntOption.filter(_ > 0).getOrElse(args.fail(s"--jobs takes a count, not $n"))).getOrElse(4)
  args.exactly(0)
  val said = Sh(zig.toString, "version").timeout(30).check(false).run()
  if !said.ok || said.out.trim != zigVersion then Log.die(s"$zig is not zig $zigVersion")

  val (dl, src, build, stage, bin, logs, assets) =
    (out.resolve("dl"), out.resolve("src"), out.resolve("build"), out.resolve("stage"), out.resolve("bin"), out.resolve("logs"), out.resolve("assets"))
  for d <- Vector(src, build, stage, assets) do Sh.remove(d)
  for d <- Vector(dl, src, build, stage, bin, logs, assets, out.resolve("zig-cache"), out.resolve("tmp")) do Files.createDirectories(d)

  writeWrappers(zig, out, bin)
  val env = Vector(
    "PATH" -> s"$bin:${System.getenv("PATH")}", "CC" -> "teq-cc", "AR" -> "teq-ar", "RANLIB" -> "teq-ranlib",
    "LC_ALL" -> "C", "TZ" -> "UTC", "SOURCE_DATE_EPOCH" -> sourceDateEpoch, "TMPDIR" -> out.resolve("tmp").toString)

  def phase(library: Library, name: String, seconds: Int, dir: Path, cmd: String*): Unit = phaseWith(Vector(), library, name, seconds, dir, cmd*)
  def phaseWith(more: Vector[(String, String)], library: Library, name: String, seconds: Int, dir: Path, cmd: String*): Unit =
    val log = logs.resolve(s"${library.name}-$name.log")
    Log.stamped(s"${library.name}: $name")
    val r = Sh(cmd*).in(dir).withEnv((env ++ more)*).stdout(log).mergeErr.timeout(seconds).check(false).run()
    if !r.ok then
      println(s"FAIL ${library.name} $name: exit ${r.status}${if r.timedOut then " (timed out)" else ""}, $log")
      new String(Files.readAllBytes(log), "UTF-8").linesIterator.toVector.takeRight(20).foreach(println)
      Script.exit(1)

  /// The library's source tree, its tarball fetched or taken from --sources and held to its digest first.
  def unpacked(library: Library): Path =
    val tarball = dl.resolve(library.source.file)
    sources.map(_.resolve(library.source.file)).filter(Files.isRegularFile(_)).foreach(kept => Files.copy(kept, tarball, java.nio.file.StandardCopyOption.REPLACE_EXISTING))
    if !Files.isRegularFile(tarball) then phase(library, "fetch", 300, out, "curl", "-fsSL", "--max-time", "300", "-o", tarball.toString, library.source.url)
    val found = sha256(Files.readAllBytes(tarball))
    if found != library.source.sha256 then Log.die(s"${library.source.url} has sha256 $found where the script pins ${library.source.sha256}")
    val into = src.resolve(library.name)
    Files.createDirectories(into)
    phase(library, "unpack", 120, out, "tar", "-xzf", tarball.toString, "-C", into.toString)
    into.resolve(library.source.dir)

  /// Each library's archive, the directory its public headers are copied from, and its source tree.
  val archives = Vector.newBuilder[(Library, Path, Path, Path)]

  val zSource = unpacked(z)
  phaseWith(Vector("CFLAGS" -> s"-O2 $commonFlags"), z, "configure", 300, zSource, "./configure", "--static")
  phase(z, "build", 600, zSource, "make", s"-j$jobs", "libz.a")
  val zHeaders = Files.createDirectories(stage.resolve("z/include"))
  for header <- Vector("zlib.h", "zconf.h") do Files.copy(zSource.resolve(header), zHeaders.resolve(header))
  archives += ((z, zSource.resolve("libz.a"), zHeaders, zSource))

  val cryptoSource = unpacked(crypto)
  val cryptoBuild = Files.createDirectories(build.resolve("crypto"))
  // no-dso, no-engine: nothing is loaded into a static executable; no-autoload-config: the host's openssl.cnf is
  // never read; the default trust locations are /etc/ssl/cert.pem and /etc/ssl/certs.
  phase(crypto, "configure", 300, cryptoBuild, "perl", cryptoSource.resolve("Configure").toString, opensslTarget,
    "--prefix=/usr/local", "--libdir=lib", "--openssldir=/etc/ssl", "no-shared", "no-tests", "no-apps", "no-docs", "no-dso",
    "no-engine", "no-async", "no-comp", "no-autoload-config", "-ffunction-sections", "-fdata-sections")
  phase(crypto, "build", 1500, cryptoBuild, "make", s"-j$jobs", "build_libs")
  phase(crypto, "install", 300, cryptoBuild, "make", "install_dev", s"DESTDIR=$stage")
  val installed = stage.resolve("usr/local")
  archives += ((crypto, installed.resolve("lib/libcrypto.a"), installed.resolve("include"), cryptoSource))

  val s2nSource = unpacked(s2n)
  val s2nBuild = build.resolve("s2n")
  phase(s2n, "configure", 300, out, "cmake", "-S", s2nSource.toString, "-B", s2nBuild.toString, "-G", "Unix Makefiles",
    "-DCMAKE_SYSTEM_NAME=Linux", s"-DCMAKE_SYSTEM_PROCESSOR=$cmakeProcessor", "-DCMAKE_C_COMPILER=teq-cc", "-DCMAKE_AR=" + bin.resolve("teq-ar"),
    "-DCMAKE_RANLIB=" + bin.resolve("teq-ranlib"), s"-DCMAKE_C_FLAGS=$commonFlags", "-DCMAKE_BUILD_TYPE=Release", "-DBUILD_SHARED_LIBS=OFF",
    "-DBUILD_TESTING=OFF", s"-DCMAKE_PREFIX_PATH=$installed", s"-DCMAKE_FIND_ROOT_PATH=$installed",
    "-DCMAKE_FIND_ROOT_PATH_MODE_LIBRARY=ONLY", "-DCMAKE_FIND_ROOT_PATH_MODE_INCLUDE=ONLY", "-DCMAKE_FIND_ROOT_PATH_MODE_PACKAGE=ONLY",
    s"-Dcrypto_INCLUDE_DIR=$installed/include", s"-Dcrypto_STATIC_LIBRARY=$installed/lib/libcrypto.a")
  phase(s2n, "build", 900, out, "cmake", "--build", s2nBuild.toString, "-j", jobs.toString)
  phase(s2n, "install", 120, out, "cmake", "--install", s2nBuild.toString, "--prefix", stage.resolve("s2n").toString)
  archives += ((s2n, s2nBuild.resolve("lib/libs2n.a"), stage.resolve("s2n/include"), s2nSource))

  /// The files under `dir`, by their paths relative to it, in their names' order.
  def tree(dir: Path, under: String): Vector[(String, Path)] =
    val s = Files.list(dir)
    val entries = try s.toArray.toVector.map(_.asInstanceOf[Path]).sortBy(_.getFileName.toString) finally s.close()
    entries.flatMap(p => if Files.isDirectory(p) then tree(p, s"$under${p.getFileName}/") else Vector((s"$under${p.getFileName}", p)))

  for (library, archive, headers, source) <- archives.result() do
    val file = s"lib${library.name}-${library.version}-$target.zip"
    val zip = assets.resolve(file)
    val stream = new java.util.zip.ZipOutputStream(Files.newOutputStream(zip))
    def entry(name: String, from: Path): Unit =
      val e = new java.util.zip.ZipEntry(name)
      e.setTime(0L)
      stream.putNextEntry(e)
      stream.write(Files.readAllBytes(from))
      stream.closeEntry()
    if !Files.isRegularFile(archive) then Log.die(s"${library.name}: the build made no $archive")
    entry(s"lib/lib${library.name}.a", archive)
    for (name, header) <- tree(headers, "include/") do entry(name, header)
    for license <- library.source.licenses do entry(s"licenses/$license", source.resolve(license))
    stream.close()
    val bytes = Files.readAllBytes(zip)
    println(s"native-libs lib${library.name} ${library.version} $target $tag/$file ${sha256(bytes)} ${bytes.length}")
}
