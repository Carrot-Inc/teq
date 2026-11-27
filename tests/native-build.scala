//> using file ../tools/script
//> using file support/null-guards-judge.scala
// Tests of the native build: `teq nir --link <main> <class path> --build=<executable>` over NIR that Scala Native's
// compiler plugin wrote (tests/native-build/fixtures: hello world, the nine programs of tests/cases/bench_*.scala, a
// program reading a null receiver where a path into the read checked it and another did not, and the renamed fixture;
// tests/native-build/README says how), linked against the jars of Scala Native 0.5.12 that tests/nir/digests.txt
// names, built by zig into a static x86_64-linux-musl executable and run: its output has to be what scalac's JVM
// build prints (tests/native-build/hello.expected, nullguard.expected, tests/cases/bench_<name>.expected); the null
// receiver's program lowered by teq's own null-check rule (its listing and its modules) has to be what the rule of
// Scala Native's (--null-guards=scala-native) lowers or differ from it only by added null checks, as
// tests/support/null_guards.py judges it (the judge of tests/native-link.scala, held to its planted verdicts). The commands
// hello world's build gives zig have to be Scala Native's own build's (tests/native-build/commands.txt). A build
// compiles every C, C++ and assembly source again and takes the program's LLVM IR modules from zig's cache: a second
// build has to compile every source again and no module, and a zig of another build that prints the same version is
// served the modules of the first; a module renamed by a new directory of its hash has to be the fresh build's, and the
// files of a module the build no longer makes are left unlinked. What zig builds of its own is in its global cache,
// which every build shares (--zig-cache), and the program's module objects in its local cache in the work directory:
// fifty edits, a work directory removed, two programs built at once and builds over what an older teq left each leave
// the shared cache as a build of hello world leaves it, and the whole run leaves it as its first build did. A build
// into a work directory that held another has to print what a fresh one prints after each change: a header changed,
// added to an earlier include directory (which zig's own cache does not see) or removed, a dangling link there
// gaining its target (nor this), a define, the library's descriptor, another program built in between, a header changed while its library compiles (the cut
// check's two reproductions), headers reached by two names, through a link, a linked directory, a linked include
// directory or into zig's own files, a header named as a build's list, one found above an include directory, and an
// include directory that cannot be listed. The dependency output options of a library's (the forms
// src/native/library-options.txt drops), each alone and every two, have to leave every compilation's object and
// command those of the same build without them; names holding `#`, `$`, a blank, `:`, a backslash or a tab have to
// build; the caller's environment has to give zig nothing but SOURCE_DATE_EPOCH (and `zig version` nothing at all),
// and a build under every variable zig and clang's driver read has to write the objects and the executable a build
// under none writes; the cases of tests/native-build/library-options.expected have to be passed to zig as they are or
// refused by name; the review's descriptors have to be read as Scala Native reads them; the jars of
// tests/native-build/jar-names.expected have to be read as Scala Native's build reads them through the JDK's readers
// (refused before anything is unpacked or compiled, taken as no library, or unpacked to the same files); two and three
// builds at once into one work directory, and a build after one killed while it compiled, have to link their own
// objects, a read-only work directory is refused; a library refused last on the class path has to be refused before
// anything is compiled; a build for another target, or without its work directory, is refused; and the wraps of
// src/native/static_musl.c that find a system's trust roots, linked over a stand-in for libcrypto
// (tests/native-build/trust_roots.c, libcrypto_stub.c), report a verification that fails with no certificate in any
// place once on stderr, and nothing where a place holds one or nothing is verified. The
// zig is the one TEQ_ZIG names, else zig 0.16.0 on the PATH, else the one bench/zig.sh fetches
// (target/cross/zig-x86_64-linux-0.16.0/zig); zig's cache is the run's own, under its output (out/native-build/zig-cache),
// so that no run leaves anything in a cache another run or machine shares. Without
// such a zig, without a jar in the coursier cache (tests/support/jars.sh names them for the fetch), without sha256sum, cut,
// cat, flock, pgrep, ln, chmod, kill, env, sh, diff or sleep on the PATH, or on a host other than x86_64 Linux, a `skip`
// line says what did not run and nothing passes. Run from the repository's root:
// `./teq interp tests/native-build.scala [-- --teq <binary>]`, the compiler `--teq`, else $TEQ, else
// target/release/teq.
import java.nio.file.{Files, Path, Paths}

val zigVersion = "0.16.0"

/// The programs: the fixtures' directory, the main class, the output scalac's JVM build prints.
val programs = Vector(
  ("hello", "hello", "tests/native-build/hello.expected"),
  ("adts", "AdtsBench", "tests/cases/bench_adts.expected"),
  ("boxing", "BoxingBench", "tests/cases/bench_boxing.expected"),
  ("closures", "ClosuresBench", "tests/cases/bench_closures.expected"),
  ("codec", "CodecBench", "tests/cases/bench_codec.expected"),
  ("collections", "CollectionsBench", "tests/cases/bench_collections.expected"),
  ("dispatch", "DispatchBench", "tests/cases/bench_dispatch.expected"),
  ("numeric", "NumericBench", "tests/cases/bench_numeric.expected"),
  ("strings", "StringsBench", "tests/cases/bench_strings.expected"),
  ("toplevel", "TopLevelBench", "tests/cases/bench_toplevel.expected"),
  ("nullguard", "nullguard", "tests/native-build/nullguard.expected"))

def version(zig: Path): Option[String] =
  val said = Sh(zig.toString, "version").timeout(30).check(false).run()
  if said.ok then Some(said.out.trim) else None

/// Whether a run ended well, within its bound, and printed exactly the expected bytes: every output check of the suite.
def printedExactly(ran: Sh.Result, expected: Array[Byte]): Boolean =
  ran.ok && !ran.timedOut && java.util.Arrays.equals(Files.readAllBytes(ran.outFile), expected)

def files(dir: Path): Vector[Path] =
  if !Files.isDirectory(dir) then Vector.empty
  else
    val s = Files.walk(dir)
    try s.toArray.toVector.map(_.asInstanceOf[Path]).filter(Files.isRegularFile(_)).sortBy(_.toString)
    finally s.close()

/// zig's local cache in a build's work directory, where it keeps the program's module objects (src/native/build.rs,
/// `LOCAL_CACHE`).
val localCache = "zig-cache"

/// The files of a work directory that the build writes of itself: all but zig's local cache there.
def builtFiles(work: Path): Vector[Path] = files(work).filterNot(f => work.relativize(f).startsWith(localCache))

