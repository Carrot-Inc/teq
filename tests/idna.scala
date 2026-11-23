//> using file ../tools/script
// The native target's IDNA library (src/native/idn2/idna.c) against GNU libidn2 2.3.8's answers, tests/idna/expected.txt
// (tests/idna/README says how they were recorded). The library is compiled as a native build compiles it, by zig 0.16.0
// for x86_64-linux-musl with the build's flags, a diagnostic failing the suite, and linked with the driver
// tests/idna/driver.c into a static executable, which converts every name of the table in one run; each row passes
// where its return code and output are GNU's. The zig is the one TEQ_ZIG names, else zig 0.16.0 on the PATH, else the
// one bench/zig.sh fetches (target/cross/zig-x86_64-linux-0.16.0/zig); zig's cache is the run's own, under its output
// (out/idna/zig-cache), so that no run leaves anything in a cache another run or machine shares. Without such a zig,
// or on a host other than x86_64 Linux, nothing runs: a `skip` line says why. Run from the repository's root:
// `./teq interp tests/idna.scala`.
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Path, Paths}

val zigVersion = "0.16.0"
val flags = Vector("-target", "x86_64-linux-musl", "-std=c99", "-O2", "-g0", "-fPIC", "-ffunction-sections", "-fdata-sections",
  "-Wall", "-Wextra")

final case class Row(group: String, input: String, answer: String)

def version(zig: Path): Option[String] =
  val said = Sh(zig.toString, "version").timeout(30).check(false).run()
  if said.ok then Some(said.out.trim) else None

/// The table: `# <group>` before each group's rows, a row `<input hex>\t<code>\t<output hex>`.
def table(file: Path): Vector[Row] =
  var group = ""
  val rows = Vector.newBuilder[Row]
  for line <- new String(Files.readAllBytes(file), UTF_8).split("\n") do
    if line.startsWith("# ") then group = line.substring(2)
    else
      val tab = line.indexOf('\t')
      rows += Row(group, line.substring(0, tab), line.substring(tab + 1))
  rows.result()

/// A name for a failure line: its bytes as UTF-8, a control character or an ill-formed byte escaped.
def shown(hex: String): String =
  val bytes = java.util.HexFormat.of().parseHex(hex)
  val text = new String(bytes, UTF_8)
  if !java.util.Arrays.equals(text.getBytes(UTF_8), bytes) then s"<bytes $hex>"
  else text.flatMap(c => if c < ' ' || c == '\u007f' then "\\u00" + (if c < 16 then "0" else "") + Integer.toHexString(c) else c.toString)

@main def idna(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/idna.scala")
  args.exactly(0)
  val expected = Paths.get("tests/idna/expected.txt")
  if !Files.isRegularFile(expected) then Log.die("tests/idna.scala runs from the repository's root")

  val host = s"${System.getProperty("os.name")} ${System.getProperty("os.arch")}"
  if host != "Linux amd64" then
    println(s"skip idna: the driver is an x86_64 Linux executable, not one of $host")
    println("idna: 0 passed, 0 failed")
    Script.exit(0)
  val named = Option(System.getenv("TEQ_ZIG")).filter(_.nonEmpty).map(Paths.get(_).toAbsolutePath)
  for zig <- named do
    val found = version(zig)
    if !found.contains(zigVersion) then
      println(s"FAIL TEQ_ZIG names $zig, which is ${found.fold("no zig")(v => s"zig $v")}, not zig $zigVersion")
      println("idna: 0 passed, 1 failed")
      Script.exit(1)
  val pinned = s"target/cross/zig-x86_64-linux-$zigVersion/zig"
  val zig = named.orElse((Sh.which("zig").toVector :+ Paths.get(pinned).toAbsolutePath).find(z => Files.isExecutable(z) && version(z).contains(zigVersion)))
  if zig.isEmpty then
    println(s"skip idna: no zig $zigVersion (TEQ_ZIG, zig on the PATH, or $pinned, which bench/zig.sh fetches)")
    println("idna: 0 passed, 0 failed")
    Script.exit(0)

  val work = Paths.get("out/idna").toAbsolutePath
  Sh.remove(work)
  Files.createDirectories(work)
  val cacheDir = work.resolve("zig-cache").toString
  def cc(cmd: String*): Sh.Result =
    Sh((zig.get.toString +: "cc" +: cmd)*).withEnv("ZIG_GLOBAL_CACHE_DIR" -> cacheDir, "ZIG_LOCAL_CACHE_DIR" -> cacheDir)
      .timeout(300).check(false).run()
  var pass = 0
  var fail = 0

  // The library as the native build compiles it: no diagnostic.
  val library = cc((flags ++ Vector("-c", "src/native/idn2/idna.c", "-o", work.resolve("idna.o").toString))*)
  if library.ok && library.errAll().isEmpty then pass += 1
  else
    fail += 1
    println(s"FAIL src/native/idn2/idna.c: zig cc exited ${library.status} with a diagnostic")
    library.errTail.linesIterator.take(20).foreach(println)
  val driver = work.resolve("driver")
  val linked = cc("-target", "x86_64-linux-musl", "-std=c99", "-O2", "-g0", "-Wall", "-Wextra", "-Werror",
    "-Isrc/native/idn2", "-static", "tests/idna/driver.c", work.resolve("idna.o").toString, "-o", driver.toString)
  if !linked.ok then
    println(s"FAIL tests/idna/driver.c: zig cc exited ${linked.status}")
    linked.errTail.linesIterator.take(20).foreach(println)
    println(s"idna: $pass passed, ${fail + 1} failed")
    Script.exit(1)

  val rows = table(expected)
  val inputs = work.resolve("inputs.hex")
  Files.writeString(inputs, rows.map(_.input + "\n").mkString)
  val run = Sh(driver.toString).stdinFrom(inputs).timeout(120).check(false).run()
  val answers = run.outAll().split("\n", -1).dropRight(1)
  if !run.ok || answers.length != rows.length then
    println(s"FAIL the driver exited ${run.status} after ${answers.length} of ${rows.length} rows")
    run.errTail.linesIterator.take(5).foreach(println)
    println(s"idna: $pass passed, ${fail + 1} failed")
    Script.exit(1)
  var differing = 0
  for (row, answer) <- rows.zip(answers) do
    if answer == row.answer then pass += 1
    else
      differing += 1
      if differing <= 20 then
        println(s"FAIL ${row.group}: ${shown(row.input)} (${row.input}): GNU libidn2 answers ${row.answer.replace('\t', ' ')}, the library ${answer.replace('\t', ' ')}")
  if differing > 20 then println(s"FAIL ${differing - 20} rows more differ")
  fail += differing
  println(s"idna: $pass passed, $fail failed")
  Script.exit(if fail == 0 then 0 else 1)
}