@main def nativeBuild(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/native-build.scala [--teq <binary>] [--strand <name>]")
  val teq = args.teq("./target/release/teq")
  val strand = args.option("strand")
  args.exactly(0)
  if !Files.isDirectory(Paths.get("tests/native-build")) then Log.die("tests/native-build.scala runs from the repository's root")
  var pass = 0
  var fail = 0
  def done(): Nothing =
    println(s"native-build: $pass passed, $fail failed")
    Script.exit(if fail == 0 then 0 else 1)

  val host = s"${System.getProperty("os.name")} ${System.getProperty("os.arch")}"
  if host != "Linux amd64" then
    println(s"skip native-build: the executables are x86_64 Linux ones, not of $host")
    done()
  val named = Option(System.getenv("TEQ_ZIG")).filter(_.nonEmpty).map(Paths.get(_).toAbsolutePath)
  val pinned = s"target/cross/zig-x86_64-linux-$zigVersion/zig"
  val zig = named.orElse((Sh.which("zig").toVector :+ Paths.get(pinned).toAbsolutePath).find(z => Files.isExecutable(z) && version(z).contains(zigVersion)))
  if zig.isEmpty then
    println(s"skip native-build: no zig $zigVersion (TEQ_ZIG, zig on the PATH, or $pinned, which bench/zig.sh fetches)")
    done()
  val toolsMissing = Vector("sha256sum", "cut", "cat", "flock", "pgrep", "ln", "chmod", "kill", "env", "sh", "diff", "sleep", "python3").filter(Sh.which(_).isEmpty)
  if toolsMissing.nonEmpty then
    println(s"skip native-build: no ${toolsMissing.mkString(", ")} on the PATH")
    done()
  val caches = Option(System.getenv("COURSIER_CACHE")).filter(_.nonEmpty).toList ++
    List("/Library/Caches/Coursier/v1", "/.cache/coursier/v1").map(System.getProperty("user.home") + _)
  val wanted = Files.readString(Paths.get("tests/nir/digests.txt")).linesIterator.filter(l => l.nonEmpty && !l.startsWith("#")).map(_.split(" ").last).toList
  val found = wanted.map(path => path -> caches.map(c => Paths.get(s"$c/https/repo1.maven.org/maven2/$path")).find(Files.isRegularFile(_)))
  for (path, jar) <- found if jar.isEmpty do println(s"skip ${path.substring(path.lastIndexOf('/') + 1)}: not in the coursier cache")
  if found.exists(_._2.isEmpty) then done()

  val separator = if java.io.File.separatorChar == '\\' then ";" else ":"
  val jars = found.map(_._2.get.toString)
  val out = Paths.get("out/native-build").toAbsolutePath
  if strand.isEmpty then Sh.remove(out)
  Files.createDirectories(out)
  val cache = out.resolve("zig-cache")
  def buildBy(by: Path, name: String, main: String, extra: String*): Sh.Result =
    val classPath = (Paths.get(s"tests/native-build/fixtures/$name").toAbsolutePath.toString :: jars).mkString(separator)
    Sh((List(teq, "nir", "--link", main, classPath, s"--build=${out.resolve(name)}", s"--work=${out.resolve("work").resolve(name)}",
      s"--zig=$by", s"--zig-cache=$cache") ++ extra)*).mergeErr.timeout(900).check(false).run()
  def build(name: String, main: String, extra: String*): Sh.Result = buildBy(zig.get, name, main, extra*)
  /// A cache's files with their sizes, by their names in it.
  def cacheListing(dir: Path): Map[String, Long] = files(dir).map(f => dir.relativize(f).toString -> Files.size(f)).toMap
  /// How a cache's listing moved from `was` to `now`.
  def moved(was: Map[String, Long], now: Map[String, Long]): String =
    val (added, gone) = (now.keySet -- was.keySet, was.keySet -- now.keySet)
    val resized = now.keySet.intersect(was.keySet).filter(k => now(k) != was(k))
    s"${added.size} files added (${added.toVector.sorted.take(3).mkString(", ")}), ${gone.size} gone, ${resized.size} of another size"
  /// The listing of the shared cache the suite's first build left, written before the strands start.
  val firstListing = out.resolve("first-cache.txt")
  /// hello world built from `fixture` into `work` with a zig and a shared cache of the caller's.
  val helloFixture = Paths.get("tests/native-build/fixtures/hello").toAbsolutePath
  def helloAt(fixture: Path, work: Path, exe: Path, zigCache: Path, by: Path): Sh.Result =
    Sh(teq, "nir", "--link", "hello", (fixture.toString :: jars).mkString(separator), s"--build=$exe", s"--work=$work", s"--zig=$by",
      s"--zig-cache=$zigCache").mergeErr.timeout(900).check(false).run()
  def localObjects(work: Path): Int = files(work.resolve(localCache).resolve("o")).count(_.toString.endsWith(".o"))
  def prints(exe: Path, line: String): Boolean = printedExactly(Sh(exe.toString).timeout(300).check(false).run(), line.getBytes("UTF-8"))
  def runs(name: String, expected: String): Boolean = runsAt(out.resolve(name), expected)
  def runsAt(executable: Path, expected: String): Boolean =
    val ran = Sh(executable.toString).timeout(300).check(false).run()
    val same = printedExactly(ran, Files.readAllBytes(Paths.get(expected)))
    if !same then
      println(s"FAIL $executable: it exited ${ran.status}, its output against $expected:")
      Sh("diff", ran.outFile.toString, expected).check(false).run().lines.take(10).foreach(println)
      ran.errTail.linesIterator.take(5).foreach(println)
    same

  // The library's dependency output options: the dependency file a compilation is given is the build's own (it turns
  // zig's cache off, src/native/build.rs, `compile_all`), so a library's options lose every form
  // src/native/library-options.txt drops (the table the build reads; read here too), its value with it. The table's
  // cells: each such form in each spelling, its value holding `#`, `$`, a blank or `:`, alone, and every two of them,
  // each the options of a library of its own (the C file `cell.c` beside its descriptor) between two defines the build
  // keeps and with one between; all of them in one build, and the same build again where each library's options are the
  // three defines alone, the control. Zig is one that keeps each compilation's arguments and environment (`keeping`).
  // Of every cell, the object and the arguments zig was given are byte for byte the control's; the options it was
  // written with hold a dropped form and the arguments zig was given none (both read by this suite's own matcher,
  // `holdsOption`); the build leaves no file the control's does not; the executable prints what the control's does.
  // Then the header every library includes changed, built again, and built fresh: the two print the same, the
  // header's new value. The three findings of the review are cells by name: the header changed (above), `-MP` and
  // `-MT,hash#target`.
  final case class Row(spelling: String, prefix: Boolean, role: String)
  val rows = Files.readString(Paths.get("src/native/library-options.txt")).linesIterator.filter(l => l.nonEmpty && !l.startsWith("#"))
    .map(_.trim.split("\\s+")).map(f => Row(f(0).stripSuffix("*"), f(0).endsWith("*"), f(1))).toVector
  val droppedRows = rows.filter(r => r.role == "dropped" || r.role == "dropped+next")
  /// Whether arguments hold one of a form the table drops.
  def holdsOption(args: Seq[String]): Boolean =
    args.exists(a => droppedRows.exists(r => if r.prefix then a.startsWith(r.spelling) && a.length > r.spelling.length else a == r.spelling))
  val tableValues = Vector("hash#target", "dollar$target", "a blank", "colon:target")
  def spellingsOf(r: Row, value: String): Vector[String] =
    if r.role == "dropped+next" then Vector(r.spelling, value) else if r.prefix then Vector(r.spelling + value) else Vector(r.spelling)
  val singleCells = (for r <- droppedRows; v <- (if r.role == "dropped" && !r.prefix then Vector("") else tableValues) yield spellingsOf(r, v)).distinct
  val plainSpellings = droppedRows.zipWithIndex.map((r, i) => spellingsOf(r, tableValues(i % tableValues.length)))
  val pairCells = for (a, i) <- plainSpellings.zipWithIndex; b <- plainSpellings.drop(i) yield (a, b)
  /// A cell: its name (the options as the descriptor writes them), the options before the define between and after it.
  val cells: Vector[(String, Vector[String], Vector[String])] =
    singleCells.map(s => (s.mkString(","), s, Vector.empty[String])) ++ pairCells.map((a, b) => (s"${a.mkString(",")} + ${b.mkString(",")}", a, b))

  /// What a compilation leaves of itself: its object and the arguments zig was given.
  final case class Kept(obj: Option[Array[Byte]], args: Option[Vector[String]])
  def same(a: Option[Array[Byte]], b: Option[Array[Byte]]): Boolean = a.isDefined && b.isDefined && java.util.Arrays.equals(a.get, b.get)
  /// The table's verdict: the build ended well within its bound; every cell of the manifest once, kept on both sides,
  /// the object and the arguments the control's; its options holding an option of the table and zig's arguments
  /// (but the build's own last three, `-MD -MF <file>`) none; nothing kept of a cell outside the manifest; no file the
  /// control's build did not leave.
  def tableFaults(manifest: Vector[(String, Vector[String])], control: Map[String, Kept], cell: Map[String, Kept], built: Sh.Result, stray: Vector[String]): Vector[String] =
    val names = manifest.map(_._1)
    val ended = if built.timedOut then Vector("the build ran past its bound") else if !built.ok then Vector(s"the build exited ${built.status}") else Vector.empty
    ended ++ names.diff(names.distinct).distinct.map(n => s"$n: twice in the manifest") ++
      cell.keySet.diff(names.toSet).toVector.sorted.map(n => s"$n: kept of no cell") ++ stray.map(f => s"$f: left by this build alone") ++
      manifest.flatMap { (name, written) =>
        (control.get(name), cell.get(name)) match
          case (Some(c), Some(k)) =>
            Vector(
              Option.when(!same(k.obj, c.obj))("the object is not the control's"),
              Option.when(k.args.isEmpty || k.args != c.args)("zig's arguments are not the control's"),
              Option.when(!holdsOption(written))("its options hold no option of the table"),
              Option.when(k.args.exists(a => holdsOption(a.dropRight(3))))("zig was given an option of the table")).flatten.map(f => s"$name: $f")
          case _ => Vector(s"$name: nothing kept")
      }

  // The self-test: the output check given planted bad runs, each of which it must refuse, and a good one it must take.
  val planted = Files.createDirectories(out.resolve("self-test"))
  def planting(name: String, body: String, seconds: Int): Sh.Result =
    val file = planted.resolve(name)
    Files.writeString(file, s"#!/bin/sh\n$body\n")
    Sh("chmod", "755", file.toString).run()
    Sh(file.toString).timeout(seconds).check(false).run()
  val good = "x\n".getBytes
  if strand.isEmpty then
    val plantedRuns = Vector("another output" -> planting("other", "echo y", 10), "an empty output" -> planting("empty", "true", 10),
      "an output cut short" -> planting("short", "printf x", 10), "a run past its bound" -> planting("slow", "sleep 5; echo x", 1),
      "a failed run" -> planting("failed", "echo x; exit 3", 10), "a missing executable" -> Sh("sh", "-c", "exec \"$0\"", planted.resolve("absent").toString).timeout(10).check(false).run())
    val takenBad = plantedRuns.filter((_, ran) => printedExactly(ran, good))
    val goodTaken = printedExactly(planting("good", "echo x", 10), good)
    for (what, _) <- takenBad do
      fail += 1
      println(s"FAIL the self-test: $what was taken")
    if !goodTaken then
      fail += 1
      println("FAIL the self-test: the good run was refused")
    if takenBad.isEmpty && goodTaken then println(s"native-build self-test: ${plantedRuns.length} planted bad runs refused, the good run taken")
    // The verdict's self-test: planted bad compilations, each of which it must refuse, and the good ones it must take.
    {
      val bytes = (s: String) => Some(s.getBytes)
      val good = Kept(bytes("OBJ"), Some(Vector("cc", "-c", "s.c", "-MD", "-MF", "o.d")))
      val manifest = Vector("-MP" -> Vector("-MP"), "-MT,x" -> Vector("-MT", "x"))
      val both = manifest.map(_._1 -> good).toMap
      val okRun = planting("table-ok", "echo x", 10)
      val takenGood = tableFaults(manifest, both, both, okRun, Vector.empty).isEmpty
      val plants = Vector(
        "a cell whose object differs" -> tableFaults(manifest, both, both.updated("-MP", good.copy(obj = bytes("OBJ2"))), okRun, Vector.empty),
        "a cell whose arguments differ" -> tableFaults(manifest, both, both.updated("-MT,x", good.copy(args = Some(Vector("cc", "-c", "t.c")))), okRun, Vector.empty),
        "a cell without its object" -> tableFaults(manifest, both, both.updated("-MT,x", good.copy(obj = None)), okRun, Vector.empty),
        "a missing cell" -> tableFaults(manifest, both, both - "-MT,x", okRun, Vector.empty),
        "a run past its bound" -> tableFaults(manifest, both, both, planting("table-slow", "sleep 5", 1), Vector.empty),
        "a failed build" -> tableFaults(manifest, both, both, planting("table-failed", "exit 1", 10), Vector.empty),
        "zig given an option of the table" -> tableFaults(manifest, both.updated("-MP", good.copy(args = Some("-MP" +: good.args.get))), both.updated("-MP", good.copy(args = Some("-MP" +: good.args.get))), okRun, Vector.empty),
        "a cell whose options hold none" -> tableFaults(manifest :+ ("-DX" -> Vector("-DX")), both + ("-DX" -> good), both + ("-DX" -> good), okRun, Vector.empty),
        "a cell twice in the manifest" -> tableFaults(manifest :+ manifest.head, both, both, okRun, Vector.empty),
        "a compilation of no cell" -> tableFaults(manifest, both, both + ("-MD" -> good), okRun, Vector.empty),
        "a file left by the cells' build alone" -> tableFaults(manifest, both, both, okRun, Vector("stray.d")))
      for (what, faults) <- plants if faults.isEmpty do
        fail += 1
        println(s"FAIL the table's self-test: $what was taken")
      if !takenGood then
        fail += 1
        println("FAIL the table's self-test: the good compilations were refused")
      if takenGood && plants.forall(_._2.nonEmpty) then println(s"native-build table self-test: ${plants.length} planted bad compilations refused, the good ones taken")
    }
    Sh.remove(planted)

  def shellQuoted(s: String): String = "'" + s.replace("'", "'\\''") + "'"
  def tool(name: String): String = Sh.which(name).map(_.toString).getOrElse(Log.die(s"tests/native-build.scala needs $name on the PATH"))
  val (sha256sum, cutTool, catTool, flockTool) = (tool("sha256sum"), tool("cut"), tool("cat"), tool("flock"))
  /// A zig that keeps under `into`, for every compilation it is given, its arguments and the environment it was given, by
  /// the first sixteen hexadecimal digits of the SHA-256 of the object's path (of a `zig version`, the environment as
  /// `version.env`), then ends as the suite's zig ends. Every path it needs is written into it: the build gives zig no
  /// variable of the suite's.
  def keeping(name: String, into: Path): Path =
    Files.createDirectories(into)
    val dir = Files.createDirectories(out.resolve("keeping").resolve(name))
    val script = dir.resolve("zig")
    val (q, kept) = (shellQuoted, shellQuoted(into.toString))
    Files.writeString(script, s"#!/bin/sh\n[ \"$$1\" = version ] && ${q(catTool)} /proc/$$$$/environ > $kept/version.env\no=; p=\nfor a; do\n  [ \"$$p\" = -o ] && o=$$a\n  p=$$a\ndone\n" +
      s"${q(zig.get.toString)} \"$$@\"; c=$$?\n" +
      s"if [ -n \"$$o\" ]; then\n  k=$$(printf '%s' \"$$o\" | ${q(sha256sum)} | ${q(cutTool)} -c1-16)\n" +
      s"  printf '%s\\n' \"$$@\" > $kept/\"$$k.args\"\n" +
      s"  ${q(catTool)} /proc/$$$$/environ > $kept/\"$$k.env\"\nfi\nexit $$c\n")
    Sh("chmod", "755", script.toString).run()
    script
  def keyOf(obj: Path): String =
    java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(obj.toString.getBytes("UTF-8"))).take(16)
  def readIf(p: Path): Option[Array[Byte]] = if Files.isRegularFile(p) then Some(Files.readAllBytes(p)) else None
  def keptArgs(into: Path, obj: Path): Option[Vector[String]] = readIf(into.resolve(s"${keyOf(obj)}.args")).map(b => new String(b, "UTF-8").linesIterator.toVector)
  /// The environment zig was given for an object's compilation: its variables' names and values.
  def keptEnvironment(into: Path, obj: Path): Option[Map[String, String]] = environmentIn(into.resolve(s"${keyOf(obj)}.env"))
  def environmentIn(file: Path): Option[Map[String, String]] =
    readIf(file).map(b => new String(b, "UTF-8").split('\u0000').filter(_.nonEmpty).map(v => v.takeWhile(_ != '=') -> v.dropWhile(_ != '=').drop(1)).toMap)
  /// What a build into `work` left that it writes of itself: every file but the libraries' descriptors and stamps, with
  /// its SHA-256.
  def leftIn(work: Path): Map[String, String] =
    builtFiles(work).filter(f => f.getFileName.toString != "scala-native.properties" && f.getFileName.toString != "stamp")
      .map(f => work.relativize(f).toString -> java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(f)))).toMap
  val helloLine = Files.readString(Paths.get("tests/native-build/hello.expected"))

  // Two builds into one work directory at once (the second started a moment after the first, the moment changing),
  // with --lib-dirs whose header gives the sixty C files of a library one value or the other: each build waits its turn
  // for the directory (`<work>/lock`) and its executable prints its own value, in every one of forty pairs; then three
  // at once, ten times. Then a build killed (teq alone, SIGKILL) while it compiles, and the other built into the
  // directory: the killed build's compilers (each a zig that, its compilation done, waits until the cell releases it)
  // hold the lock (src/native/build.rs, `locked`), so the next build says once that it waits, and it is still waiting,
  // with the compilers left, when the cell sees that line; only then are they released. The next build takes the lock
  // only after its own link (`build::build`), seconds after it starts, so a compiler that ended on its own clock could
  // be gone by then and leave nothing to wait for. Then the next build ends (no compiler is left by then) and prints its
  // own value; then the first again, its own. A work directory that cannot be written is refused naming its lock, and
  // nothing is built.
  val raceFiles = 60
  /// The lock's cells' library in `race`: sixty C files and the main one, and the two --lib-dirs' headers.
  def raceAt(race: Path): Path =
    for dir <- List("lib/scala-native", "LA/include", "LB/include") do Files.createDirectories(race.resolve(dir))
    Files.writeString(race.resolve("LA/include/which.h"), "#define WHICH 1\n")
    Files.writeString(race.resolve("LB/include/which.h"), "#define WHICH 2\n")
    for k <- 0 until raceFiles do Files.writeString(race.resolve(s"lib/scala-native/f$k.c"), s"#include <which.h>\nint c3_$k(void) { return WHICH; }\n")
    Files.writeString(race.resolve("lib/scala-native/main.c"), "#include <stdio.h>\n" + (0 until raceFiles).map(k => s"int c3_$k(void);\n").mkString +
      "__attribute__((constructor)) static void race(void) {\n    printf(\"c3=\");\n" + (0 until raceFiles).map(k => s"    printf(\"%d\", c3_$k());\n").mkString +
      "    printf(\"\\n\");\n    fflush(stdout);\n}\n")
    race
  def raceCmd(race: Path, which: String, by: Path = zig.get): Sh.Cmd =
    val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, race.resolve("lib").toString) ++ jars).mkString(separator)
    Sh(teq, "nir", "--link", "hello", classPath, s"--build=${race.resolve(s"hello-$which")}", s"--work=${race.resolve("work")}", s"--zig=$by",
      s"--zig-cache=$cache", s"--lib-dir=${race.resolve(s"L$which")}").mergeErr.timeout(1800).check(false)
  def raceRan(race: Path, which: String, built: Sh.Result): Boolean =
    val value = if which == "A" then "1" else "2"
    built.ok && printedExactly(Sh(race.resolve(s"hello-$which").toString).timeout(300).check(false).run(), s"c3=${value * raceFiles}\n$helloLine".getBytes("UTF-8"))
  def raceThreeCmd(race: Path, which: String, k: Int): Sh.Cmd =
    val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, race.resolve("lib").toString) ++ jars).mkString(separator)
    Sh(teq, "nir", "--link", "hello", classPath, s"--build=${race.resolve(s"hello-$which-$k")}", s"--work=${race.resolve("work")}", s"--zig=${zig.get}",
      s"--zig-cache=$cache", s"--lib-dir=${race.resolve(s"L$which")}").mergeErr.timeout(1800).check(false)

  // The cells run as strands side by side, each the suite's own child (`--strand <name>`) running its cells below one
  // after another, at most as many at once as the machine has CPUs, the longest first. A strand's cells write
  // directories of their own; every build shares zig's cache. A strand counts by its last line, `native-build strand <name>: <p> passed, <f>
  // failed`, its exit 0 exactly when it failed nothing; one that ends otherwise, past its bound or without that line
  // fails the suite.
  val strands = Vector("edits", "pairs", "cases", "caches", "rounds", "table", "jars", "programs", "misc", "probe")
  if strand.exists(!strands.contains(_)) then args.fail(s"no strand ${strand.get}: the strands are ${strands.mkString(", ")}")
  def strandHas(name: String): Boolean = strand.contains(name)
  def strandCounts(name: String, ran: Sh.Result): Either[String, (Int, Int)] =
    val summary = s"native-build strand ${java.util.regex.Pattern.quote(name)}: ([0-9]+) passed, ([0-9]+) failed".r
    val last = ran.outAll().linesIterator.filter(_.nonEmpty).toVector.lastOption
    last match
      case Some(summary(p, f)) if !ran.timedOut && ran.status == (if f == "0" then 0 else 1) => Right((p.toInt, f.toInt))
      case _ => Left(s"the strand $name ${if ran.timedOut then "ran past its bound" else s"exited ${ran.status}"}, its last line: ${last.getOrElse("none")}")
  if strand.isEmpty then
    // The strands' verdict's self-test: planted strands that died, ended without their line, with another strand's,
    // with a line their exit denies, with more after it, or past their bound, each refused; a good one taken.
    Files.createDirectories(planted)
    val line = "native-build strand pairs: 3 passed, 0 failed"
    val strandPlants = Vector(
      "a strand that died" -> planting("strand-died", "echo started; kill -9 $$", 10),
      "a strand without its line" -> planting("strand-silent", "echo started", 10),
      "another strand's line" -> planting("strand-other", "echo 'native-build strand probe: 3 passed, 0 failed'", 10),
      "a line its exit denies" -> planting("strand-denied", s"echo '$line'; exit 1", 10),
      "a failure its exit denies" -> planting("strand-hidden", "echo 'native-build strand pairs: 2 passed, 1 failed'", 10),
      "a line not its last" -> planting("strand-later", s"echo '$line'; echo more", 10),
      "a strand past its bound" -> planting("strand-slow", s"echo '$line'; sleep 5", 1))
    val strandTaken = strandPlants.filter((_, ran) => strandCounts("pairs", ran).isRight)
    val strandGood = strandCounts("pairs", planting("strand-good", s"echo started; echo '$line'", 10)) == Right((3, 0))
    Sh.remove(planted)
    for (what, _) <- strandTaken do
      fail += 1
      println(s"FAIL the strands' self-test: $what was taken")
    if !strandGood then
      fail += 1
      println("FAIL the strands' self-test: the good strand was refused")
    if strandTaken.isEmpty && strandGood then println(s"native-build strands self-test: ${strandPlants.length} planted bad strands refused, the good one taken")
    // The suite's first build, into the shared cache every build of the suite is given, before any strand starts: what
    // it leaves there is what the whole run is to leave (the last cell, once every strand has ended).
    val firstBuilt = Sh(teq, "nir", "--link", "hello", (Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString :: jars).mkString(separator),
      s"--build=${out.resolve("first")}", s"--work=${out.resolve("work").resolve("first")}", s"--zig=${zig.get}", s"--zig-cache=$cache")
      .mergeErr.timeout(900).check(false).run()
    val afterFirst = cacheListing(cache)
    Files.writeString(firstListing, afterFirst.toVector.sorted.map((name, size) => s"$size $name\n").mkString)
    val cpus = Runtime.getRuntime.availableProcessors
    val slots = cpus.min(strands.length).max(1)
    println(s"native-build strands: ${strands.length}, $slots at once on $cpus CPUs")
    val ended = scala.collection.mutable.Map.empty[String, Sh.Result]
    Sh.pool(slots, strands) { name =>
      Sh.Then(Sh(teq, "interp", "tests/native-build.scala", "--", "--teq", teq, "--strand", name).mergeErr.timeout(3600).check(false),
        ran => { ended(name) = ran; Sh.End })
    }
    for name <- strands do
      val ran = ended(name)
      print(ran.outAll())
      println(s"native-build strand $name: ${ran.millis / 1000} s")
      strandCounts(name, ran) match
        case Right((p, f)) =>
          pass += p
          fail += f
        case Left(why) =>
          fail += 1
          println(s"FAIL $why")
    val atEnd = cacheListing(cache)
    if firstBuilt.ok && afterFirst.nonEmpty && atEnd == afterFirst then pass += 1
    else
      fail += 1
      println(s"FAIL the suite's shared cache after the whole run, against what its first build left (exit code ${firstBuilt.status}): ${moved(afterFirst, atEnd)}")
    println(s"native-build zig's caches: the shared cache ${afterFirst.values.sum} bytes in ${afterFirst.size} files after the first build and " +
      s"${atEnd.values.sum} in ${atEnd.size} after the run")
    done()

  if strandHas("programs") then
    val keptHello = out.resolve("kept-hello")
    for (name, main, expected) <- programs do
      val built = if name == "hello" then buildBy(keeping("hello", keptHello), name, main) else build(name, main)
      if !built.ok then
        fail += 1
        println(s"FAIL $name: the build exited ${built.status}")
        built.out.linesIterator.take(20).foreach(println)
      else if runs(name, expected) then pass += 1
      else fail += 1

    // The pinned case's lowered listing and modules by teq's own null-check rule, each against the reference rule's
    // (`teq nir --lower`, and with --null-guards=scala-native), judged by tests/support/null_guards.py as
    // tests/native-link.scala judges its programs' (`guardsOf`, held to its planted verdicts first): its `reorder`
    // brings a null pointer's slow path forward, an added check in a catch case being the first use of the path
    // outside any `try`.
    val judgeWrong = guardsSelfTest()._3
    val nullguardPath = (Paths.get("tests/native-build/fixtures/nullguard").toAbsolutePath.toString :: jars).mkString(separator)
    def lowerNullguard(more: String*) =
      Sh((List(teq, "nir", "--lower", "nullguard", nullguardPath, "--triple=x86_64-unknown-linux-musl") ++ more)*).timeout(300).check(false).run()
    val lowered = out.resolve("nullguard-lowered")
    Sh.remove(lowered)
    val (referenceDir, ownDir) = (lowered.resolve("reference"), lowered.resolve("own"))
    val referenceListing = lowerNullguard(ScalaNativeGuards)
    val ownListing = lowerNullguard()
    val referenceModules = lowerNullguard(ScalaNativeGuards, s"--ll=$referenceDir")
    val ownModules = lowerNullguard(s"--ll=$ownDir")
    def classified(reference: Path, own: Path) =
      Sh("python3", "-I", "-B", "tests/support/null_guards.py", reference.toString, own.toString).mergeErr.timeout(600).check(false).run()
    val modules = (files(referenceDir) ++ files(ownDir)).map(_.getFileName.toString).distinct.length
    val judged = List(
      ("its lowered listing", classified(referenceListing.outFile, ownListing.outFile), ownListing, 1),
      ("its modules", classified(referenceDir, ownDir), ownModules, modules))
    val verdicts = judged.map((what, ran, written, listings) => (what, ran, guardsOf(written.status, written.timedOut, ran.status, ran.timedOut, ran.out, listings)))
    val referenceFailed = List("listing" -> referenceListing, "modules" -> referenceModules).collect {
      case (what, ran) if !ran.ok || ran.timedOut => s"the reference rule's $what: exit code ${ran.status}${if ran.timedOut then ", timed out" else ""}" }
    val problems = judgeWrong ++ referenceFailed ++ verdicts.collect { case (what, _, Left(why)) => s"$what: ${why.mkString("; ")}" }
    if problems.isEmpty then
      pass += 1
      val said = verdicts.collect { case (what, _, Right(g)) => s"$what ${g.equal} equal, ${g.added} by added null checks (${g.checks} checks)" }
      println(s"native-build nullguard, teq's null-check rule against Scala Native's: ${said.mkString("; ")}; 0 otherwise")
    else
      fail += 1
      println(s"FAIL nullguard, teq's null-check rule against Scala Native's: ${problems.mkString("; ")}")
      verdicts.flatMap(_._2.lines).filter(l => l.startsWith("UNCLASSIFIED") || l.startsWith("UNRECONCILED") || l.startsWith("null_guards: cannot")).take(8).foreach(l => println(s"  $l"))

    // The commands hello world's build gave zig, as its `keeping` zig kept them, its work directory written `<work>`: the
    // class path's C code and the program's modules, each object's arguments without the build's own `-MD -MF <file>`, in
    // the lines' order.
    val work = out.resolve("work").resolve("hello")
    val helloObjects = (files(work.resolve("dependencies")) ++ files(work.resolve("generated"))).filter(_.getFileName.toString.endsWith(".o"))
    val gave = helloObjects.flatMap(keptArgs(keptHello, _)).map(a => if a.length > 3 && a(a.length - 3) == "-MD" then a.dropRight(3) else a)
      .map(_.mkString(" ").replace(work.toString, "<work>")).sorted
    val commands = Files.readString(Paths.get("tests/native-build/commands.txt")).linesIterator.toVector
    if gave == commands then pass += 1
    else
      fail += 1
      println(s"FAIL hello: the commands given zig (${gave.length}) are not Scala Native's (${commands.length}, tests/native-build/commands.txt)")
      gave.diff(commands).take(5).foreach(line => println(s"  teq: $line"))
      commands.diff(gave).take(5).foreach(line => println(s"  Scala Native: $line"))

    // A second build into the same directory compiles every C, C++ and assembly source again and no LLVM IR module, zig's
    // cache standing for those (src/native/build.rs, `through_zig_cache`): in zig's local cache in the work directory, its
    // object of each of hello world's modules (`o/<digest>/<module>.o`) is not written again and no other is made, while
    // every other object of the work directory is written again; the executable still runs. Then a zig of another build that
    // prints the same version (a script that answers `version` and hands the rest to the suite's zig, writing down each
    // command): every C source goes through it, and zig's cache, which keys an object by zig's version and not by its
    // binary (zig's src/Compilation.zig 2122), serves it the modules the suite's zig compiled.
    val irCache = out.resolve("ir-cache")
    val irWork = out.resolve("work").resolve("ir")
    def irBuild(by: Path): Sh.Result =
      val classPath = (Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString :: jars).mkString(separator)
      Sh(teq, "nir", "--link", "hello", classPath, s"--build=${out.resolve("ir-hello")}", s"--work=$irWork", s"--zig=$by", s"--zig-cache=$irCache")
        .mergeErr.timeout(900).check(false).run()
    def stamped(of: Vector[Path]): Map[Path, java.nio.file.attribute.FileTime] = of.map(f => f -> Files.getLastModifiedTime(f)).toMap
    def moduleObjects(): Map[Path, java.nio.file.attribute.FileTime] =
      val modules = files(irWork.resolve("generated")).map(_.getFileName.toString).filter(_.endsWith(".ll")).map(_.stripSuffix(".ll") + ".o").toSet
      stamped(files(irWork.resolve(localCache).resolve("o")).filter(f => modules.contains(f.getFileName.toString)))
    def sourceObjects(): Map[Path, java.nio.file.attribute.FileTime] =
      stamped(builtFiles(irWork).filter(f => f.toString.endsWith(".o") && !f.toString.endsWith(".ll.o")))
    val irFirst = irBuild(zig.get)
    val (firstModules, firstSources) = (moduleObjects(), sourceObjects())
    val irAgain = irBuild(zig.get)
    val (againModules, againSources) = (moduleObjects(), sourceObjects())
    val written = firstSources.keySet.filter(o => againSources.get(o).exists(t => !firstSources.get(o).contains(t)))
    val moduleCount = files(irWork.resolve("generated")).count(_.toString.endsWith(".ll"))
    if irFirst.ok && irAgain.ok && irAgain.out.isEmpty && moduleCount > 0 && firstModules.size == moduleCount && againModules == firstModules &&
        firstSources.nonEmpty && written == firstSources.keySet && runsAt(out.resolve("ir-hello"), "tests/native-build/hello.expected") then pass += 1
    else
      fail += 1
      println(s"FAIL hello built again: exit codes ${irFirst.status} and ${irAgain.status}, ${irAgain.out.length} characters printed; of $moduleCount modules " +
        s"${firstModules.size} in zig's cache, ${againModules.count((f, t) => !firstModules.get(f).contains(t))} written again; ${written.size} of ${firstSources.size} other objects written again")
      irAgain.out.linesIterator.take(5).foreach(println)
    val otherZig = Files.createDirectories(out.resolve("other-zig")).resolve("zig")
    val otherCommands = out.resolve("other-zig/commands")
    Files.writeString(otherZig, s"#!/bin/sh\nif [ \"$$1\" = version ]; then echo $zigVersion; exit 0; fi\nprintf '%s\\n' \"$$*\" >> ${shellQuoted(otherCommands.toString)}\n" +
      s"exec ${shellQuoted(zig.get.toString)} \"$$@\"\n")
    Sh("chmod", "755", otherZig.toString).run()
    val byOther = irBuild(otherZig)
    val otherModules = moduleObjects()
    val commandsRun = if Files.isRegularFile(otherCommands) then Files.readString(otherCommands).linesIterator.toVector else Vector.empty
    val sourcesThrough = firstSources.keySet.count(o => commandsRun.exists(_.contains(s" -o $o ")))
    if byOther.ok && otherModules == firstModules && sourcesThrough == firstSources.size && runsAt(out.resolve("ir-hello"), "tests/native-build/hello.expected") then pass += 1
    else
      fail += 1
      println(s"FAIL a zig of another build that prints the same version: exit code ${byOther.status}; of ${firstModules.size} modules' objects in zig's cache " +
        s"${otherModules.count((f, t) => !firstModules.get(f).contains(t))} written again, ${otherModules.size - firstModules.size} more; $sourcesThrough of ${firstSources.size} sources through it")
      byOther.out.linesIterator.take(5).foreach(println)

  if strandHas("probe") then
    // A module renamed (tests/native-build/fixtures/renamed): a directory's module is named by the directory's hash,
    // and of directories of one hash the first in their bytes' order keeps it, the next taking `<hash>-1`, ...
    // (src/native/codegen.rs, `module_names`). renamed.First reaches B alone (AaBB/), whose module is then the hash's;
    // renamed.Second reaches A as well (AaAa/, of the same hash, sorting before AaBB): A's module takes the hash's name
    // and B's becomes `<hash>-1`. Second built into First's work directory gives the module under the hash's name, now
    // A's, A's object, and leaves what a fresh build of Second there leaves, every module, object and the executable,
    // which prints what scalac's JVM build prints. First built there once more makes B's module the hash's again: the
    // files of Second's `<hash>-1`, which First does not make, are left in the work directory and never linked, its
    // executable a fresh First's.
    {
      val renamedWork = out.resolve("work").resolve("renamed")
      val generated = renamedWork.resolve("generated")
      val hashed = "1f0080"
      def digest(file: Path): String = java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(file)))
      def left(): Map[String, String] =
        files(generated).filter(f => f.toString.endsWith(".ll") || f.toString.endsWith(".ll.o")).map(f => generated.relativize(f).toString -> digest(f)).toMap ++
          Option.when(Files.isRegularFile(out.resolve("renamed")))("<executable>" -> digest(out.resolve("renamed")))
      def within(made: Map[String, String], fresh: Map[String, String]): Boolean = fresh.nonEmpty && fresh.forall((k, v) => made.get(k).contains(v))
      def renamedRuns(expected: String): Boolean = runs("renamed", s"tests/native-build/renamed-$expected.expected")
      val first = build("renamed", "renamed.First")
      val firstRan = first.ok && renamedRuns("first")
      val byFirst = left()
      val second = build("renamed", "renamed.Second")
      val secondRan = second.ok && renamedRuns("second")
      val bySecond = left()
      val recompiled = byFirst.contains(s"$hashed.ll.o") && bySecond.contains(s"$hashed.ll.o") && byFirst(s"$hashed.ll.o") != bySecond(s"$hashed.ll.o")
      val again = build("renamed", "renamed.First")
      val againRan = again.ok && renamedRuns("first")
      val byAgain = left()
      Sh.remove(renamedWork)
      val freshSecond = build("renamed", "renamed.Second")
      val byFreshSecond = left()
      Sh.remove(renamedWork)
      val freshFirst = build("renamed", "renamed.First")
      val byFreshFirst = left()
      val named = byFirst.contains(s"$hashed.ll") && !byFirst.contains(s"$hashed-1.ll") && byFreshSecond.contains(s"$hashed-1.ll")
      val leftUnlinked = byAgain.contains(s"$hashed-1.ll.o") && !byFreshFirst.contains(s"$hashed-1.ll.o")
      if firstRan && secondRan && againRan && freshSecond.ok && freshFirst.ok && named && recompiled && leftUnlinked && within(bySecond, byFreshSecond) && within(byAgain, byFreshFirst) then
        pass += 1
      else
        fail += 1
        println(s"FAIL a module renamed: the builds exited ${first.status}, ${second.status}, ${again.status}, ${freshSecond.status}, ${freshFirst.status}; " +
          s"First's modules name $hashed and not $hashed-1, a fresh Second's $hashed-1: $named; $hashed.ll.o another object: $recompiled; " +
          s"Second's $hashed-1 left beside First built again: $leftUnlinked; unlike a fresh build's: " +
          (byFreshSecond.keySet.filter(k => bySecond.get(k) != byFreshSecond.get(k)) ++ byFreshFirst.keySet.filter(k => byAgain.get(k) != byFreshFirst.get(k))).toVector.sorted.take(5).mkString(", "))
    }

    // A build into a work directory that held another prints what a fresh build prints: a C file of the class path that
    // includes a header of a --lib-dir and prints it with a define of its library's descriptor and whether the build
    // links with threads, built again into the same directory after each change, its executable printing the value the
    // change gives. The review's case (the header changed); a header added to the first --lib-dir, which shadows the
    // second's; that header removed; a dangling link at the header's name in the first --lib-dir, then its target made
    // (these two zig's own cache does not see: its manifest names the files a compilation read and none of the places
    // looked in before them, so they fail the day zig takes an object of its cache for a compilation the build gives a
    // dependency file); a define the command alone changes (threads, the library unchanged); the descriptor's define
    // changed (the library's own change, unpacked again); the same work directory shared with another program, built in
    // between. zig's cache (--zig-cache) is the same in every build here, a fresh work directory over it among them.
    val probe = out.resolve("probe")
    val code = probe.resolve("code/scala-native")
    for dir <- List(code, probe.resolve("first/include"), probe.resolve("second/include")) do Files.createDirectories(dir)
    Files.writeString(code.resolve("probe.c"), "#include <stdio.h>\n#include <probe_value.h>\n#ifdef SCALANATIVE_MULTITHREADING_ENABLED\n#define PROBE_THREADS 1\n#else\n#define PROBE_THREADS 0\n#endif\n__attribute__((constructor)) static void probe(void) {\n    printf(\"header=%d define=%d threads=%d\\n\", PROBE_VALUE, PROBE_DEFINE, PROBE_THREADS);\n    fflush(stdout);\n}\n")
    def descriptor(define: Int, options: String = ""): Unit =
      Files.writeString(code.resolve("scala-native.properties"), s"preprocessor.defines = PROBE_DEFINE=$define\n" + (if options.isEmpty then "" else s"compile.c.options = $options\n"))
    def header(where: String, value: Option[Int]): Unit =
      val file = probe.resolve(s"$where/include/probe_value.h")
      value match
        case Some(v) => Files.writeString(file, s"#define PROBE_VALUE $v\n")
        case None => Files.deleteIfExists(file)
    def probeBuild(dir: String, threads: Boolean): Sh.Result =
      val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, probe.resolve("code").toString) ++ jars).mkString(separator)
      Sh((List(teq, "nir", "--link", "hello", classPath, s"--build=${probe.resolve("hello")}", s"--work=${probe.resolve(dir)}", s"--zig=${zig.get}",
        s"--zig-cache=$cache", s"--lib-dir=${probe.resolve("first")}", s"--lib-dir=${probe.resolve("second")}") ++ Option.when(threads)("--threads=true"))*)
        .mergeErr.timeout(900).check(false).run()
    var probedCells = Vector.empty[String]
    def probed(label: String, dir: String, value: Int, define: Int, threads: Boolean = false): Unit =
      val built = probeBuild(dir, threads)
      val ran = if built.ok then Some(Sh(probe.resolve("hello").toString).timeout(300).check(false).run()) else None
      val expected = s"header=$value define=$define threads=${if threads then 1 else 0}\n$helloLine"
      probedCells :+= label
      if ran.exists(r => printedExactly(r, expected.getBytes(java.nio.charset.StandardCharsets.UTF_8))) then pass += 1
      else
        fail += 1
        println(s"FAIL $label: the build exited ${built.status}, the executable printed ${ran.map(_.outAll().take(80)).getOrElse("nothing")}, not ${expected.take(30)}")
        built.out.linesIterator.take(10).foreach(println)
    descriptor(1)
    header("second", Some(1))
    probed("a header of a --lib-dir, first build", "work", 1, 1)
    header("second", Some(2))
    probed("the header changed, the same work directory", "work", 2, 1)
    probed("the header changed, a fresh work directory over the same zig cache", "fresh", 2, 1)
    header("first", Some(3))
    probed("a header added to the first --lib-dir, shadowing the second's", "work", 3, 1)
    header("first", None)
    probed("the shadowing header removed", "work", 2, 1)
    val dangling = probe.resolve("first/include/probe_value.h")
    Sh("ln", "-s", "probe_target.h", dangling.toString).run()
    probed("a dangling link at the header's name in the first --lib-dir", "work", 2, 1)
    Files.writeString(probe.resolve("first/include/probe_target.h"), "#define PROBE_VALUE 4\n")
    probed("the dangling link's target made", "work", 4, 1)
    Files.delete(dangling)
    Files.delete(probe.resolve("first/include/probe_target.h"))
    probed("a define the command alone changes (threads)", "work", 2, 1, threads = true)
    probed("that define gone again", "work", 2, 1)
    descriptor(2)
    probed("the define changed in the library's descriptor", "work", 2, 2)
    val (other, otherMain, otherExpected) = programs(1)
    val otherClassPath = (Paths.get(s"tests/native-build/fixtures/$other").toAbsolutePath.toString :: jars).mkString(separator)
    val between = Sh(teq, "nir", "--link", otherMain, otherClassPath, s"--build=${probe.resolve(other)}", s"--work=${probe.resolve("work")}",
      s"--zig=${zig.get}", s"--zig-cache=$cache").mergeErr.timeout(900).check(false).run()
    val betweenRan = if between.ok then Some(Sh(probe.resolve(other).toString).timeout(300).check(false).run()) else None
    if betweenRan.exists(r => printedExactly(r, Files.readAllBytes(Paths.get(otherExpected)))) then pass += 1
    else
      fail += 1
      println(s"FAIL $other built into the probe's work directory: the build exited ${between.status}")
    probed("the work directory shared with another program", "work", 2, 2)

    // A header changed while its library is still compiling (the cut check's item 1): a library of eighty C files beside
    // `a0rc.c`, which prints what `rc.h` of a --lib-dir defines. The deterministic twin: zig a script that runs the
    // suite's zig and rewrites rc.h to 2 as soon as the compilation of a0rc.c returns; built with it (printing 1, the
    // header it compiled), then with the suite's zig into the same work directory: 2. The real edit: a build in the
    // background, rc.h rewritten to 3 as soon as a0rc.c's object is there; then built again into the same directory: 3.
    val rcRoot = out.resolve("rc")
    val rcCode = Files.createDirectories(rcRoot.resolve("lib/scala-native"))
    val rcInclude = Files.createDirectories(rcRoot.resolve("L/include"))
    Files.writeString(rcCode.resolve("a0rc.c"), "#include <stdio.h>\n#include <rc.h>\n__attribute__((constructor)) static void rc_says(void) {\n" +
      "    printf(\"rc=%d\\n\", RC);\n    fflush(stdout);\n}\n")
    for k <- 1 to 80 do Files.writeString(rcCode.resolve(s"f$k.c"), s"#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\nint rc_f$k(void) { return (int)strlen(\"$k\"); }\n")
    def rcHeader(value: Int): Unit = Files.writeString(rcInclude.resolve("rc.h"), s"#define RC $value\n")
    def rcCmd(by: Path, work: String): Sh.Cmd =
      val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, rcRoot.resolve("lib").toString) ++ jars).mkString(separator)
      Sh(teq, "nir", "--link", "hello", classPath, s"--build=${rcRoot.resolve("hello")}", s"--work=${rcRoot.resolve(work)}", s"--zig=$by", s"--zig-cache=$cache",
        s"--lib-dir=${rcRoot.resolve("L")}").mergeErr.timeout(900).check(false)
    def rcPrinted(built: Sh.Result): String =
      if built.ok then Sh(rcRoot.resolve("hello").toString).timeout(300).check(false).run().out.linesIterator.find(_.startsWith("rc=")).getOrElse("nothing") else s"exit ${built.status}"
    val rewriting = rcRoot.resolve("rewriting-zig")
    Files.writeString(rewriting, s"#!/bin/sh\n${shellQuoted(zig.get.toString)} \"$$@\"; c=$$?\ncase \"$$*\" in *a0rc.c*) printf '#define RC 2\\n' > ${shellQuoted(rcInclude.resolve("rc.h").toString)};; esac\nexit $$c\n")
    Sh("chmod", "755", rewriting.toString).run()
    rcHeader(1)
    val twinFirst = rcPrinted(rcCmd(rewriting, "twin").run())
    val twinAgain = rcPrinted(rcCmd(zig.get, "twin").run())
    if twinFirst == "rc=1" && twinAgain == "rc=2" then pass += 1
    else
      fail += 1
      println(s"FAIL a header changed as its compilation returns: the first build printed $twinFirst (rc=1 wanted), the next $twinAgain (rc=2 wanted)")
    rcHeader(1)
    val rcObject = rcRoot.resolve("edit/dependencies/lib-0/scala-native/a0rc.c.o")
    val editing = rcCmd(zig.get, "edit").spawn()
    var rcWaited = 0
    while rcWaited < 3000 && editing.alive && !scala.util.Try(Files.isRegularFile(rcObject)).getOrElse(false) do
      Thread.sleep(20)
      rcWaited += 1
    val editedWhileBuilding = editing.alive
    rcHeader(3)
    val editFirst = rcPrinted(editing.await())
    val editAgain = rcPrinted(rcCmd(zig.get, "edit").run())
    if editedWhileBuilding && editAgain == "rc=3" then pass += 1
    else
      fail += 1
      println(s"FAIL a header changed while its library compiles: changed while building $editedWhileBuilding; the build printed $editFirst, the next $editAgain (rc=3 wanted)")

  if strandHas("table") then
    val table = out.resolve("table")
    val tableLib = Files.createDirectories(table.resolve("lib/include"))
    Files.writeString(tableLib.resolve("table_value.h"), "#define TABLE_VALUE 1\n")
    val cellDirs = cells.indices.map(k => table.resolve(s"cells/c$k"))
    for (k, dir) <- cells.indices.zip(cellDirs) do
      val code = Files.createDirectories(dir.resolve("scala-native"))
      Files.writeString(code.resolve("cell.c"), s"#include <stdio.h>\n#include <table_value.h>\n__attribute__((constructor)) static void cell_$k(void) {\n" +
        s"    printf(\"cell $k: header=%d before=%d between=%d after=%d\\n\", TABLE_VALUE, BEFORE, BETWEEN, AFTER);\n    fflush(stdout);\n}\n")
    def describeCells(withOptions: Boolean): Unit =
      for ((_, a, b), k) <- cells.zipWithIndex do
        val options = Vector(s"-DBEFORE=$k") ++ (if withOptions then a else Vector.empty) ++ Vector(s"-DBETWEEN=$k") ++ (if withOptions then b else Vector.empty) ++ Vector(s"-DAFTER=$k")
        Files.writeString(cellDirs(k).resolve("scala-native/scala-native.properties"), s"compile.c.options = ${options.mkString(",")}\n")
    // The class path in an argument file: as many entries as cells pass the system's bound of one argument.
    val tableArgs = table.resolve("class-path.args")
    Files.writeString(tableArgs, ((Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString +: cellDirs.map(_.toString)) ++ jars).mkString(separator) + "\n")
    val tableWork = table.resolve("work")
    def tableBuild(work: Path, by: Path): Sh.Result =
      Sh(teq, "nir", "--link", "hello", s"@$tableArgs", s"--build=${table.resolve("hello")}", s"--work=$work", s"--zig=$by", s"--zig-cache=$cache",
        s"--lib-dir=${table.resolve("lib")}").mergeErr.timeout(2400).check(false).run()
    def keptOf(keptInto: Path): Map[String, Kept] =
      cells.indices.map { k =>
        val obj = tableWork.resolve("dependencies").resolve(s"c$k-$k").resolve("scala-native/cell.c.o")
        cells(k)._1 -> Kept(readIf(obj), keptArgs(keptInto, obj))
      }.toMap
    def tableRun(): Option[Sh.Result] = Some(Sh(table.resolve("hello").toString).timeout(300).check(false).run())
    val startedTable = System.nanoTime()
    describeCells(false)
    val controlBuilt = tableBuild(tableWork, keeping("table-control", table.resolve("kept-control")))
    val controlRan = if controlBuilt.ok then tableRun() else None
    val controlKept = keptOf(table.resolve("kept-control"))
    val controlLeft = leftIn(tableWork)
    describeCells(true)
    val cellsBuilt = tableBuild(tableWork, keeping("table-cells", table.resolve("kept-cells")))
    val cellsRan = if cellsBuilt.ok then tableRun() else None
    val cellsKept = keptOf(table.resolve("kept-cells"))
    val strayFiles = leftIn(tableWork).toVector.filter((f, d) => !controlLeft.get(f).contains(d)).map(_._1).sorted
    val manifest = cells.map((name, a, b) => name -> (a ++ b))
    val faults = tableFaults(manifest, controlKept, cellsKept, cellsBuilt, strayFiles) ++
      (if controlBuilt.ok && controlRan.exists(_.ok) then Vector.empty else Vector(s"the control's build exited ${controlBuilt.status}")) ++
      (if controlRan.zip(cellsRan).exists((c, k) => printedExactly(k, Files.readAllBytes(c.outFile))) then Vector.empty else Vector("the cells' executable does not print what the control's does"))
    for named <- Vector("-MP", "-MT,hash#target") if !manifest.exists(_._1 == named) do
      fail += 1
      println(s"FAIL the table: no cell $named")
    if faults.isEmpty then pass += 1
    else
      fail += 1
      println(s"FAIL the table: ${faults.length} faults over ${cells.length} cells")
      faults.take(12).foreach(f => println(s"  $f"))
      (controlBuilt.out.linesIterator ++ cellsBuilt.out.linesIterator).filter(_.nonEmpty).take(10).foreach(println)
    Files.writeString(tableLib.resolve("table_value.h"), "#define TABLE_VALUE 2\n")
    val againBuilt = tableBuild(tableWork, zig.get)
    val againRan = if againBuilt.ok then tableRun() else None
    val freshBuilt = tableBuild(table.resolve("fresh"), zig.get)
    val freshRan = if freshBuilt.ok then tableRun() else None
    val headerTwo = cells.indices.map(k => s"cell $k: header=2 ").toSet
    val freshLines = freshRan.map(r => r.outAll().linesIterator.filter(_.startsWith("cell ")).toVector).getOrElse(Vector.empty)
    if againRan.zip(freshRan).exists((a, f) => printedExactly(a, Files.readAllBytes(f.outFile))) && freshLines.length == cells.length &&
        freshLines.forall(l => headerTwo.exists(l.startsWith)) then pass += 1
    else
      fail += 1
      println(s"FAIL the table, its header changed: built again exited ${againBuilt.status}, fresh ${freshBuilt.status}; ${freshLines.length} cells printed")
      againBuilt.out.linesIterator.take(10).foreach(println)
    println(s"native-build table: ${singleCells.length} options alone and ${pairCells.length} pairs, ${cells.length} cells, ${faults.length} faults, " +
      s"${(System.nanoTime() - startedTable) / 1000000000} s")

  if strandHas("misc") then
    // Names zig is given whatever they hold: a C file, the header it includes, the --lib-dir that holds the header and the
    // work directory, each named with `#`, `$`, a blank, `:`, a backslash and a tab. Built, the header changed and built
    // again, built fresh into another such directory: each prints its header's value.
    val odd = "#$ :\\\t"
    val namesRoot = out.resolve("names")
    val namedCode = Files.createDirectories(namesRoot.resolve("code/scala-native"))
    val namedInclude = Files.createDirectories(namesRoot.resolve(s"inc$odd").resolve("include"))
    Files.writeString(namedCode.resolve(s"n$odd.c"), s"#include <stdio.h>\n#include <h$odd.h>\n__attribute__((constructor)) static void names(void) {\n" +
      "    printf(\"names header=%d\\n\", NAMES_VALUE);\n    fflush(stdout);\n}\n")
    def namedBuild(work: String): Sh.Result =
      val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, namesRoot.resolve("code").toString) ++ jars).mkString(separator)
      Sh(teq, "nir", "--link", "hello", classPath, s"--build=${namesRoot.resolve("hello")}", s"--work=${namesRoot.resolve(work)}", s"--zig=${zig.get}",
        s"--zig-cache=$cache", s"--lib-dir=${namesRoot.resolve(s"inc$odd")}").mergeErr.timeout(900).check(false).run()
    for (label, work, value) <- List(("names: built", s"work$odd", 1), ("names: the header changed, built again", s"work$odd", 2), ("names: built fresh", s"fresh$odd", 2)) do
      Files.writeString(namedInclude.resolve(s"h$odd.h"), s"#define NAMES_VALUE $value\n")
      val built = namedBuild(work)
      val ran = if built.ok then Some(Sh(namesRoot.resolve("hello").toString).timeout(300).check(false).run()) else None
      if ran.exists(r => printedExactly(r, s"names header=$value\n$helloLine".getBytes("UTF-8"))) then pass += 1
      else
        fail += 1
        println(s"FAIL $label: the build exited ${built.status}, the executable printed ${ran.map(_.outAll().take(80)).getOrElse("nothing")}")
        built.out.linesIterator.take(10).foreach(println)

    // The caller's environment: zig is given its two cache variables, and SOURCE_DATE_EPOCH where the caller has it, and
    // nothing else of the caller's (src/native/build.rs, `PASSED_ENVIRONMENT`); `zig version` is given nothing. A C file
    // that includes a header of its --lib-dir (`table_value.h`, 2) and prints it, built under each of the variables clang's driver or zig reads
    // (CPATH, C_INCLUDE_PATH and CPLUS_INCLUDE_PATH naming a directory whose header would stand before the table's,
    // CCC_OVERRIDE_OPTIONS adding a target, ZIG_LIB_DIR naming no directory, ZIG_LIBC a file of no C library,
    // DEPENDENCIES_OUTPUT and SUNPRO_DEPENDENCIES naming a dependency file): its object and arguments the plain build's,
    // zig given the plain build's environment, no file of theirs written, the executable printing that header.
    // Then every variable zig 0.16.0, clang 21.1.0's driver and LLVM's Support read (below, each list with its source),
    // GCC's dependency variables, CFLAGS and LC_ALL, all at once: every object of the work directory and the executable
    // byte for byte the plain build's. A C file printing `__DATE__ __TIME__` built under SOURCE_DATE_EPOCH, then under another value into
    // the same directory: zig is given it and the executable prints the new date, and once more under that value, the
    // same.
    val envRoot = out.resolve("env")
    val envCode = Files.createDirectories(envRoot.resolve("code/scala-native"))
    val shadow = Files.createDirectories(envRoot.resolve("shadow"))
    Files.writeString(shadow.resolve("table_value.h"), "#define TABLE_VALUE 9\n")
    Files.writeString(envCode.resolve("env.c"), "#include <stdio.h>\n#include <table_value.h>\n__attribute__((constructor)) static void env(void) {\n" +
      "    printf(\"env header=%d\\n\", TABLE_VALUE);\n    fflush(stdout);\n}\n")
    Files.writeString(Files.createDirectories(envRoot.resolve("lib/include")).resolve("table_value.h"), "#define TABLE_VALUE 2\n")
    val zigLibc = envRoot.resolve("libc.txt")
    Files.writeString(zigLibc, s"include_dir=${envRoot.resolve("nowhere")}\nsys_include_dir=${envRoot.resolve("nowhere")}\ncrt_dir=${envRoot.resolve("nowhere")}\n" +
      "msvc_lib_dir=\nkernel32_lib_dir=\ngcc_dir=\n")
    val envWork = envRoot.resolve("work")
    val envObject = envWork.resolve("dependencies/code-0/scala-native/env.c.o")
    def envBuild(keptInto: Path, vars: Seq[(String, String)]): Sh.Result =
      val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, envRoot.resolve("code").toString) ++ jars).mkString(separator)
      Sh(teq, "nir", "--link", "hello", classPath, s"--build=${envRoot.resolve("hello")}", s"--work=$envWork",
        s"--zig=${keeping(keptInto.getFileName.toString, keptInto)}", s"--zig-cache=$cache", s"--lib-dir=${envRoot.resolve("lib")}")
        .withEnv(vars*).mergeErr.timeout(900).check(false).run()
    def envKept(keptInto: Path): Kept = Kept(readIf(envObject), keptArgs(keptInto, envObject))
    def digests(): Map[String, String] =
      (builtFiles(envWork).filter(_.toString.endsWith(".o")) :+ envRoot.resolve("hello")).filter(Files.isRegularFile(_))
        .map(f => envRoot.relativize(f).toString -> java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(f)))).toMap
    val callerEpoch = Option(System.getenv("SOURCE_DATE_EPOCH")).filter(_.nonEmpty)
    val zigEnvironment = Set("ZIG_GLOBAL_CACHE_DIR", "ZIG_LOCAL_CACHE_DIR") ++ callerEpoch.map(_ => "SOURCE_DATE_EPOCH")
    val envPlain = envBuild(envRoot.resolve("kept-plain"), Nil)
    val plainKept = envKept(envRoot.resolve("kept-plain"))
    val plainGiven = keptEnvironment(envRoot.resolve("kept-plain"), envObject)
    val plainDigests = digests()
    if !plainGiven.exists(_.keySet == zigEnvironment) then
      fail += 1
      println(s"FAIL the environment: zig was given ${plainGiven.map(_.keySet.toVector.sorted.mkString(", ")).getOrElse("nothing kept")}, not ${zigEnvironment.toVector.sorted.mkString(", ")}")
    val envCases = List(
      "CPATH" -> Seq("CPATH" -> shadow.toString),
      "C_INCLUDE_PATH" -> Seq("C_INCLUDE_PATH" -> shadow.toString),
      "CPLUS_INCLUDE_PATH" -> Seq("CPLUS_INCLUDE_PATH" -> shadow.toString),
      "CCC_OVERRIDE_OPTIONS adding a target" -> Seq("CCC_OVERRIDE_OPTIONS" -> "# +-MT +injected"),
      "ZIG_LIB_DIR" -> Seq("ZIG_LIB_DIR" -> envRoot.resolve("nowhere").toString),
      "ZIG_LIBC" -> Seq("ZIG_LIBC" -> zigLibc.toString),
      "DEPENDENCIES_OUTPUT and SUNPRO_DEPENDENCIES" -> Seq("DEPENDENCIES_OUTPUT" -> s"${envRoot.resolve("extra.d")} injected",
        "SUNPRO_DEPENDENCIES" -> envRoot.resolve("sunpro.d").toString))
    for ((label, vars), n) <- envCases.zipWithIndex do
      val keptInto = envRoot.resolve(s"kept-$n")
      val built = envBuild(keptInto, vars)
      val kept = envKept(keptInto)
      val received = keptEnvironment(keptInto, envObject)
      val ran = if built.ok then Some(Sh(envRoot.resolve("hello").toString).timeout(300).check(false).run()) else None
      val equal = envPlain.ok && same(kept.obj, plainKept.obj) && kept.args.isDefined && kept.args == plainKept.args
      val written = List("extra.d", "sunpro.d").filter(f => Files.exists(envRoot.resolve(f)))
      if equal && received.isDefined && received == plainGiven && written.isEmpty && ran.exists(r => printedExactly(r, s"env header=2\n$helloLine".getBytes("UTF-8"))) then pass += 1
      else
        fail += 1
        println(s"FAIL the environment, $label: the build exited ${built.status}; the object and arguments the plain build's: $equal; " +
          s"zig given ${received.map(_.keySet.toVector.sorted.mkString(", ")).getOrElse("nothing kept")}; written: ${written.mkString(", ")}")
        built.out.linesIterator.take(10).foreach(println)
    // Every variable zig 0.16.0 reads, from its own list (lib/std/zig.zig beside the zig binary, `EnvVar`); those clang
    // 21.1.0's driver reads, by file and line of its source (clang/lib/Driver/ToolChains/Clang.cpp 1094-1102, 1165, 2743,
    // 3874; ToolChains/CommonArgs.cpp 464, 2740, 2897; Driver.cpp 703, 1176, 1397, 1641, 6168; ToolChain.cpp 122;
    // clang/lib/Frontend/CompilerInvocation.cpp 2678; tools/driver/driver.cpp, as zig carries it in src/zig_clang_driver.cpp,
    // 136-187, 300, 389), but SOURCE_DATE_EPOCH, which zig is given on purpose; those of LLVM's Support it runs on
    // (llvm/lib/Support/Unix/Path.inc 171, 371, 1352, 1406, 1428, 1444); GCC's dependency variables, CFLAGS and LC_ALL.
    // Each set to an empty directory of its own, whatever it names.
    val zigNames =
      val lines = Files.readString(zig.get.toRealPath().getParent.resolve("lib/std/zig.zig")).linesIterator.toVector
      lines.dropWhile(!_.trim.startsWith("pub const EnvVar = enum {")).drop(1).takeWhile(l => !l.trim.startsWith("pub fn") && l.trim != "};")
        .map(_.trim).filter(l => l.nonEmpty && !l.startsWith("//")).map(_.stripSuffix(","))
    val clangNames = Vector("CPATH", "C_INCLUDE_PATH", "CPLUS_INCLUDE_PATH", "OBJC_INCLUDE_PATH", "OBJCPLUS_INCLUDE_PATH", "AS_SECURE_LOG_FILE",
      "CLANG_MODULE_CACHE_PATH", "LIBRARY_PATH", "OBJECT_MODE", "CLANG_CONFIG_PATH", "CLANG_NO_DEFAULT_CONFIG", "COMPILER_PATH",
      "CLANG_CRASH_DIAGNOSTICS_DIR", "CLANG_TOOLCHAIN_PROGRAM_TIMEOUT", "NO_COLOR", "CC_PRINT_OPTIONS", "CC_PRINT_OPTIONS_FILE", "CC_PRINT_HEADERS",
      "CC_PRINT_HEADERS_FILE", "CC_PRINT_HEADERS_FORMAT", "CC_PRINT_HEADERS_FILTERING", "CC_LOG_DIAGNOSTICS", "CC_LOG_DIAGNOSTICS_FILE",
      "CC_PRINT_PROC_STAT", "CC_PRINT_PROC_STAT_FILE", "CC_PRINT_INTERNAL_STAT", "CC_PRINT_INTERNAL_STAT_FILE", "CCC_OVERRIDE_OPTIONS",
      "FORCE_CLANG_DIAGNOSTICS_CRASH")
    val supportNames = Vector("PATH", "PWD", "HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "TMPDIR", "TMP", "TEMP", "TEMPDIR")
    val everything = (zigNames ++ clangNames ++ supportNames ++ Vector("DEPENDENCIES_OUTPUT", "SUNPRO_DEPENDENCIES", "CFLAGS", "LC_ALL")).distinct
      .map(_ -> Files.createDirectories(envRoot.resolve("elsewhere")).toString)
    if zigNames.length < 20 || !zigNames.contains("ZIG_LIB_DIR") then
      fail += 1
      println(s"FAIL the environment: zig's list read as ${zigNames.length} names (${zigNames.take(5).mkString(", ")})")
    val allBuilt = envBuild(envRoot.resolve("kept-all"), everything)
    val allDigests = digests()
    val versionGiven = environmentIn(envRoot.resolve("kept-all/version.env"))
    val unlike = (plainDigests.keySet ++ allDigests.keySet).toVector.sorted.filter(f => plainDigests.get(f) != allDigests.get(f))
    if allBuilt.ok && plainDigests.size > 2 && unlike.isEmpty && versionGiven.contains(Map.empty) &&
        keptEnvironment(envRoot.resolve("kept-all"), envObject) == plainGiven then pass += 1
    else
      fail += 1
      println(s"FAIL the environment, the ${everything.length} variables zig, clang's driver and LLVM read at once: the build exited ${allBuilt.status}; of ${plainDigests.size} files " +
        s"${unlike.length} not the plain build's (${unlike.take(3).mkString(", ")}); `zig version` given ${versionGiven.map(_.keySet.toVector.sorted.mkString(", ")).getOrElse("nothing kept")}")
      allBuilt.out.linesIterator.take(10).foreach(println)
    val dated = Files.createDirectories(envRoot.resolve("dated/scala-native"))
    Files.writeString(dated.resolve("dated.c"), "#include <stdio.h>\n__attribute__((constructor)) static void dated(void) {\n" +
      "    printf(\"dated %s %s\\n\", __DATE__, __TIME__);\n    fflush(stdout);\n}\n")
    val datedObject = envRoot.resolve("dated-work/dependencies/dated-0/scala-native/dated.c.o")
    def datedBuild(epoch: String): (Sh.Result, Option[Sh.Result], Option[Map[String, String]]) =
      val keptInto = envRoot.resolve(s"kept-dated-$epoch")
      val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, envRoot.resolve("dated").toString) ++ jars).mkString(separator)
      val built = Sh(teq, "nir", "--link", "hello", classPath, s"--build=${envRoot.resolve("dated-hello")}", s"--work=${envRoot.resolve("dated-work")}",
        s"--zig=${keeping(keptInto.getFileName.toString, keptInto)}", s"--zig-cache=$cache").withEnv("SOURCE_DATE_EPOCH" -> epoch).mergeErr.timeout(900).check(false).run()
      (built, if built.ok then Some(Sh(envRoot.resolve("dated-hello").toString).timeout(300).check(false).run()) else None, keptEnvironment(keptInto, datedObject))
    val (_, ranEpoch, _) = datedBuild("0")
    val (_, ranYear, givenYear) = datedBuild("31536000")
    val (_, ranAgain, _) = datedBuild("31536000")
    if ranEpoch.exists(r => printedExactly(r, s"dated Jan  1 1970 00:00:00\n$helloLine".getBytes("UTF-8"))) &&
        ranYear.exists(r => printedExactly(r, s"dated Jan  1 1971 00:00:00\n$helloLine".getBytes("UTF-8"))) &&
        givenYear.exists(_.get("SOURCE_DATE_EPOCH").contains("31536000")) &&
        ranAgain.exists(r => printedExactly(r, s"dated Jan  1 1971 00:00:00\n$helloLine".getBytes("UTF-8"))) then pass += 1
    else
      fail += 1
      println(s"FAIL SOURCE_DATE_EPOCH: printed ${ranEpoch.map(_.outAll().take(40))}, then ${ranYear.map(_.outAll().take(40))}, then ${ranAgain.map(_.outAll().take(40))}; " +
        s"zig given ${givenYear.flatMap(_.get("SOURCE_DATE_EPOCH"))}")

  if strandHas("cases") then
    // The cases of tests/native-build/library-options.expected (tests/native-build/library-options.scala writes them from
    // src/native/library-options.txt and zig's option table): every case passed, the options of a library of its own
    // between two defines, all of them in one build, the C file `pass.c` (or the C++ file `pass.cpp`) printing both, its
    // `keeping` zig given the defines and the options between them as they are, the executable printing every library's
    // line; every case refused, a build of its own, refused naming the library and the case's first option, and nothing
    // compiled. The dropped cases are the table's above. The file has to hold each case once, a passed case of every
    // form the table keeps, a dropped one of every form it drops and the review's rows by name (`caseFaults`).
    final case class Case(answer: String, language: String, options: Vector[String])
    def casesOf(text: String): Vector[Case] = text.linesIterator.filter(l => l.nonEmpty && !l.startsWith("#"))
      .map(_.split(" ", 3)).map(f => Case(f(0), f(1), f.lift(2).getOrElse("").split(",", -1).toVector)).toVector
    val optionCases = casesOf(Files.readString(Paths.get("tests/native-build/library-options.expected")))
    val reviewRows = Vector("--include-directory=/abs/shA", "-isystem-after,/abs/shB", "-I,-MD,-DFOO=1", "-iprefix,-MD,-DFOO=1", "-Xarch_,-MD,-DFOO=1",
      "-I,-MD", "-header-include-file,-DFOO=1")
    val refuseRoot = out.resolve("refused")
    def refusalOf(c: Case): String =
      val what = if c.options.head.isEmpty then "an empty compile option" else s"the compile option ${c.options.head}"
      s"${refuseRoot.resolve("code")}: scala-native.properties: not supported on native: $what in a library's descriptor"
    /// What became of a case: of one passed, the arguments zig was given for its object and whether its line was
    /// printed; of one refused, its build's end, its lines and whether it compiled the library's object.
    final case class CaseRun(args: Option[Vector[String]], printed: Boolean, status: Int, timedOut: Boolean, lines: Vector[String], compiled: Boolean)
    def ofForm(r: Row, option: String): Boolean = if r.prefix then option.startsWith(r.spelling) && option.length > r.spelling.length else option == r.spelling
    /// The cases' verdict: the file's coverage, then the build of the passed cases (ended well within its bound), each
    /// passed case's run (zig given its options between its defines, its line printed) and each refused case's (exit 1
    /// within its bound, its refusal's line, nothing compiled), a case without its run a fault.
    def caseFaults(cases: Vector[Case], built: Sh.Result, passRuns: Vector[CaseRun], refusedRuns: Vector[CaseRun]): Vector[String] =
      val (passed, dropped, refused) = (cases.filter(_.answer == "passed"), cases.filter(_.answer == "dropped"), cases.filter(_.answer == "refused"))
      val names = cases.map(c => s"${c.answer} ${c.options.mkString(",")}")
      Vector(Option.when(cases.isEmpty)("no case"), Option.when(passed.nonEmpty && (built.timedOut || !built.ok))(s"the cases passed: their build exited ${built.status}" +
        (if built.timedOut then ", past its bound" else ""))).flatten ++
        names.diff(names.distinct).distinct.map(n => s"$n: twice") ++
        rows.filter(_.role == "kept").filterNot(r => passed.exists(c => ofForm(r, c.options.head))).map(r => s"${r.spelling}: no case passed of the form") ++
        droppedRows.filterNot(r => dropped.exists(c => ofForm(r, c.options.head))).map(r => s"${r.spelling}: no case dropped of the form") ++
        reviewRows.filterNot(n => refused.exists(_.options.mkString(",") == n)).map(n => s"$n: no case refused by that name") ++
        passed.zipWithIndex.flatMap { (c, k) =>
          val run = s"-DBEFORE=$k" +: c.options :+ s"-DAFTER=$k"
          passRuns.lift(k) match
            case None => Vector(s"${c.options.mkString(",")}: no run")
            case Some(r) => Vector(Option.when(!r.args.exists(a => a.indices.exists(i => a.slice(i, i + run.length) == run)))(s"zig was not given ${run.mkString(" ")}"),
              Option.when(!r.printed)("the executable printed no line of it")).flatten.map(f => s"${c.options.mkString(",")}: $f")
        } ++
        refused.zipWithIndex.flatMap { (c, k) =>
          refusedRuns.lift(k) match
            case None => Vector(s"${c.options.mkString(",")}: no run")
            case Some(r) => Vector(Option.when(r.timedOut)("its build ran past its bound"), Option.when(!r.timedOut && r.status != 1)(s"its build exited ${r.status}"),
              Option.when(!r.lines.contains(refusalOf(c)))("not refused naming the library and its first option"), Option.when(r.compiled)("its library compiled"))
              .flatten.map(f => s"${c.options.mkString(",")}: $f")
        }
    // The verdict's self-test: planted bad cases and runs, each of which it must find, and the good ones it must take.
    {
      Files.createDirectories(planted)
      val okBuild = planting("cases-ok", "true", 10)
      def runsOf(cases: Vector[Case]): (Vector[CaseRun], Vector[CaseRun]) =
        (cases.filter(_.answer == "passed").zipWithIndex.map((c, k) => CaseRun(Some(("cc" +: s"-DBEFORE=$k" +: c.options :+ s"-DAFTER=$k") :+ "-O0"), true, 0, false, Vector.empty, false)),
          cases.filter(_.answer == "refused").map(c => CaseRun(None, false, 1, false, Vector("x", refusalOf(c)), false)))
      val (goodPassed, goodRefused) = runsOf(optionCases)
      val faultsOf = (cases: Vector[Case], built: Sh.Result, p: Vector[CaseRun], r: Vector[CaseRun]) => caseFaults(cases, built, p, r)
      val without = (what: Case => Boolean) => optionCases.filterNot(what)
      val plants = Vector(
        "a passed case zig was not given" -> faultsOf(optionCases, okBuild, goodPassed.updated(0, goodPassed(0).copy(args = Some(Vector("cc", "-O0")))), goodRefused),
        "a passed case whose line was not printed" -> faultsOf(optionCases, okBuild, goodPassed.updated(1, goodPassed(1).copy(printed = false)), goodRefused),
        "a passed case without its run" -> faultsOf(optionCases, okBuild, goodPassed.dropRight(1), goodRefused),
        "the passed cases' build past its bound" -> faultsOf(optionCases, planting("cases-slow", "sleep 5", 1), goodPassed, goodRefused),
        "the passed cases' build failed" -> faultsOf(optionCases, planting("cases-failed", "exit 1", 10), goodPassed, goodRefused),
        "a refused case that built" -> faultsOf(optionCases, okBuild, goodPassed, goodRefused.updated(0, goodRefused(0).copy(status = 0))),
        "a refused case naming another option" -> faultsOf(optionCases, okBuild, goodPassed, goodRefused.updated(1, goodRefused(1).copy(lines = Vector(refusalOf(Case("refused", "c", Vector("-other"))))))),
        "a refused case that compiled its library" -> faultsOf(optionCases, okBuild, goodPassed, goodRefused.updated(2, goodRefused(2).copy(compiled = true))),
        "a refused case past its bound" -> faultsOf(optionCases, okBuild, goodPassed, goodRefused.updated(3, goodRefused(3).copy(timedOut = true, status = 124))),
        "a refused case without its run" -> faultsOf(optionCases, okBuild, goodPassed, goodRefused.dropRight(1)),
        "a case twice" -> faultsOf(optionCases :+ optionCases.last, okBuild, goodPassed, goodRefused),
        "an empty file" -> faultsOf(Vector.empty, okBuild, Vector.empty, Vector.empty),
        "a form the table keeps without a case" -> { val cs = without(c => c.answer == "passed" && c.options.head.startsWith("-std=")); val (p, r) = runsOf(cs); faultsOf(cs, okBuild, p, r) },
        "a form the table drops without a case" -> { val cs = without(c => c.answer == "dropped" && c.options.head == "-MP"); val (p, r) = runsOf(cs); faultsOf(cs, okBuild, p, r) },
        "a review row missing" -> { val cs = without(c => c.options.mkString(",") == "-I,-MD"); val (p, r) = runsOf(cs); faultsOf(cs, okBuild, p, r) })
      for (what, faults) <- plants if faults.isEmpty do
        fail += 1
        println(s"FAIL the cases' self-test: $what was taken")
      val goodTaken = faultsOf(optionCases, okBuild, goodPassed, goodRefused).isEmpty
      if !goodTaken then
        fail += 1
        println("FAIL the cases' self-test: the good runs were refused")
      if goodTaken && plants.forall(_._2.nonEmpty) then println(s"native-build cases self-test: ${plants.length} planted bad cases and runs found, the good ones taken")
      Sh.remove(planted)
    }
    val passedCases = optionCases.filter(_.answer == "passed")
    val refusedCases = optionCases.filter(_.answer == "refused")
    val passRoot = out.resolve("passed")
    val passDirs = passedCases.indices.map(k => passRoot.resolve(s"cases/p$k"))
    for ((c, dir), k) <- passedCases.zip(passDirs).zipWithIndex do
      val code = Files.createDirectories(dir.resolve("scala-native"))
      val options = (s"-DBEFORE=$k" +: c.options :+ s"-DAFTER=$k").mkString(",")
      val (file, property, include) = if c.language == "cpp" then ("pass.cpp", "compile.cpp.options", "cstdio") else ("pass.c", "compile.c.options", "stdio.h")
      Files.writeString(code.resolve(file), s"#include <$include>\n__attribute__((constructor)) static void pass_$k(void) {\n" +
        s"    printf(\"pass $k: before=%d after=%d\\n\", BEFORE, AFTER);\n    fflush(stdout);\n}\n")
      Files.writeString(code.resolve("scala-native.properties"), s"$property = $options\n")
    val passArgs = passRoot.resolve("class-path.args")
    Files.writeString(passArgs, ((Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString +: passDirs.map(_.toString)) ++ jars).mkString(separator) + "\n")
    val passKept = passRoot.resolve("kept")
    val passBuilt = Sh(teq, "nir", "--link", "hello", s"@$passArgs", s"--build=${passRoot.resolve("hello")}", s"--work=${passRoot.resolve("work")}",
      s"--zig=${keeping("passed", passKept)}", s"--zig-cache=$cache").mergeErr.timeout(2400).check(false).run()
    val passRan = if passBuilt.ok then Some(Sh(passRoot.resolve("hello").toString).timeout(300).check(false).run()) else None
    val passLines = passRan.map(_.outAll().linesIterator.filter(_.startsWith("pass ")).toSet).getOrElse(Set.empty)
    val passRuns = passedCases.zipWithIndex.map { (c, k) =>
      val obj = passRoot.resolve(s"work/dependencies/p$k-$k/scala-native/${if c.language == "cpp" then "pass.cpp" else "pass.c"}.o")
      CaseRun(keptArgs(passKept, obj), passLines.contains(s"pass $k: before=$k after=$k"), passBuilt.status, passBuilt.timedOut, Vector.empty, true)
    }
    Files.createDirectories(refuseRoot.resolve("code/scala-native"))
    Files.writeString(refuseRoot.resolve("code/scala-native/r.c"), "int refused_r;\n")
    val refusedObject = refuseRoot.resolve("work/dependencies/code-0/scala-native/r.c.o")
    val refusedRuns = refusedCases.map { c =>
      Files.writeString(refuseRoot.resolve("code/scala-native/scala-native.properties"), s"compile.c.options = ${c.options.mkString(",")}\n")
      val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, refuseRoot.resolve("code").toString) ++ jars).mkString(separator)
      val built = Sh(teq, "nir", "--link", "hello", classPath, s"--build=${refuseRoot.resolve("hello")}", s"--work=${refuseRoot.resolve("work")}", s"--zig=${zig.get}",
        s"--zig-cache=$cache").mergeErr.timeout(900).check(false).run()
      CaseRun(None, false, built.status, built.timedOut, built.out.linesIterator.toVector, Files.exists(refusedObject))
    }
    val caseFound = caseFaults(optionCases, passBuilt, passRuns, refusedRuns)
    if caseFound.isEmpty then pass += 1
    else
      fail += 1
      println(s"FAIL the cases: ${caseFound.length} faults over ${optionCases.length} cases")
      caseFound.take(12).foreach(f => println(s"  $f"))
      passBuilt.out.linesIterator.filter(_.nonEmpty).take(10).foreach(println)
    println(s"native-build cases: ${passedCases.length} passed in one build, ${refusedCases.length} refused, ${optionCases.length - passedCases.length - refusedCases.length} " +
      s"dropped (the table's), ${caseFound.length} faults")

  if strandHas("misc") then
    // The review's descriptors, each built in turn with a library whose C file prints FOO (the library unpacked again
    // as its descriptor changes, its object compiled again): read as Scala Native reads them
    // (src/native/descriptor.rs), each builds what its define gives; one that leaves an empty define, a malformed escape
    // and a project.gcProject neither true nor false fail, naming the library.
    val readRoot = out.resolve("read")
    Files.createDirectories(readRoot.resolve("code/scala-native"))
    Files.writeString(readRoot.resolve("code/scala-native/t.c"), "#include <stdio.h>\n#ifndef FOO\n#define FOO 0\n#endif\n" +
      "__attribute__((constructor)) static void t(void) {\n    printf(\"foo=%d\\n\", FOO);\n    fflush(stdout);\n}\n")
    val failed = s"${readRoot.resolve("code")}: scala-native.properties: "
    val bs = "\\"
    val readCases: List[(String, Array[Byte], Either[String, Int])] = List(
      ("the define", "preprocessor.defines = FOO=1\n".getBytes("UTF-8"), Right(1)),
      ("a byte that is not UTF-8 in a comment", ("# " + 0xa9.toChar + " 2024 a library\npreprocessor.defines = FOO=1\n").getBytes("ISO-8859-1"), Right(1)),
      ("a blank as the separator", "compile.c.options -DFOO=1\n".getBytes("UTF-8"), Right(1)),
      ("a trailing comma in the options", "compile.c.options = -DFOO=1,\n".getBytes("UTF-8"), Right(1)),
      ("a trailing comma in the defines", "preprocessor.defines = FOO=1,\n".getBytes("UTF-8"), Right(1)),
      ("a continued line and an escape", s"preprocessor.defines = F${bs}u004F$bs\n    O=1\n".getBytes("UTF-8"), Right(1)),
      ("a trailing comma and a blank in the defines", "preprocessor.defines = FOO=1, \n".getBytes("UTF-8"),
        Left(s"${failed}not supported on native: an empty define in preprocessor.defines")),
      ("a malformed escape", s"preprocessor.defines = FOO=${bs}u12\n".getBytes("UTF-8"), Left(s"${failed}Malformed ${bs}uxxxx encoding.")),
      ("project.gcProject neither true nor false", "project.gcProject = yes\n".getBytes("UTF-8"),
        Left(s"${failed}project.gcProject is yes, which is neither true nor false")))
    for (label, bytes, expected) <- readCases do
      Files.write(readRoot.resolve("code/scala-native/scala-native.properties"), bytes)
      val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, readRoot.resolve("code").toString) ++ jars).mkString(separator)
      val built = Sh(teq, "nir", "--link", "hello", classPath, s"--build=${readRoot.resolve("hello")}", s"--work=${readRoot.resolve("work")}", s"--zig=${zig.get}",
        s"--zig-cache=$cache").mergeErr.timeout(900).check(false).run()
      val ok = expected match
        case Right(foo) => built.ok && printedExactly(Sh(readRoot.resolve("hello").toString).timeout(300).check(false).run(), s"foo=$foo\n$helloLine".getBytes("UTF-8"))
        case Left(line) => built.status == 1 && built.out.linesIterator.exists(_ == line)
      if ok then pass += 1
      else
        fail += 1
        println(s"FAIL a descriptor with $label: the build exited ${built.status}, ${built.out.linesIterator.find(_.nonEmpty).getOrElse("").take(200)}")

    // An assembly source's object is compiled in every build (the review's second finding): a .S file whose `.incbin`
    // takes a file of a --lib-dir's include directory, which no list names, built, the file changed, built again into the
    // same directory: the executable prints the file's new bytes.
    val blobRoot = out.resolve("blob")
    Files.createDirectories(blobRoot.resolve("code/scala-native"))
    Files.createDirectories(blobRoot.resolve("lib/include"))
    Files.writeString(blobRoot.resolve("code/scala-native/blob.S"), "    .section .rodata\n    .globl probe_blob\nprobe_blob:\n    .incbin \"probe_blob.bin\"\n" +
      "    .byte 0\n    .section .note.GNU-stack,\"\",@progbits\n")
    Files.writeString(blobRoot.resolve("code/scala-native/print.c"), "#include <stdio.h>\nextern const char probe_blob[];\n" +
      "__attribute__((constructor)) static void print(void) {\n    printf(\"blob=%s\\n\", probe_blob);\n    fflush(stdout);\n}\n")
    for (label, bytes) <- List(("built", "one"), ("its blob changed, built again", "two")) do
      Files.writeString(blobRoot.resolve("lib/include/probe_blob.bin"), bytes)
      val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, blobRoot.resolve("code").toString) ++ jars).mkString(separator)
      val built = Sh(teq, "nir", "--link", "hello", classPath, s"--build=${blobRoot.resolve("hello")}", s"--work=${blobRoot.resolve("work")}", s"--zig=${zig.get}",
        s"--zig-cache=$cache", s"--lib-dir=${blobRoot.resolve("lib")}").mergeErr.timeout(900).check(false).run()
      val ran = if built.ok then Some(Sh(blobRoot.resolve("hello").toString).timeout(300).check(false).run()) else None
      if ran.exists(r => printedExactly(r, s"blob=$bytes\n$helloLine".getBytes("UTF-8"))) then pass += 1
      else
        fail += 1
        println(s"FAIL an assembly source $label: the build exited ${built.status}, the executable printed ${ran.map(_.outAll().take(60)).getOrElse("nothing")}")
        built.out.linesIterator.take(5).foreach(println)

    // An include directory that cannot be listed: a library whose C file includes a header from an include directory its
    // descriptor names, which can be searched but not listed (where the suite's user is not a superuser, who lists any).
    // Built, then built again into the same directory after the header changed: each prints its header's value.
    val shut = out.resolve("shut")
    Files.createDirectories(shut.resolve("code/scala-native"))
    val shutInclude = Files.createDirectories(shut.resolve("inc"))
    Files.writeString(shut.resolve("code/scala-native/s.c"), "#include <stdio.h>\n#include <shut_value.h>\n" +
      "__attribute__((constructor)) static void s(void) {\n    printf(\"shut=%d\\n\", SHUT_VALUE);\n    fflush(stdout);\n}\n")
    Files.writeString(shut.resolve("code/scala-native/scala-native.properties"), s"compile.c.options = -I$shutInclude\n")
    def shutBuild(value: Int): Boolean =
      Files.writeString(shutInclude.resolve("shut_value.h"), s"#define SHUT_VALUE $value\n")
      val classPath = (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, shut.resolve("code").toString) ++ jars).mkString(separator)
      val built = Sh(teq, "nir", "--link", "hello", classPath, s"--build=${shut.resolve("hello")}", s"--work=${shut.resolve("work")}", s"--zig=${zig.get}",
        s"--zig-cache=$cache").mergeErr.timeout(900).check(false).run()
      built.ok && printedExactly(Sh(shut.resolve("hello").toString).timeout(300).check(false).run(), s"shut=$value\n$helloLine".getBytes("UTF-8"))
    Files.writeString(shutInclude.resolve("shut_value.h"), "#define SHUT_VALUE 1\n")
    Sh("chmod", "311", shutInclude.toString).run()
    val listable = scala.util.Try(Files.list(shutInclude).close()).isSuccess
    val firstShut = shutBuild(1)
    val againShut = shutBuild(2)
    Sh("chmod", "755", shutInclude.toString).run()
    if firstShut && againShut then pass += 1
    else
      fail += 1
      println(s"FAIL an include directory that cannot be listed (listable: $listable): built $firstShut, its header changed and built again $againShut")

    // Headers reached by two names, and names whatever their spelling: libraries whose C files
    // add what headers of three --lib-dirs' include directories (L, N, M, in that order) say, all built into one work
    // directory; one thing changed at a time, then built again there, and every library's line has to be what a fresh
    // build prints. q1: a header and a link to it (`b/x.h -> ../a/x.h`), the link retargeted; q2: a header through a
    // linked directory (`b -> a`), the link retargeted; q5: a header of M while N's include directory is a link to L's,
    // retargeted to one whose header stands before M's; q6: a header that is a link into zig's own `lib`, retargeted to
    // a header of its own; q4: a header named `q4conf.d` of M, the same name added to L's; r1: `<../r1x.h>`, found above
    // M's include directory, the same name added above L's; r2: the same through a directory of each (`<sub/../../r2x.h>`).
    val two = out.resolve("two-names")
    for dir <- List("L/include/q1/a", "L/include/q2/a", "L/include/q2/d", "L/include/q6", "L/include/sub", "M/include/sub", "O/include/sub", "work") do
      Files.createDirectories(two.resolve(dir))
    def twoFile(name: String, text: String): Unit = Files.writeString(two.resolve(name), text)
    def link(target: String, name: String): Unit = Sh("ln", "-sfn", target, two.resolve(name).toString).run()
    twoFile("L/include/q1/a/x.h", "v += 1;\n")
    twoFile("L/include/q1/c.h", "v += 10;\n")
    Files.createDirectories(two.resolve("L/include/q1/b"))
    link("../a/x.h", "L/include/q1/b/x.h")
    twoFile("L/include/q2/a/x.h", "v += 1;\n")
    twoFile("L/include/q2/d/x.h", "v += 100;\n")
    link("a", "L/include/q2/b")
    twoFile("M/include/q5y.h", "v += 3;\n")
    twoFile("O/include/q5y.h", "v += 4;\n")
    Files.createDirectories(two.resolve("N"))
    link("../L/include", "N/include")
    val zigOwn = zig.get.toRealPath().getParent.resolve("lib/include/stddef.h")
    link(zigOwn.toString, "L/include/q6/s.h")
    twoFile("L/include/q6/real.h", "#define Q6 6\n")
    twoFile("M/include/q4conf.d", "#define Q4 2\n")
    twoFile("M/r1x.h", "#define R1 2\n")
    twoFile("M/r2x.h", "#define R2 2\n")
    /// Each library's C file: what it includes, at the file's level or within its function (q1, q2, q5, whose headers are
    /// statements), and the value it prints.
    def printing(name: String, top: String, inside: String, value: String): String =
      s"#include <stdio.h>\n${top}__attribute__((constructor)) static void $name(void) {\n    int v = 0;\n${inside}    v += $value;\n" +
        s"    printf(\"$name=%d\\n\", v);\n    fflush(stdout);\n}\n"
    val twoLibs = Vector(
      "q1" -> printing("q1", "", "#include \"q1/a/x.h\"\n#include \"q1/b/x.h\"\n", "0"),
      "q2" -> printing("q2", "", "#include \"q2/a/x.h\"\n#include \"q2/b/x.h\"\n", "0"),
      "q5" -> printing("q5", "", "#include <q5y.h>\n", "0"),
      "q6" -> printing("q6", "#include \"q6/s.h\"\n#ifndef Q6\n#define Q6 0\n#endif\n", "", "Q6"),
      "q4" -> printing("q4", "#include \"q4conf.d\"\n", "", "Q4"),
      "r1" -> printing("r1", "#include <../r1x.h>\n", "", "R1"),
      "r2" -> printing("r2", "#include <sub/../../r2x.h>\n", "", "R2"))
    for (name, source) <- twoLibs do
      Files.writeString(Files.createDirectories(two.resolve(s"libs/$name/scala-native")).resolve(s"$name.c"), source)
    def twoBuild(): Map[String, String] =
      val classPath = (Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString +: twoLibs.map((name, _) => two.resolve(s"libs/$name").toString) ++: jars).mkString(separator)
      val built = Sh(teq, "nir", "--link", "hello", classPath, s"--build=${two.resolve("hello")}", s"--work=${two.resolve("work")}", s"--zig=${zig.get}",
        s"--zig-cache=$cache", s"--lib-dir=${two.resolve("L")}", s"--lib-dir=${two.resolve("N")}", s"--lib-dir=${two.resolve("M")}").mergeErr.timeout(900).check(false).run()
      if !built.ok then
        built.out.linesIterator.take(5).foreach(println)
        Map.empty
      else Sh(two.resolve("hello").toString).timeout(300).check(false).run().out.linesIterator.filter(_.contains("=")).map(l => l.takeWhile(_ != '=') -> l.dropWhile(_ != '=').drop(1)).toMap
    var twoExpected = Map("q1" -> "2", "q2" -> "2", "q5" -> "3", "q6" -> "0", "q4" -> "2", "r1" -> "2", "r2" -> "2")
    def twoCell(label: String, change: () => Unit, changed: (String, String)): Unit =
      change()
      twoExpected += changed
      val printed = twoBuild()
      if printed.filter((k, _) => twoExpected.contains(k)) == twoExpected then pass += 1
      else
        fail += 1
        println(s"FAIL two names, $label: printed ${printed.toVector.sorted.map((k, v) => s"$k=$v").mkString(" ")}, not ${twoExpected.toVector.sorted.map((k, v) => s"$k=$v").mkString(" ")}")
    twoCell("built", () => (), "q1" -> "2")
    twoCell("a header's link retargeted", () => link("../c.h", "L/include/q1/b/x.h"), "q1" -> "11")
    twoCell("a linked directory retargeted", () => link("d", "L/include/q2/b"), "q2" -> "101")
    twoCell("a linked include directory retargeted", () => link("../O/include", "N/include"), "q5" -> "4")
    twoCell("a header linking into zig's lib retargeted", () => link("real.h", "L/include/q6/s.h"), "q6" -> "6")
    twoCell("a header named as an object's list added before it", () => twoFile("L/include/q4conf.d", "#define Q4 1\n"), "q4" -> "1")
    twoCell("a header found above an include directory added above an earlier one", () => twoFile("L/r1x.h", "#define R1 1\n"), "r1" -> "1")
    twoCell("a header found above an include directory through a piece of it, added above an earlier one", () => twoFile("L/r2x.h", "#define R2 1\n"), "r2" -> "1")

  if strandHas("pairs") then
    val race = raceAt(out.resolve("race"))
    val racePairs = 40
    val raceFaults = (0 until racePairs).flatMap { k =>
      val a = raceCmd(race, "A").spawn()
      Thread.sleep((k % 8) * 150L)
      val b = raceCmd(race, "B").spawn()
      val (ra, rb) = (a.await(), b.await())
      val waits = (ra.out.linesIterator ++ rb.out.linesIterator).count(_.endsWith(": another native build holds this work directory; waiting for it"))
      Vector(Option.when(!raceRan(race, "A", ra))(s"pair $k: A exited ${ra.status}, ${ra.out.linesIterator.find(_.nonEmpty).getOrElse("").take(120)}"),
        Option.when(!raceRan(race, "B", rb))(s"pair $k: B exited ${rb.status}, ${rb.out.linesIterator.find(_.nonEmpty).getOrElse("").take(120)}"),
        Option.when(waits > 1)(s"pair $k: the waiting line $waits times")).flatten
    }
    if raceFaults.isEmpty then pass += 1
    else
      fail += 1
      println(s"FAIL two builds at once into one work directory: ${raceFaults.length} faults over $racePairs pairs")
      raceFaults.take(8).foreach(f => println(s"  $f"))

  if strandHas("rounds") then
    val race = raceAt(out.resolve("race-three"))
    val threeFaults = (0 until 10).flatMap { k =>
      val runs = Vector("A", "B", "A").zipWithIndex.map((which, i) => (which, i, raceThreeCmd(race, which, i).spawn()))
      runs.flatMap { (which, i, run) =>
        val built = run.await()
        val value = if which == "A" then "1" else "2"
        val ran = built.ok && printedExactly(Sh(race.resolve(s"hello-$which-$i").toString).timeout(300).check(false).run(), s"c3=${value * raceFiles}\n$helloLine".getBytes("UTF-8"))
        val waits = built.out.linesIterator.count(_.endsWith(": another native build holds this work directory; waiting for it"))
        Vector(Option.when(!ran)(s"round $k: build $i ($which) exited ${built.status}, ${built.out.linesIterator.find(_.nonEmpty).getOrElse("").take(120)}"),
          Option.when(waits > 1)(s"round $k: build $i printed the waiting line $waits times")).flatten
      }
    }
    if threeFaults.isEmpty then pass += 1
    else
      fail += 1
      println(s"FAIL three builds at once into one work directory: ${threeFaults.length} faults over 10 rounds")
      threeFaults.take(8).foreach(f => println(s"  $f"))
    val beforeKill = raceCmd(race, "B").run()
    val heldZig = race.resolve("held-zig")
    val released = race.resolve("released")
    Files.deleteIfExists(released)
    val waitingLine = ": another native build holds this work directory; waiting for it"
    // zig, then the wait for the cell's release (half an hour at most), exiting with zig's status.
    Files.writeString(heldZig, s"#!/bin/sh\n[ \"$$1\" = version ] && exec ${shellQuoted(zig.get.toString)} version\n${shellQuoted(zig.get.toString)} \"$$@\"; c=$$?\n" +
      s"n=0; while [ ! -e ${shellQuoted(released.toString)} ] && [ $$n -lt 18000 ]; do sleep 0.1; n=$$((n+1)); done\nexit $$c\n")
    Sh("chmod", "755", heldZig.toString).run()
    val killedFrom = System.currentTimeMillis()
    val killed = raceCmd(race, "A", heldZig).spawn()
    val raceObjects = race.resolve("work/dependencies/lib-0/scala-native")
    // Killed once its compilers run on the library's C files: an object of its own there, written since it started (the
    // build before wrote every object too, a moment earlier). A file that goes while it is looked at (zig writes an object
    // beside it and moves it into place) is none.
    def freshObject(): Boolean =
      scala.util.Try(files(raceObjects).exists(f => f.toString.endsWith(".c.o") &&
        scala.util.Try(Files.getLastModifiedTime(f).toMillis > killedFrom).getOrElse(false))).getOrElse(false)
    var waited = 0
    while waited < 600 && killed.alive && !freshObject() do
      Thread.sleep(100)
      waited += 1
    val killedWhileCompiling = killed.alive
    Sh("kill", "-9", killed.process.pid().toString).check(false).run()
    killed.await()
    /// The killed build's compilers: the processes whose command names its work directory's library.
    def compilersLeft(): Int = Sh("pgrep", "-f", raceObjects.toString).check(false).run().out.linesIterator.count(_.nonEmpty)
    val leftAtKill = compilersLeft()
    val startedAfter = System.nanoTime()
    val next = raceCmd(race, "B").spawn()
    def nextSaysItWaits(): Boolean =
      scala.util.Try(new String(Files.readAllBytes(next.out), "UTF-8").linesIterator.exists(_.endsWith(waitingLine))).getOrElse(false)
    var polled = 0
    while polled < 9000 && next.alive && !nextSaysItWaits() do
      Thread.sleep(100)
      polled += 1
    val lineSeen = nextSaysItWaits()
    val waitingWhenSeen = next.alive
    val leftWhenSeen = compilersLeft()
    Files.writeString(released, "")
    val afterKill = next.await()
    val afterSeconds = (System.nanoTime() - startedAfter) / 1000000000
    val leftAfter = compilersLeft()
    val waitedLine = afterKill.out.linesIterator.count(_.endsWith(waitingLine))
    val firstAgain = raceCmd(race, "A").run()
    if raceRan(race, "B", beforeKill) && killedWhileCompiling && leftAtKill > 0 && lineSeen && waitingWhenSeen && leftWhenSeen > 0 &&
        raceRan(race, "B", afterKill) && waitedLine == 1 && leftAfter == 0 && raceRan(race, "A", firstAgain) then pass += 1
    else
      fail += 1
      println(s"FAIL a build killed while it compiles: killed while compiling $killedWhileCompiling, $leftAtKill of its compilers left; the next build's " +
        s"waiting line seen $lineSeen, it still waiting then $waitingWhenSeen with $leftWhenSeen compilers left; it exited ${afterKill.status} after " +
        s"$afterSeconds s, said it waits $waitedLine times, $leftAfter compilers left as it ended; the one after exited ${firstAgain.status}")
      afterKill.out.linesIterator.take(5).foreach(println)
    val leftAsItSaid = if lineSeen then s" ($leftWhenSeen compilers left as it said so)" else ""
    println(s"native-build lock: the build after one killed found $leftAtKill of its compilers holding the lock, said it waits $waitedLine times" +
      s"$leftAsItSaid and ended after $afterSeconds s")
    val readOnly = Files.createDirectories(race.resolve("read-only"))
    Sh("chmod", "555", readOnly.toString).run()
    val writable = scala.util.Try { Files.writeString(readOnly.resolve("probe"), ""); Files.delete(readOnly.resolve("probe")); true }.getOrElse(false)
    val roBuilt = Sh(teq, "nir", "--link", "hello", (List(Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString, race.resolve("lib").toString) ++ jars).mkString(separator),
      s"--build=${race.resolve("hello-read-only")}", s"--work=$readOnly", s"--zig=${zig.get}", s"--zig-cache=$cache").mergeErr.timeout(900).check(false).run()
    val roLeft = files(readOnly)
    Sh("chmod", "755", readOnly.toString).run()
    if writable || (roBuilt.status == 1 && roBuilt.out.linesIterator.exists(_.startsWith(s"${readOnly.resolve("lock")}: ")) && roLeft.isEmpty &&
        !Files.exists(race.resolve("hello-read-only"))) then pass += 1
    else
      fail += 1
      println(s"FAIL a work directory that cannot be written: exit ${roBuilt.status}, ${roLeft.length} files written, ${roBuilt.out.linesIterator.find(_.nonEmpty).getOrElse("").take(160)}")

  if strandHas("misc") then
    // A library refused by its descriptor last on the class path, after the jars: refused before anything is compiled,
    // the work directory without an object.
    val lastRoot = out.resolve("refused-last")
    Files.createDirectories(lastRoot.resolve("bad/scala-native"))
    Files.writeString(lastRoot.resolve("bad/scala-native/bad.c"), "int bad;\n")
    Files.writeString(lastRoot.resolve("bad/scala-native/scala-native.properties"), "compile.c.options = -Wall\n")
    val lastBuilt = Sh(teq, "nir", "--link", "hello", (Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString +: jars :+ lastRoot.resolve("bad").toString).mkString(separator),
      s"--build=${lastRoot.resolve("hello")}", s"--work=${lastRoot.resolve("work")}", s"--zig=${zig.get}", s"--zig-cache=$cache").mergeErr.timeout(900).check(false).run()
    val lastObjects = files(lastRoot.resolve("work")).filter(_.getFileName.toString.endsWith(".o"))
    val lastLine = s"${lastRoot.resolve("bad")}: scala-native.properties: not supported on native: the compile option -Wall in a library's descriptor"
    if lastBuilt.status == 1 && lastBuilt.out.linesIterator.exists(_ == lastLine) && lastObjects.isEmpty then pass += 1
    else
      fail += 1
      println(s"FAIL a library refused last on the class path: exit ${lastBuilt.status}, ${lastObjects.length} objects compiled, ${lastBuilt.out.linesIterator.find(_.nonEmpty).getOrElse("").take(160)}")

    // A build that waits for its work directory says so once: the directory held by another process (`flock`), the build
    // prints its line once, waits, then builds; a build whose directory is free prints none (the cells above).
    val waiting = Files.createDirectories(out.resolve("waiting/work"))
    val waitLine = s"$waiting: another native build holds this work directory; waiting for it"
    val holder = Sh(flockTool, waiting.resolve("lock").toString, "sleep", "20").spawn()
    var held = 0
    while held < 100 && Sh(flockTool, "-n", waiting.resolve("lock").toString, "true").check(false).run().ok do
      Thread.sleep(100)
      held += 1
    val heldBuild = Sh(teq, "nir", "--link", "hello", (Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString :: jars).mkString(separator),
      s"--build=${out.resolve("waiting/hello")}", s"--work=$waiting", s"--zig=${zig.get}", s"--zig-cache=$cache").mergeErr.timeout(1800).check(false).run()
    holder.await()
    val waitLines = heldBuild.out.linesIterator.count(_ == waitLine)
    if held < 100 && heldBuild.ok && waitLines == 1 && runsAt(out.resolve("waiting/hello"), "tests/native-build/hello.expected") then pass += 1
    else
      fail += 1
      println(s"FAIL a build that waits for its work directory: held ${held < 100}, exit ${heldBuild.status}, the line $waitLines times")
      heldBuild.out.linesIterator.take(5).foreach(println)

  if strandHas("jars") then
    // The jars of tests/native-build/jar-names.expected (tests/native-build/jar-names-oracle.scala writes it: what Scala
    // Native 0.5.12's build makes of each through the JDK's readers), each written from its entries and held to the
    // oracle's SHA-256, then given to a build. A jar the JDK refuses: a build of its own, the jar last on the class path
    // after a library of C code and the jars of Scala Native, refused naming the jar for what the JDK refused it
    // (`refusalKind`), the work directory holding its lock alone. Every other jar, all in one build: one of no library
    // unpacked nowhere, each other unpacked to the JDK's files, name for name and byte for byte. Where the JDK reads an
    // entry as a file and a directory both (`clash`: it keeps one and leaves the other out), the build refuses the jar
    // (src/native/build.rs, `read_jar`), as one the JDK refuses: counted apart.
    /// An entry as the oracle writes it: flag, external attributes, name, comment, data, method, extra field, the
    /// compressed size, size, offset and CRC-32 its central header gives (-1: the entry's own), disk number, local name.
    final case class JarEntry(flag: Int, attr: Int, name: Array[Byte], comment: Array[Byte], data: Array[Byte], method: Int, extra: Array[Byte],
        csize: Long, size: Long, offset: Long, crc: Long, disk: Int, local: Array[Byte])
    /// A jar of the oracle's file: its name, SHA-256 and verdict; its entries and the bytes after its directory's last
    /// entry, or, of one written byte by byte, its bytes; whether a real tool wrote it (tests/native-build/tool-jars.py).
    final case class JarCase(name: String, sha: String, verdict: String, entries: Vector[JarEntry], trailing: Array[Byte], raw: Option[Array[Byte]] = None,
        tool: Boolean = false)
    def unescaped(text: String): Array[Byte] =
      val bytes = new java.io.ByteArrayOutputStream
      var at = 0
      while at < text.length do
        if text.charAt(at) == '\\' && text.startsWith("\\x", at) then
          bytes.write(Integer.parseInt(text.substring(at + 2, at + 4), 16))
          at += 4
        else
          bytes.write(text.charAt(at).toInt)
          at += 1
      bytes.toByteArray
    def escapedName(bytes: Array[Byte]): String =
      bytes.map(x => x & 0xff).map(c => if c > 0x20 && c < 0x7f && !"\\,;|=".contains(c.toChar) then c.toChar.toString else f"\\x$c%02X").mkString
    def sha256Hex(bytes: Array[Byte]): String = java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(bytes))
    /// The oracle's writer: a stored zip, each entry's local header and data, the central directory and the bytes after
    /// its last entry, its end.
    def jarBytes(entries: Vector[JarEntry], trailing: Array[Byte]): Array[Byte] =
      val bytes = new java.io.ByteArrayOutputStream
      def u16(v: Int): Unit = { bytes.write(v & 0xff); bytes.write((v >> 8) & 0xff) }
      def u32(v: Long): Unit = { u16((v & 0xffff).toInt); u16(((v >> 16) & 0xffff).toInt) }
      val written = entries.map { e =>
        val at = bytes.size
        val crc = new java.util.zip.CRC32
        crc.update(e.data)
        val local = if e.local.isEmpty then e.name else e.local
        u32(0x04034b50L); u16(20); u16(e.flag); u16(e.method); u16(0); u16(0x21); u32(crc.getValue)
        u32(e.data.length.toLong); u32(e.data.length.toLong); u16(local.length); u16(0)
        bytes.write(local); bytes.write(e.data)
        (at, crc.getValue)
      }
      val directory = bytes.size
      for (e, (at, crc)) <- entries.zip(written) do
        def or(set: Long, own: Long): Long = if set >= 0 then set else own
        u32(0x02014b50L); u16(0x0314); u16(20); u16(e.flag); u16(e.method); u16(0); u16(0x21); u32(or(e.crc, crc))
        u32(or(e.csize, e.data.length.toLong)); u32(or(e.size, e.data.length.toLong)); u16(e.name.length); u16(e.extra.length); u16(e.comment.length)
        u16(e.disk); u16(0); u32(e.attr.toLong & 0xffffffffL); u32(or(e.offset, at.toLong))
        bytes.write(e.name); bytes.write(e.extra); bytes.write(e.comment)
      bytes.write(trailing)
      val size = bytes.size - directory
      u32(0x06054b50L); u16(0); u16(0); u16(entries.length); u16(entries.length); u32(size.toLong); u32(directory.toLong); u16(0)
      bytes.toByteArray
    val jarLines = Files.readString(Paths.get("tests/native-build/jar-names.expected")).linesIterator.filter(l => l.nonEmpty && !l.startsWith("#")).toVector
    def hex(text: String): Long = text.foldLeft(0L)((a, c) => a * 16 + Character.digit(c, 16))
    /// A line of the oracle's file as a case, or why it is none (a line cut short or of another shape is a fault, not an
    /// exception).
    def jarCase(line: String): Either[String, JarCase] = scala.util.Try {
      val last = line.lastIndexOf(" | ")
      val cut = line.lastIndexOf(" | ", last - 1)
      val fields = line.substring(0, cut).split(" ", 3)
      val written = line.substring(cut + 3, last)
      val raw = Option.when(written == "raw")(line.substring(last + 3).grouped(2).map(Integer.parseInt(_, 16).toByte).toArray)
      val entries = if written.isEmpty || raw.isDefined then Vector.empty else written.split(";", -1).toVector.map { e =>
        val f = e.split(",", -1)
        def field(text: String): Long = if text.isEmpty then -1L else hex(text)
        JarEntry(hex(f(0)).toInt, hex(f(1)).toInt, unescaped(f(2)), unescaped(f(3)), unescaped(f(4)), hex(f(5)).toInt, unescaped(f(6)),
          field(f(7)), field(f(8)), field(f(9)), field(f(10)), hex(f(11)).toInt, unescaped(f(12)))
      }
      val tool = fields(2).startsWith("tool ")
      JarCase(fields(0), fields(1), fields(2).stripPrefix("tool "), entries, if raw.isDefined then Array[Byte]() else unescaped(line.substring(last + 3)), raw, tool)
    } match
      case scala.util.Success(read) => Right(read)
      case scala.util.Failure(e) => Left(s"a line of tests/native-build/jar-names.expected that does not read (${e.getClass.getSimpleName}: ${e.getMessage}): ${line.take(100)}")
    val parsedLines = jarLines.map(jarCase)
    val unread = parsedLines.collect { case Left(why) => why }
    val jarCases = parsedLines.collect { case Right(read) => read }
    val clashLine = "not supported on native: a file and a directory of one name"
    /// zlib 1.3.1's refusals of a raw deflate stream (the JDK's bundled inflate.c), which the copy's inflater makes in
    /// zlib's words (src/native/inflate.rs).
    val zlibRefusals = Vector("invalid block type", "invalid stored block lengths", "too many length or distance symbols", "invalid code lengths set",
      "invalid bit length repeat", "invalid code -- missing end-of-block", "invalid literal/lengths set", "invalid distances set",
      "invalid literal/length code", "invalid distance code", "invalid distance too far back")
    /// The refusal of teq's class path read, the read the link shares, and of a jar whose directory it takes elsewhere.
    val classPathLine = "the class path, entry"
    val elsewhereLine = "teq's class path read at byte"
    /// What the build's refusal says, by what the JDK refused a jar for: ZipFile's bad name or comment, encryption,
    /// method, header size, zip64 block, extra field or bytes no entry's, the zip file system's `.` or `..` piece or bytes
    /// after the last entry, the copy's local header, CRC-32 or size belied, a NUL, a name too long, or its root a file (an
    /// empty name); besides, a file and a directory of one name (`clash`), a jar of a shape teq's class path read refuses
    /// (`classpath`) and one whose directory it takes elsewhere than the JDK (`teqread`), which the build refuses where the
    /// JDK reads them.
    def refusalKind(verdict: String): Option[String] =
      if verdict.startsWith("classpath ") then Some(classPathLine)
      else if zlibRefusals.exists(verdict.contains) then zlibRefusals.find(verdict.contains).map(why => s"its deflated data: $why")
      else if verdict.startsWith("teqread ") then Some(elsewhereLine)
      else if verdict.contains("bad entry name or comment") then Some("is not UTF-8")
      else if verdict.contains("(encrypted entry)") then Some("it is encrypted")
      else if verdict.contains("bad compression method") || verdict.contains("unsupported compression method") then Some("neither stored nor deflated")
      else if verdict.contains("(bad header size)") then Some("is over 65535 or runs past the directory|bytes after its last entry")
      else if verdict.contains("zip64 extra") || verdict.contains("Zip64 extra") then Some("zip64 block|fields of zip64")
      else if verdict.contains("invalid extra data field size for tag") then Some("runs past the field")
      else if verdict.contains("(bad signature)") && verdict.contains("CEN") then Some("are no entry's")
      else if verdict.contains("invalid LOC header") then Some("are no local header")
      else if verdict.contains("invalid entry CRC") || verdict.contains("invalid entry size") || verdict.contains("invalid entry compressed size") then Some("mismatch|truncated entry")
      else if verdict.contains("has a '.' or '..' element") then Some("is . or ..")
      else if verdict.contains("Nul character not allowed") then Some("the name holds a NUL")
      else if verdict.contains("File name too long") then Some("longer than")
      else if verdict.startsWith("refused unzip java.nio.file.FileAlreadyExistsException") then Some("an entry with an empty name")
      else if verdict.contains("zip END header not found") then Some("no end record")
      else if verdict.contains("for entry reading") then Some("no local header at|are no local header")
      // The zip file system reads the local header of an entry whose central extra field holds Info-ZIP's extended
      // timestamp, and names a wrong signature by its hexadecimal value (ZipFileSystem.java `readLocEXTT` 3352-3359).
      else if verdict.matches(".*ZipException: R[0-9a-f]+( .*)?") then Some("no local header at|are no local header")
      else if verdict.contains("Unexpected end of ZLIB input stream") then Some("its deflated data: unexpected end")
      else if verdict.startsWith("clash files") then Some(clashLine)
      else None
    /// What a build made of a jar: its refusal's lines and what it left in its work directory, or the jar's files as the
    /// verdict writes them (`none` where it was no library).
    enum JarRun:
      case Refused(status: Int, lines: Vector[String], left: Vector[String])
      case Built(verdict: String)
    /// A jar's fault, or none: a refusal is of the JDK's kind (`refusalKind`), naming the jar, the work directory its lock
    /// alone (or none, where the link's class path read refused it); a jar the JDK refuses may be refused by teq's class
    /// path read, or as one whose directory it takes elsewhere, instead; a build gives the JDK's files; the written jar is
    /// the oracle's.
    def jarFault(c: JarCase, jar: String, written: String, run: Option[JarRun]): Option[String] =
      if written != c.sha then Some(s"${c.name}: the jar written here is not the oracle's")
      else
        (run, refusalKind(c.verdict)) match
          case (None, _) => Some(s"${c.name}: no run")
          case (Some(JarRun.Refused(status, lines, left)), kind) =>
            val said = lines.find(l => l.startsWith(s"$jar: ") || l.startsWith(classPathLine) && l.contains(s" $jar: "))
            val byClassPath = said.exists(_.startsWith(classPathLine))
            if status != 1 || said.isEmpty then Some(s"${c.name}: exit $status, no line naming the jar")
            else if left != Vector("lock") && !(byClassPath && left.isEmpty) then Some(s"${c.name}: refused, the work directory holding ${left.take(3).mkString(", ")}")
            else kind match
              case Some(k) if k.split('|').exists(said.get.contains) => None
              case Some(_) if c.verdict.startsWith("refused") && (byClassPath || said.get.contains(elsewhereLine)) => None
              case Some(k) => Some(s"${c.name}: refused as ${said.get.drop(jar.length + 2).take(120)}, the JDK for ${c.verdict.take(120)}")
              case None if c.verdict.startsWith("refused") => Some(s"${c.name}: the JDK's refusal is of no kind the table knows: ${c.verdict.take(120)}")
              case None => Some(s"${c.name}: refused as ${said.get.drop(jar.length + 2).take(120)}, the JDK's ${c.verdict.take(80)}")
          case (Some(JarRun.Built(verdict)), kind) =>
            if kind.isDefined then Some(s"${c.name}: built, the JDK refused it: ${c.verdict.take(120)}")
            else Option.when(verdict != c.verdict)(s"${c.name}: ${verdict.take(160)}, the JDK's ${c.verdict.take(160)}")
    // The verdict's self-test: planted bad runs, each of which it must find, and the good ones it must take.
    {
      val refusedCase = JarCase("r", "s", "refused unzip java.util.zip.ZipException: entry has a '.' or '..' element in its name", Vector.empty, Array[Byte]())
      val builtCase = JarCase("b", "s", "files scala-native/jk.c=00", Vector.empty, Array[Byte]())
      val clashCase = JarCase("c", "s", "clash files scala-native/jk.c=00 scala-native/x=00", Vector.empty, Array[Byte]())
      val noneCase = JarCase("n", "s", "none", Vector.empty, Array[Byte]())
      val good = Vector(
        jarFault(refusedCase, "/j.jar", "s", Some(JarRun.Refused(1, Vector("/j.jar: the entry named ./x.c: a piece of the name is . or .."), Vector("lock")))),
        jarFault(builtCase, "/j.jar", "s", Some(JarRun.Built("files scala-native/jk.c=00"))),
        jarFault(clashCase, "/j.jar", "s", Some(JarRun.Refused(1, Vector(s"/j.jar: the entries named x and x/y: $clashLine"), Vector("lock")))),
        jarFault(noneCase, "/j.jar", "s", Some(JarRun.Built("none"))),
        jarFault(JarCase("p", "s", "classpath files scala-native/jk.c=00", Vector.empty, Array[Byte]()), "/j.jar", "s",
          Some(JarRun.Refused(1, Vector("the class path, entry 2: /j.jar: central directory lists 1 entries, found 0"), Vector()))),
        jarFault(refusedCase, "/j.jar", "s", Some(JarRun.Refused(1, Vector(s"/j.jar: not supported on native: the JDK finds its central directory at byte 9 (46 bytes), $elsewhereLine 0 (46 bytes)"), Vector("lock")))))
      val plants = Vector(
        "a jar written unlike the oracle's" -> jarFault(builtCase, "/j.jar", "t", Some(JarRun.Built("files scala-native/jk.c=00"))),
        "a refused jar that built" -> jarFault(refusedCase, "/j.jar", "s", Some(JarRun.Built("files scala-native/jk.c=00"))),
        "a refusal that left files" -> jarFault(refusedCase, "/j.jar", "s", Some(JarRun.Refused(1, Vector("/j.jar: the entry named ./x.c: a piece of the name is . or .."), Vector("lock", "dependencies/x")))),
        "a refusal of another kind" -> jarFault(refusedCase, "/j.jar", "s", Some(JarRun.Refused(1, Vector("/j.jar: the entry named x: the name holds a NUL"), Vector("lock")))),
        "a refusal naming another jar" -> jarFault(refusedCase, "/j.jar", "s", Some(JarRun.Refused(1, Vector("/k.jar: the entry named ./x.c: a piece of the name is . or .."), Vector("lock")))),
        "a jar unpacked unlike the JDK" -> jarFault(builtCase, "/j.jar", "s", Some(JarRun.Built("files scala-native/jk.c=01"))),
        "a jar of no library unpacked" -> jarFault(noneCase, "/j.jar", "s", Some(JarRun.Built("files scala-native/jk.c=00"))),
        "a jar the JDK unpacks refused" -> jarFault(builtCase, "/j.jar", "s", Some(JarRun.Refused(1, Vector("/j.jar: the entry named x: the name holds a NUL"), Vector("lock")))),
        "a jar of no library refused as a clash" -> jarFault(noneCase, "/j.jar", "s", Some(JarRun.Refused(1, Vector(s"/j.jar: the entries named x and x/y: $clashLine"), Vector("lock")))),
        "a jar the JDK unpacks refused as a clash" -> jarFault(builtCase, "/j.jar", "s", Some(JarRun.Refused(1, Vector(s"/j.jar: the entries named x and x/y: $clashLine"), Vector("lock")))),
        "a clash built" -> jarFault(clashCase, "/j.jar", "s", Some(JarRun.Built("files scala-native/jk.c=00 scala-native/x=00"))),
        "a jar without its run" -> jarFault(builtCase, "/j.jar", "s", None),
        "a jar the JDK reads refused by the class path read" -> jarFault(builtCase, "/j.jar", "s", Some(JarRun.Refused(1, Vector("the class path, entry 2: /j.jar: zip64 archives are not supported"), Vector()))),
        "a jar of the class path read's shape built" -> jarFault(JarCase("p", "s", "classpath files scala-native/jk.c=00", Vector.empty, Array[Byte]()), "/j.jar", "s", Some(JarRun.Built("files scala-native/jk.c=00"))),
        "a stream zlib refuses for one reason refused for another" -> jarFault(JarCase("z", "s", "refused unzip java.util.zip.ZipException: invalid literal/lengths set", Vector.empty, Array[Byte]()),
          "/j.jar", "s", Some(JarRun.Refused(1, Vector("/j.jar: the entry named scala-native/z.c: its deflated data: invalid distances set"), Vector("lock")))),
        "a refusal of a kind the table does not know" -> jarFault(JarCase("u", "s", "refused ZipFile java.util.zip.ZipException: a reason of a later JDK", Vector.empty, Array[Byte]()), "/j.jar", "s",
          Some(JarRun.Refused(1, Vector("/j.jar: the entry named x: the name holds a NUL"), Vector("lock")))),
        "a line that does not read" -> (jarCase("cut short") match { case Left(why) => Some(why); case Right(_) => None }))
      for (what, found) <- plants if found.isEmpty do
        fail += 1
        println(s"FAIL the jar names' self-test: $what was taken")
      if good.exists(_.isDefined) then
        fail += 1
        println(s"FAIL the jar names' self-test: a good run was refused: ${good.flatten.mkString("; ")}")
      if good.forall(_.isEmpty) && plants.forall(_._2.isDefined) then println(s"native-build jar names self-test: ${plants.length} planted bad runs found, the good ones taken")
    }
    val namesStarted = System.nanoTime()
    val jarRoot = Files.createDirectories(out.resolve("jar-names"))
    val jarPaths = jarCases.map(c => jarRoot.resolve(s"jars/${c.name}.jar"))
    Files.createDirectories(jarRoot.resolve("jars"))
    val writtenShas = jarCases.zip(jarPaths).map { (c, jar) =>
      val bytes = c.raw.getOrElse(jarBytes(c.entries, c.trailing))
      Files.write(jar, bytes)
      sha256Hex(bytes)
    }
    val goodLib = Files.createDirectories(jarRoot.resolve("good/scala-native"))
    Files.writeString(goodLib.resolve("good.c"), "int jar_names_good(void) { return 0; }\n")
    def jarBuild(classPath: String, work: Path): Sh.Cmd =
      Sh(teq, "nir", "--link", "hello", classPath, s"--build=${jarRoot.resolve("hello")}", s"--work=$work", s"--zig=${zig.get}", s"--zig-cache=$cache").mergeErr.timeout(900).check(false)
    val refusing = jarCases.indices.filter(k => !jarCases(k).verdict.startsWith("files") && jarCases(k).verdict != "none").toSet
    val jarRuns = scala.collection.mutable.Map.empty[Int, JarRun]
    for batch <- jarCases.indices.filter(refusing).grouped(6) do
      val spawned = batch.map { k =>
        val work = jarRoot.resolve(s"work/${jarCases(k).name}")
        val classPath = (Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString +: jarRoot.resolve("good").toString +: jars :+ jarPaths(k).toString).mkString(separator)
        (k, work, jarBuild(classPath, work).spawn())
      }
      for (k, work, run) <- spawned do
        val built = run.await()
        jarRuns(k) = if built.ok then JarRun.Built("built") else JarRun.Refused(built.status, built.out.linesIterator.toVector, files(work).map(f => work.relativize(f).toString))
    val together = jarCases.indices.filterNot(refusing)
    val togetherArgs = jarRoot.resolve("together.args")
    Files.writeString(togetherArgs, ((Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString +: together.map(k => jarPaths(k).toString)) ++ jars).mkString(separator) + "\n")
    val togetherWork = jarRoot.resolve("work/together")
    val togetherBuilt = jarBuild(s"@$togetherArgs", togetherWork).run()
    val dependencies = togetherWork.resolve("dependencies")
    val unpackedDirs = if Files.isDirectory(dependencies) then Files.list(dependencies).toArray.toVector.map(_.asInstanceOf[Path]) else Vector.empty
    for k <- together do
      if !togetherBuilt.ok then jarRuns(k) = JarRun.Refused(togetherBuilt.status, togetherBuilt.out.linesIterator.toVector, Vector.empty)
      else
        val prefix = s"${jarCases(k).name}-"
        val dest = unpackedDirs.find(d => d.getFileName.toString.startsWith(prefix) && d.getFileName.toString.drop(prefix.length).forall(_.isDigit))
        jarRuns(k) = JarRun.Built(dest match
          case None => "none"
          case Some(d) =>
            // The build's own objects beside the sources it compiled left out.
            val unpacked = files(d.resolve("scala-native")).filterNot(f => f.toString.endsWith(".o") && Files.isRegularFile(Paths.get(f.toString.stripSuffix(".o"))))
            val named = unpacked.map(f => (d.relativize(f).toString.getBytes("UTF-8"), sha256Hex(Files.readAllBytes(f)).take(16)))
              .sortBy(p => new String(p._1, "ISO-8859-1"))
            "files" + named.map((n, digest) => s" ${escapedName(n)}=$digest").mkString)
    val faultOf = jarCases.indices.map(k => jarFault(jarCases(k), jarPaths(k).toString, writtenShas(k), jarRuns.get(k)))
    val jarFaults = unread ++ faultOf.flatten
    // What decides the part: every jar a real tool wrote is as Scala Native's build makes it, under no row's mark. A jar
    // crafted byte by byte agrees, is refused by both, or is a row: a file and a directory of one name (which the copy
    // cannot write both of), a shape teq's class path read refuses, a directory that read takes elsewhere.
    def rowMarked(c: JarCase): Boolean = c.verdict.startsWith("clash ") || c.verdict.startsWith("classpath ") || c.verdict.startsWith("teqread ")
    val byTools = jarCases.indices.filter(jarCases(_).tool)
    val toolAsScalaNative = byTools.filter(k => faultOf(k).isEmpty && !rowMarked(jarCases(k)))
    // The one shape of a real tool's jar the build does not make as Scala Native does: an end record carrying
    // zip64's marks (Info-ZIP's forced zip64; any tool's jar of more than 65,535 entries), which teq's class path read
    // refuses on every target before this part's code runs, the booked row. Held to that refusal's own line, so that
    // the cell fails if it ever turns into a build, or another jar takes its place.
    def classPathZip64(jar: String, run: Option[JarRun]): Boolean = run.exists {
      case JarRun.Refused(1, lines, _) => lines.exists(l => l.matches(s"the class path, entry [0-9]+: ${java.util.regex.Pattern.quote(jar)}: zip64 archives are not supported"))
      case _ => false
    }
    val toolZip64 = byTools.filter(k => jarCases(k).verdict.startsWith("classpath ") && faultOf(k).isEmpty && classPathZip64(jarPaths(k).toString, jarRuns.get(k)))
    // A jar of more than 65,535 entries, which the JDK's jar tool and Python's zipfile write with a zip64 end record and
    // locator and the end record's count at its mark: written here by Python's zipfile (65,537 entries, one a C file;
    // Scala Native's build unpacks it), refused by the class path read as above.
    val manyJar = jarRoot.resolve("tool-python-65537.jar")
    Sh("python3", "-c", "import sys, zipfile\nz = zipfile.ZipFile(sys.argv[1], 'w')\nfor i in range(65536): z.writestr(zipfile.ZipInfo('data/f%05d.txt' % i, (1980, 1, 1, 0, 0, 0)), b'')\n" +
      "z.writestr(zipfile.ZipInfo('scala-native/many.c', (1980, 1, 1, 0, 0, 0)), 'int tw_many(void) { return 0; }\\n')\nz.close()", manyJar.toString).timeout(300).run()
    val manyWork = jarRoot.resolve("work/tool-python-65537")
    val manyBuilt = jarBuild((Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString +: jars :+ manyJar.toString).mkString(separator), manyWork).run()
    val manyRefused = classPathZip64(manyJar.toString, Some(JarRun.Refused(manyBuilt.status, manyBuilt.out.linesIterator.toVector, Vector.empty))) && !Files.exists(manyWork)
    if manyRefused then pass += 1
    else
      fail += 1
      println(s"FAIL a jar of 65,537 entries (Python's zipfile): exit ${manyBuilt.status}, ${manyBuilt.out.linesIterator.find(_.nonEmpty).getOrElse("").take(200)}, " +
        s"the work directory ${if Files.exists(manyWork) then "written" else "not written"}; not the class path read's zip64 refusal")
    val crafted = jarCases.indices.filterNot(jarCases(_).tool)
    val craftedRows = crafted.filter(k => rowMarked(jarCases(k)))
    val craftedAgreeing = crafted.count(k => faultOf(k).isEmpty && !rowMarked(jarCases(k)))
    val toolRefusedAsRuled = toolZip64.map(jarCases(_).name) ++ Option.when(manyRefused)("tool-python-65537")
    if byTools.nonEmpty && toolAsScalaNative.length + toolZip64.length == byTools.length then pass += 1
    else
      fail += 1
      println(s"FAIL the jars real tools wrote: ${byTools.length - toolAsScalaNative.length - toolZip64.length} of ${byTools.length} neither as Scala Native's build makes " +
        s"them nor the class path read's zip64 refusal (${byTools.filterNot(k => toolAsScalaNative.contains(k) || toolZip64.contains(k)).map(jarCases(_).name).mkString(", ")})")
    // The JDK's refusals the build took as the class path read's, or as of a directory that read takes elsewhere.
    val refusedOtherwise = jarCases.indices.filter(k => jarCases(k).verdict.startsWith("refused")).filter(k => jarRuns.get(k).exists {
      case JarRun.Refused(_, lines, _) => lines.exists(l => l.startsWith(classPathLine) || l.contains(elsewhereLine))
      case _ => false
    }).map(jarCases(_).name)
    val clashes = jarCases.filter(_.verdict.startsWith("clash "))
    val byClassPathRead = jarCases.filter(_.verdict.startsWith("classpath "))
    val elsewhere = jarCases.filter(_.verdict.startsWith("teqread "))
    if jarFaults.isEmpty && jarCases.nonEmpty && jarRuns.size == jarCases.length then pass += 1
    else
      fail += 1
      println(s"FAIL the jar names: ${jarFaults.length} faults over ${jarCases.length} jars, ${jarRuns.size} judged")
      jarFaults.take(12).foreach(f => println(s"  $f"))
      if !togetherBuilt.ok then togetherBuilt.out.linesIterator.filter(_.nonEmpty).take(5).foreach(println)
    println(s"native-build jar names: ${jarCases.length} jars of ${jarLines.length} lines, all judged: ${jarRuns.size == jarCases.length}; by the JDK " +
      s"${refusing.size - clashes.length - byClassPathRead.length - elsewhere.length} refused, ${jarCases.count(_.verdict == "none")} of no library, " +
      s"${together.length - jarCases.count(_.verdict == "none")} unpacked; refused by teq alone: ${clashes.length} holding a file and a directory of one name " +
      s"(${clashes.map(_.name).mkString(", ")}), ${byClassPathRead.length} by its class path read (${byClassPathRead.map(_.name).mkString(", ")}), ${elsewhere.length} whose " +
      s"directory that read takes elsewhere (${elsewhere.map(_.name).mkString(", ")}); of the JDK's refusals ${refusedOtherwise.length} refused by teq's class path " +
      s"read or as of a directory it takes elsewhere (${refusedOtherwise.mkString(", ")}); ${jarFaults.length} faults, ${(System.nanoTime() - namesStarted) / 1000000000} s")
    val toolTotal = byTools.length + 1
    val numbered = Vector("none", "one", "two", "three", "four")
    println(s"native-build jar names, tool-written: ${toolAsScalaNative.length} of $toolTotal as Scala Native; the " +
      s"${numbered.lift(toolRefusedAsRuled.length).getOrElse(toolRefusedAsRuled.length.toString)}, a jar whose end record carries zip64's markers " +
      s"(Info-ZIP's forced zip64; any tool's jar of more than 65,535 entries), refused by the class-path read on every target before this part's code " +
      s"runs: the booked row (${toolRefusedAsRuled.mkString(", ")})")
    println(s"native-build jar names, crafted: ${crafted.length} judged, $craftedAgreeing agreeing, the rest rows (${craftedRows.map(jarCases(_).name).mkString(", ")})")

  if strandHas("programs") then
    // What a build refuses before it links.
    val glibc = build("hello", "hello", "--triple=x86_64-unknown-linux-gnu")
    if glibc.status == 1 && glibc.out.contains("not supported on native: an executable for x86_64-linux-gnu") then pass += 1
    else
      fail += 1
      println(s"FAIL a build for x86_64-linux-gnu: exit code ${glibc.status}, ${glibc.out.take(200)}")
    val unplaced = Sh(teq, "nir", "--link", "hello", "tests/native-build/fixtures/hello", s"--build=${out.resolve("unplaced")}").mergeErr.timeout(60).check(false).run()
    if unplaced.status == 1 && unplaced.out.contains("--build needs --work=<directory>") then pass += 1
    else
      fail += 1
      println(s"FAIL a build without its work directory: exit code ${unplaced.status}, ${unplaced.out.take(200)}")
    // A jar of the class path that is not there, named as the link names it (tests/native-link.scala), and nothing built.
    val absent = out.resolve("missing.jar").toString
    val withAbsent = (Paths.get("tests/native-build/fixtures/hello").toAbsolutePath.toString :: jars :+ absent).mkString(separator)
    val absentJar = Sh(teq, "nir", "--link", "hello", withAbsent, s"--build=${out.resolve("absent")}", s"--work=${out.resolve("work").resolve("absent")}",
      s"--zig=${zig.get}", s"--zig-cache=$cache").mergeErr.timeout(60).check(false).run()
    if absentJar.status == 1 && absentJar.out.trim == s"the class path, entry ${jars.length + 2}: $absent: no such jar" && !Files.exists(out.resolve("absent")) then pass += 1
    else
      fail += 1
      println(s"FAIL a build over a jar that is not there: exit code ${absentJar.status}, ${absentJar.out.take(300)}")

  if strandHas("misc") then
    // The trust roots' wraps over the stand-in for libcrypto, compiled and linked as a build compiles and links
    // static_musl.c, zig run in the cell's directory as a build runs it in its work directory: run from the tree, which
    // holds the pinned zig under target/cross, the same link builds a second C library of zig's into the shared cache.
    val trust = out.resolve("trust")
    Files.createDirectories(trust.resolve("empty-dir"))
    Files.writeString(trust.resolve("empty.pem"), "")
    Files.writeString(trust.resolve("one.pem"), "-----BEGIN CERTIFICATE-----\n")
    def cc(cmd: String*): Sh.Result =
      Sh((zig.get.toString +: "cc" +: "-target" +: "x86_64-linux-musl" +: cmd)*).in(trust)
        .withEnv("ZIG_GLOBAL_CACHE_DIR" -> cache.toString, "ZIG_LOCAL_CACHE_DIR" -> trust.resolve(localCache).toString)
        .mergeErr.timeout(300).check(false).run()
    val sources = Vector(
      ("src/native/static_musl.c", Vector("-O2", "-funwind-tables", "-fno-sanitize=undefined")),
      ("tests/native-build/libcrypto_stub.c", Vector("-std=c11", "-Wall", "-Wextra", "-Werror")),
      ("tests/native-build/trust_roots.c", Vector("-std=c11", "-Wall", "-Wextra", "-Werror")))
    val compiled = sources.map { (source, flags) =>
      val obj = trust.resolve(Paths.get(source).getFileName.toString + ".o").toString
      (cc((flags ++ Vector("-c", Paths.get(source).toAbsolutePath.toString, "-o", obj))*), obj)
    }
    val exe = trust.resolve("trust-roots").toString
    val linked =
      if compiled.forall(_._1.ok) then
        cc((Vector("-static", "-o", exe) ++ compiled.map(_._2) ++ Vector("-Wl,-wrap=pthread_create", "-Wl,-wrap=main",
          "-Wl,-wrap=X509_STORE_set_default_paths", "-Wl,-wrap=X509_verify_cert"))*)
      else compiled.find(!_._1.ok).get._1
    if !linked.ok then
      fail += 1
      println(s"FAIL the trust roots' check: zig cc exited ${linked.status}")
      linked.out.linesIterator.take(20).foreach(println)
    else
      val (empty, emptyDir, one) = (trust.resolve("empty.pem").toString, trust.resolve("empty-dir").toString, trust.resolve("one.pem").toString)
      val advice = "; install the system's CA certificates or set SSL_CERT_FILE or SSL_CERT_DIR\n"
      // A case runs with the variables `unset` taken out of its environment and `set` added, and its stderr has to
      // be as `stderr` says; its stdout `loads 0` (the stand-in's X509_STORE_load_locations, by which the wraps add a
      // distribution's place, never called) unless the distributions' places are `searched`.
      def check(label: String, unset: Seq[String], set: Seq[(String, String)], mode: String, searched: Boolean, stderr: String => Boolean): Unit =
        val ran = Sh((Vector("env") ++ unset.flatMap(v => Vector("-u", v)) ++ set.map((k, v) => s"$k=$v") ++ Vector(exe, mode))*).timeout(60).check(false).run()
        if ran.ok && (searched || ran.out == "loads 0\n") && stderr(ran.err) then pass += 1
        else
          fail += 1
          println(s"FAIL the trust roots, $label: exit code ${ran.status}, stdout ${ran.out.trim}, stderr ${ran.err.take(300)}")
      val defaults = Seq("STUB_DEFAULT_FILE" -> empty, "STUB_DEFAULT_DIR" -> emptyDir)
      check("empty places named", Nil, Seq("SSL_CERT_FILE" -> empty, "SSL_CERT_DIR" -> emptyDir) ++ defaults, "verify", false,
        _ == s"trust-roots: no CA certificates found in SSL_CERT_FILE=$empty, SSL_CERT_DIR=$emptyDir$advice")
      check("a file named that holds a certificate", Seq("SSL_CERT_DIR"), Seq("SSL_CERT_FILE" -> one) ++ defaults, "verify", false, _.isEmpty)
      check("nothing verified", Nil, Seq("SSL_CERT_FILE" -> empty, "SSL_CERT_DIR" -> emptyDir) ++ defaults, "never", false, _.isEmpty)
      // Neither variable set: the defaults and every distribution's place are searched, none holding a bundle here
      // whatever the machine holds, since the stand-in loads nothing.
      check("no place named", Seq("SSL_CERT_FILE", "SSL_CERT_DIR"), defaults, "verify", true,
        err => err.count(_ == '\n') == 1 && err.startsWith(s"trust-roots: no CA certificates found in $empty, $emptyDir, /etc/") && err.endsWith(advice))

  if strandHas("edits") then
    // zig's two caches (src/native/build.rs, `environment`): what zig builds of its own in its global cache, which every
    // build shares (--zig-cache), and the program's module objects in its local cache in the work directory
    // (`LOCAL_CACHE`). Fifty builds of hello world, each after a one-line change (its literal "hello " made "he000 ",
    // "he001 ", ...) and each printing the changed line, leave the shared cache as they found it, file for file and size
    // for size, and put fifty objects more in the work directory's; that work directory removed, the program built again
    // runs and the shared cache stays; two programs built at once over an empty shared cache both run, and it then holds
    // as many files as the suite's first build left in the shared cache, one libc.a and one libcompiler_rt.a among them
    // (zig's objects there carry the cache's own path, so their bytes are that directory's); a build over a shared cache
    // an older teq made (both of zig's caches in that one directory, through a zig script that sets ZIG_LOCAL_CACHE_DIR to
    // it) runs and leaves it as it was, and a build into a work directory such a teq built into (both caches in its
    // zig-cache) runs. The shared cache after the whole run against what the first build left is the parent's, once
    // every strand has ended. Fifty edits and the removed work directory are this strand; the two over caches of their
    // own the next. The cells here compare the shared cache's listing around their own builds beside the other strands'
    // builds over the same cache: a file another build adds meanwhile fails them, and no build removes one.

    val edits = Files.createDirectories(out.resolve("edits"))
    val editFixture = Files.createDirectories(edits.resolve("hello"))
    for f <- files(helloFixture) do Files.write(editFixture.resolve(f.getFileName.toString), Files.readAllBytes(f))
    val nir = editFixture.resolve("hello$package$.nir")
    val original = Files.readAllBytes(nir)
    val literal = Array[Byte](6) ++ "hello ".getBytes("UTF-8")
    val at = (0 to original.length - literal.length).find(i => literal.indices.forall(j => original(i + j) == literal(j))).getOrElse(-1)
    val (editWork, editExe) = (edits.resolve("work"), edits.resolve("hello-edited"))
    val editFirst = helloAt(editFixture, editWork, editExe, cache, zig.get)
    val beforeEdits = cacheListing(cache)
    val objectsBefore = localObjects(editWork)
    val editsRan = if at < 0 || !editFirst.ok then Vector.empty else (0 until 50).toVector.map { k =>
      val text = "he" + ("00" + k).takeRight(3) + " "
      val edited = original.clone()
      for j <- 0 until text.length do edited(at + 1 + j) = text(j).toByte
      Files.write(nir, edited)
      helloAt(editFixture, editWork, editExe, cache, zig.get).ok && prints(editExe, helloLine.replaceFirst("^hello ", text))
    }
    val afterEdits = cacheListing(cache)
    val objectsAdded = localObjects(editWork) - objectsBefore
    if editsRan.length == 50 && editsRan.forall(identity) && afterEdits == beforeEdits && objectsAdded == 50 then pass += 1
    else
      fail += 1
      println(s"FAIL fifty edits: the literal at $at, the first build exited ${editFirst.status}, ${editsRan.count(!_)} of ${editsRan.length} edited builds " +
        s"failed or printed otherwise; the shared cache: ${moved(beforeEdits, afterEdits)}; $objectsAdded objects more in the work directory's (50 wanted)")

    val removedWork = out.resolve("work").resolve("removed")
    val removedExe = out.resolve("removed-hello")
    val removedFirst = helloAt(helloFixture, removedWork, removedExe, cache, zig.get)
    val beforeRemoval = cacheListing(cache)
    Sh.remove(removedWork)
    val removedAgain = helloAt(helloFixture, removedWork, removedExe, cache, zig.get)
    val afterRemoval = cacheListing(cache)
    if removedFirst.ok && removedAgain.ok && prints(removedExe, helloLine) && afterRemoval == beforeRemoval && localObjects(removedWork) > 0 then pass += 1
    else
      fail += 1
      println(s"FAIL a work directory removed and built again: exit codes ${removedFirst.status} and ${removedAgain.status}; the shared cache: " +
        s"${moved(beforeRemoval, afterRemoval)}; ${localObjects(removedWork)} objects in the work directory's")
    println(s"native-build zig's caches: fifty edits added $objectsAdded objects to their work directory's and ${afterEdits.size - beforeEdits.size} files to the shared cache")

  if strandHas("caches") then
    val firstFiles = Files.readAllLines(firstListing).size()
    val atOnce = Files.createDirectories(out.resolve("at-once"))
    val atOnceShared = atOnce.resolve("shared")
    def startedAt(name: String, main: String): Sh.Running =
      Sh(teq, "nir", "--link", main, (Paths.get(s"tests/native-build/fixtures/$name").toAbsolutePath.toString :: jars).mkString(separator),
        s"--build=${atOnce.resolve(name)}", s"--work=${atOnce.resolve("work").resolve(name)}", s"--zig=${zig.get}", s"--zig-cache=$atOnceShared")
        .mergeErr.timeout(900).check(false).spawn()
    val startedTogether = Vector(startedAt("hello", "hello"), startedAt("collections", "CollectionsBench"))
    val together = startedTogether.map(_.await())
    val sharedTogether = cacheListing(atOnceShared)
    def named(name: String): Int = sharedTogether.keySet.count(k => Paths.get(k).getFileName.toString == name)
    if together.forall(_.ok) && runsAt(atOnce.resolve("hello"), "tests/native-build/hello.expected") &&
        runsAt(atOnce.resolve("collections"), "tests/cases/bench_collections.expected") && named("libc.a") == 1 && named("libcompiler_rt.a") == 1 &&
        sharedTogether.size == firstFiles then pass += 1
    else
      fail += 1
      println(s"FAIL two programs built at once over an empty shared cache: exit codes ${together.map(_.status).mkString(" and ")}; ${named("libc.a")} libc.a, " +
        s"${named("libcompiler_rt.a")} libcompiler_rt.a; ${sharedTogether.size} files against the first build's $firstFiles")
      together.flatMap(_.out.linesIterator.filter(_.nonEmpty).take(3)).foreach(println)

    val older = Files.createDirectories(out.resolve("older"))
    val olderShared = older.resolve("shared")
    val olderZig = older.resolve("zig")
    Files.writeString(olderZig, s"#!/bin/sh\nZIG_LOCAL_CACHE_DIR=\"$$ZIG_GLOBAL_CACHE_DIR\" exec ${shellQuoted(zig.get.toString)} \"$$@\"\n")
    Sh("chmod", "755", olderZig.toString).run()
    val byOlder = helloAt(helloFixture, older.resolve("work-older"), older.resolve("hello-older"), olderShared, olderZig)
    val olderLeft = cacheListing(olderShared)
    val overOlder = helloAt(helloFixture, older.resolve("work-newer"), older.resolve("hello-newer"), olderShared, zig.get)
    val olderWork = older.resolve("work-older-default")
    val intoOlder = helloAt(helloFixture, olderWork, older.resolve("hello-older-default"), olderWork.resolve(localCache), olderZig)
    val intoOlderAgain = helloAt(helloFixture, olderWork, older.resolve("hello-older-default"), cache, zig.get)
    if byOlder.ok && overOlder.ok && prints(older.resolve("hello-newer"), helloLine) && cacheListing(olderShared) == olderLeft &&
        localObjects(older.resolve("work-newer")) > 0 && intoOlder.ok && intoOlderAgain.ok && prints(older.resolve("hello-older-default"), helloLine) then pass += 1
    else
      fail += 1
      println(s"FAIL a build over what an older teq left: exit codes ${byOlder.status} and ${overOlder.status} over its shared cache (${moved(olderLeft, cacheListing(olderShared))}), " +
        s"${intoOlder.status} and ${intoOlderAgain.status} into its work directory")

  println(s"native-build strand ${strand.get}: $pass passed, $fail failed")
  Script.exit(if fail == 0 then 0 else 1)
}
