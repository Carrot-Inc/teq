//> using file ../tools/script
// `teq fix` (docs/FIX.md, docs/CLI.md): every case of tests/fix/<Rule>/<Case> (tests/fix/REFERENCES.md) run by
// `teq compiler fix` over a copy of its input.scala, its header comment the configuration as scalafix's testkit reads
// it (a `file:` entry made absolute from the case's directory), its `jars` the class path (tests/support/jars.sh):
// the file it leaves is the case's output.scala (its input where it has none), the lints it prints are its
// expected.txt, and those lints are the fixture's own assertions (`// assert: <id>` on a line, `/* assert: <id>` with
// a caret line and a message), no more and no fewer; the exit is 1 where an error remains, else 0; a second run over
// what the first left changes nothing and prints the same; `--check` over the input changes nothing and exits 1
// where there is an edit or a lint. A script's case is `input.sc` and `output.sc`. A case with a `refused.txt` is one the run refuses: exit 2, that text among what it
// prints, every file as it was. A build's case, a directory with a teq.lock, is run by `teq fix` twice (below). Then the verb's own cases (`verb`), the rules' phase over several workers (`parallel`) and the jobs' typing over several workers (`typing`), below. `--record` writes each expected.txt from
// a run whose lints match their assertions. Run from the repository's root:
// `./teq interp tests/fix.scala [-- --teq <binary>] [--record] [--only <Rule/Case prefix>]`.
import java.nio.file.{Files, Path, Paths, StandardCopyOption}
import scala.jdk.CollectionConverters.*

@main def fix(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/fix.scala [--teq <binary>] [--record] [--only <prefix>]")
  val teq = Paths.get(args.teq("./target/release/teq")).toAbsolutePath.toString
  val record = args.flag("record")
  val only = args.option("only")
  args.exactly(0)
  val root = Paths.get("tests/fix")
  if !Files.isDirectory(root) then Log.die("tests/fix.scala runs from the repository's root")
  val work = Files.createTempDirectory("teq-fix-suite")
  Script.atExit(Sh.remove(work))
  var pass = 0
  var fail = 0
  def bad(name: String, why: String): Unit =
    println(s"FAIL $name: $why")
    fail += 1

  val cases: Vector[Path] =
    Files.list(root).iterator.asScala.toVector.filter(Files.isDirectory(_)).filterNot(_.getFileName.toString == "support").sorted
      .flatMap(r => Files.list(r).iterator.asScala.toVector.filter(c => Files.exists(c.resolve("input.scala")) || Files.exists(c.resolve("input.sc"))).sorted)
      .filter(c => only.forall(o => root.relativize(c).toString.replace('\\', '/').startsWith(o)))

  for c <- cases do
    val name = root.relativize(c).toString.replace('\\', '/')
    // A script's case is `input.sc` and `output.sc`.
    val (inputName, outputName) = if Files.exists(c.resolve("input.sc")) then ("input.sc", "output.sc") else ("input.scala", "output.scala")
    val input = read(c.resolve(inputName))
    val expectedOut = if Files.exists(c.resolve(outputName)) then read(c.resolve(outputName)) else input
    val conf = header(input, c.toAbsolutePath)
    val confFile = work.resolve(name.replace('/', '_') + ".conf")
    Files.writeString(confFile, conf)
    val classpath = jars(c)
    // The case's other sources, beside its input in the program.
    val extras: Vector[Path] = Files.list(c).iterator.asScala.toVector.filter(f => f.toString.endsWith(".scala") && !Set(inputName, outputName)(f.getFileName.toString)).sorted
    def runOn(check: Boolean): (Sh.Result, String) =
      val dir = work.resolve(name)
      Sh.remove(dir)
      Files.createDirectories(dir)
      Files.writeString(dir.resolve(inputName), input)
      for e <- extras do Files.copy(e, dir.resolve(e.getFileName))
      val r = command(teq, name, inputName, extras, confFile, classpath, check, work).run()
      (r, read(dir.resolve(inputName)))
    // A case the run refuses: `refused.txt` holds what it prints of why.
    val refused = Option.when(Files.exists(c.resolve("refused.txt")))(read(c.resolve("refused.txt")).trim)
    val (r1, out1) = runOn(false)
    val lints = r1.errAll().replace(work.toString + "/", "")
    val parsed = lintsOf(lints, name + "/" + inputName)
    val problems = scala.collection.mutable.ListBuffer.empty[String]
    if refused.nonEmpty then
      // Exit 2, the reason printed, every file as it was.
      if r1.status != 2 || !lints.contains(refused.get) then problems += s"exit ${r1.status}, expected 2 and `${refused.get}`:\n$lints"
      val kept = out1 == input && extras.forall(e => read(work.resolve(name).resolve(e.getFileName)) == read(e))
      if !kept then problems += "a refused run wrote a file"
    else if r1.status == 2 then problems += s"exit 2:\n${lints.linesIterator.take(20).mkString("\n")}"
    else
      // scalafix's testkit compares the output without the input's configuration comment, its surrounding blank
      // text aside (`stripTestkitComments`, `assertNoDiff`); a case with no output.scala leaves its input as it was.
      val outputMatches =
        if Files.exists(c.resolve(outputName)) then withoutHeader(out1).trim == expectedOut.trim else out1 == expectedOut
      if !outputMatches then problems += s"the output differs from $outputName:\n${diff(expectedOut, withoutHeader(out1))}"
      // The run's lints are at the lines of the file it leaves, whose assertions they answer.
      problems ++= checkAsserts(out1, parsed)
      val expectedStatus = if parsed.exists(_.severity == "error") then 1 else 0
      if r1.status != expectedStatus then problems += s"exit ${r1.status}, expected $expectedStatus"
      val expectedFile = c.resolve("expected.txt")
      if record && problems.isEmpty then
        if lints.nonEmpty then Files.writeString(expectedFile, lints) else Files.deleteIfExists(expectedFile)
      else
        val want = if Files.exists(expectedFile) then read(expectedFile) else ""
        if lints != want then problems += s"the lints differ from expected.txt:\n${diff(want, lints)}"
      // A case with a `scalameta.txt`, scalameta 4.17.4's tree of its input (support/MetaDump.scala): the tree its
      // DumpLayout lint prints is that one, node for node.
      val layout = c.resolve("scalameta.txt")
      if Files.exists(layout) then
        val printed = lints.linesIterator.dropWhile(!_.endsWith("BEGIN")).drop(1).takeWhile(_ != "END").mkString("\n")
        val want = read(layout).linesIterator.mkString("\n")
        if printed != want then problems += s"the tree differs from scalameta.txt's:\n${diff(want, printed)}"
      // A case with a `structure.txt`, scalameta 4.17.4's structure of its input (support/MetaStructure.scala): the
      // structure its DumpStructure lint prints is that one, every field of every node (the node contract).
      val structure = c.resolve("structure.txt")
      if Files.exists(structure) then
        val printed = lints.linesIterator.dropWhile(!_.endsWith("BEGIN")).drop(1).takeWhile(_ != "END").mkString("\n")
        val want = read(structure).linesIterator.mkString("\n")
        if printed != want then problems += s"the structure differs from structure.txt's:\n${diff(want, printed)}"
      // A second run over what the first left changes nothing and prints what the first printed of it.
      val dir = work.resolve(name)
      val r2 = command(teq, name, inputName, extras, confFile, classpath, false, work).run()
      if read(dir.resolve(inputName)) != out1 then problems += "a second run changed the file again"
      if out1 == input && r2.errAll().replace(work.toString + "/", "") != lints then problems += "a second run printed other lints"
      // `--check` changes nothing, and prints round one's lints at the input's lines, which its assertions answer.
      val (rc, outc) = runOn(true)
      problems ++= checkAsserts(input, lintsOf(rc.errAll().replace(work.toString + "/", ""), name + "/" + inputName)).map("--check: " + _)
      val checkStatus = if out1 != input || lints.linesIterator.exists(_.matches(".*: (error|warning|info): \\[.*")) then 1 else 0
      if outc != input then problems += "--check changed the file"
      if rc.status != checkStatus then problems += s"--check exited ${rc.status}, expected $checkStatus"
    if problems.isEmpty then pass += 1 else bad(name, problems.mkString("\n  "))

  // A build's case, a directory holding a teq.lock with the configurations and sources it names: `teq fix` in a copy of
  // it, twice. The first leaves the files `output/` holds (the others as they were) and prints `expected.txt`, exit 1
  // where an error is among it; the second, over what the first left, exits as the first and prints the same, as one
  // run reports what the next identical one does.
  val builds: Vector[Path] =
    Files.list(root).iterator.asScala.toVector.filter(Files.isDirectory(_)).filterNot(_.getFileName.toString == "support").sorted
      .flatMap(r => Files.list(r).iterator.asScala.toVector.filter(c => Files.exists(c.resolve("teq.lock"))).sorted)
      .filter(c => only.forall(o => root.relativize(c).toString.replace('\\', '/').startsWith(o)))
  for c <- builds do
    val name = root.relativize(c).toString.replace('\\', '/')
    val dir = work.resolve(name)
    Sh.remove(dir)
    val files = Files.walk(c).iterator.asScala.toVector.filter(Files.isRegularFile(_)).map(c.relativize).filterNot(f => f.startsWith("output") || f.toString == "expected.txt").sorted
    for f <- files do
      Files.createDirectories(dir.resolve(f).getParent)
      Files.copy(c.resolve(f), dir.resolve(f))
    def fixRun() = Sh(teq, "fix").in(dir).timeout(300).check(false).run()
    val r1 = fixRun()
    val lints = r1.errAll()
    val r2 = fixRun()
    val problems = scala.collection.mutable.ListBuffer.empty[String]
    val expectedStatus = if lints.linesIterator.exists(_.contains(": error: [")) then 1 else 0
    if r1.status != expectedStatus then problems += s"exit ${r1.status}, expected $expectedStatus:\n$lints"
    if r2.status != r1.status || r2.errAll() != lints then problems += s"the second run exited ${r2.status} printing\n${r2.errAll()}where the first exited ${r1.status} printing\n$lints"
    for f <- files if f.toString.endsWith(".scala") do
      val output = c.resolve("output").resolve(f)
      val want = read(if Files.exists(output) then output else c.resolve(f))
      if read(dir.resolve(f)) != want then problems += s"$f differs:\n${diff(want, read(dir.resolve(f)))}"
    val expectedFile = c.resolve("expected.txt")
    if record && problems.isEmpty then
      if lints.nonEmpty then Files.writeString(expectedFile, lints) else Files.deleteIfExists(expectedFile)
    else
      val want = if Files.exists(expectedFile) then read(expectedFile) else ""
      if lints != want then problems += s"the lints differ from expected.txt:\n${diff(want, lints)}"
    if problems.isEmpty then pass += 1 else bad(name, problems.mkString("\n  "))

  if only.forall(o => "verb".startsWith(o) || o.startsWith("verb")) then
    val (vp, _) = Verb.run(teq, work, bad)
    pass += vp
  if only.forall(o => "parallel".startsWith(o) || o.startsWith("parallel")) then
    val (pp, _) = Parallel.run(teq, work, bad)
    pass += pp
  if only.forall(o => "typing".startsWith(o) || o.startsWith("typing")) then
    val (tp, _) = Typing.run(teq, work, bad)
    pass += tp
  println(s"fix: $pass passed, $fail failed")
  Script.exit(if fail == 0 then 0 else 1)
}

def read(p: Path): String = new String(Files.readAllBytes(p), java.nio.charset.StandardCharsets.UTF_8)

def command(teq: String, name: String, input: String, extras: Vector[Path], conf: Path, classpath: Option[String], check: Boolean, dir: Path): Sh.Cmd =
  val files = (name + "/" + input) +: extras.map(e => name + "/" + e.getFileName)
  // A `// teq: <flags>` line of the input passes those flags (`--kind-projector`), as tests/run.sh's cases do.
  val flags = read(dir.resolve(name).resolve(input)).linesIterator.collectFirst { case l if l.startsWith("// teq: ") => l.stripPrefix("// teq: ").split(' ').toVector.filter(_.nonEmpty) }.getOrElse(Vector())
  val base = Vector(teq, "compiler", "fix") ++ files ++ Vector("--config", conf.toString) ++ classpath.toVector.flatMap(cp => Vector("--classpath", cp)) ++ flags ++ (if check then Vector("--check") else Vector())
  Sh(base*).in(dir).timeout(300).check(false)

/** The leading comment of a fixture, its configuration, with each `file:` entry made absolute from `dir`. */
def header(input: String, dir: Path): String =
  val start = input.indexOf("/*")
  val end = input.indexOf("*/")
  if start != 0 && input.substring(0, start).trim.nonEmpty || start < 0 || end < 0 then ""
  else
    val text = input.substring(start + 2, end)
    "\"file:([^\"]+)\"".r.replaceAllIn(text, m => java.util.regex.Matcher.quoteReplacement("\"file:" + dir.resolve(m.group(1)).normalize.toString.replace('\\', '/') + "\""))

/** The text without its leading configuration comment. */
def withoutHeader(text: String): String =
  if text.startsWith("/*") && text.indexOf("*/") > 0 then text.substring(text.indexOf("*/") + 2) else text

/** The case's class path: the jars its `jars` file names by their keys of tests/support/jars.sh. */
def jars(c: Path): Option[String] =
  val f = c.resolve("jars")
  if !Files.exists(f) then None
  else
    val keys = read(f).trim.split("\\s+").toVector.filter(_.nonEmpty)
    val paths = keys.map(k => Sh("bash", "-c", s". tests/support/jars.sh && jar_of $k").timeout(600).check(false).run().out.trim)
    Some(paths.mkString(java.io.File.pathSeparator))

final case class Printed(line: Int, col: Int, width: Int, severity: String, id: String, message: String)

/** The lints a run printed of `file`: `<file>:<line>:<col>: <severity>: [<id>]: <message>`, the source line, the carets. */
def lintsOf(out: String, file: String): Vector[Printed] =
  val lines = out.linesIterator.toVector
  val head = ("^" + java.util.regex.Pattern.quote(file) + ":(\\d+):(\\d+): (error|warning|info): \\[([^\\]]+)\\]:? ?(.*)$").r
  lines.indices.flatMap { i =>
    lines(i) match
      case head(line, col, sev, id, msg) =>
        val carets = lines.lift(i + 2).map(_.count(_ == '^')).getOrElse(1)
        Some(Printed(line.toInt, col.toInt, carets, sev, id, msg))
      case _ => None
  }.toVector

/** The fixture's assertions against the lints: each `assert: <id>` a lint of that id on its line, the caret line's
  * column and width and the message where it writes them; no lint without an assertion. */
def checkAsserts(input: String, lints: Vector[Printed]): Vector[String] =
  val lines = input.linesIterator.toVector
  val single = "//\\s*assert:\\s*(\\S+)".r
  val multi = "/\\*\\s*assert:\\s*(\\S+)".r
  final case class Assert(line: Int, id: String, caret: Option[(Int, Int)], message: Option[String])
  val asserts = lines.indices.flatMap { i =>
    val l = lines(i)
    multi.findFirstMatchIn(l).map { m =>
      val caretLine = lines.lift(i + 1).getOrElse("")
      val col = caretLine.indexOf('^')
      val width = caretLine.count(_ == '^')
      val rest = lines.drop(i + 2)
      val endAt = rest.indexWhere(_.contains("*/"))
      val msgLines = if endAt < 0 then rest else rest.take(endAt + 1)
      val msg = msgLines.mkString("\n").replace("*/", "").trim
      Assert(i + 1, m.group(1), if col >= 0 then Some((col + 1, width)) else None, Some(msg).filter(_.nonEmpty))
    }.orElse(single.findFirstMatchIn(l).map(m => Assert(i + 1, m.group(1), None, None)))
  }.toVector
  // scalafix's testkit (`AssertDiff`, `AssertDelta`): an assertion every lint of its line answers, a lint on no
  // asserted line unexpected, a lint and an assertion of one line that disagree (another id, caret or message) a
  // mismatch.
  val problems = scala.collection.mutable.ListBuffer.empty[String]
  for a <- asserts if !lints.exists(_.line == a.line) do problems += s"line ${a.line}: no lint ${a.id}"
  for l <- lints if !asserts.exists(_.line == l.line) do problems += s"line ${l.line}: an unasserted lint ${l.id}: ${l.message}"
  for a <- asserts; l <- lints if l.line == a.line do
    val correct = l.id == a.id && a.caret.forall { case (col, width) => l.col == col && l.width == width } && a.message.forall(_.trim == l.message.trim)
    if !correct then problems += s"line ${a.line}: ${l.id} at column ${l.col} width ${l.width} `${l.message.trim}` against the assertion ${a.id}${a.caret.fold("")(c => s" at column ${c._1} width ${c._2}")}${a.message.fold("")(m => s" `$m`")}"
  problems.toVector

/** The lines of `b` that `a` lacks and the other way round. */
def diff(a: String, b: String): String =
  val (as, bs) = (a.linesIterator.toVector, b.linesIterator.toVector)
  val minus = as.diff(bs).take(10).map("- " + _)
  val plus = bs.diff(as).take(10).map("+ " + _)
  (minus ++ plus).mkString("\n")

/** The verb's own cases: `teq fix` over a lock of two JVM projects, `app` over `core` (each check named). */
object Verb:
  val lock: String = lockOf(Nil)

  /** The lock, core's `excludes` those named and its generators those written (the lock's YAML items). */
  def lockOf(coreExcludes: List[String], coreGenerators: String = ""): String =
    lockWith(project("app", List("core")) + project("core", Nil, coreExcludes, coreGenerators))

  /** A lock of the projects `projects` writes (`project`'s blocks). */
  def lockWith(projects: String): String =
    """|teq: 0.1.9
       |format: 1
       |binaries: {}
       |inputs:
       |  files: {}
       |  sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
       |jars:
       |  org.scala-lang:scala-library:3.8.4: maven-central e44f14c38dba7da4d7050cbf72fbf9dcbb6f9e7d 9179090
       |  org.scala-lang:scala3-library_3:3.8.4: maven-central ada8b099f5425afff3e9164afd6601bed4ef2e21 319
       |java:
       |  outputVersion: 17
       |projects:
       |""".stripMargin + projects +
      """|repositories:
         |  - id: maven-central
         |    url: https://repo1.maven.org/maven2/
         |""".stripMargin

  /** A project's block; `scalafix` the fields of the compile configuration's `scalafix` record, as sbt-teq exports it,
    * and `root` its source root where it is not `<name>/src`. */
  def project(name: String, deps: List[String], excludes: List[String] = Nil, generators: String = "", files: List[String] = Nil, scalafix: List[String] = Nil, root: String = ""): String =
    val own = if root.isEmpty then s"$name/src" else root
    val cp = deps.map(d => s"          - {configuration: compile, project: $d}\n").mkString + files.map(f => s"          - {file: $f}\n").mkString
    val sources = (deps.map(d => s"        - $d/src\n") :+ s"        - $own\n").mkString
    s"""|  $name:
        |    base: $name
        |    configurations:
        |      compile:
        |        classpath:
        |$cp          - org.scala-lang:scala3-library_3:3.8.4
        |          - org.scala-lang:scala-library:3.8.4
        |        flags:
        |          ignoredScalacOptions: []
        |          kindProjector: false
        |          maxInlines: 32
        |          strictEquality: false
        |          werror: false
        |        generators:${if generators.isEmpty then " []" else "\n" + generators}
        |        mainClasses: []
        |        resources: []${if scalafix.isEmpty then "" else "\n        scalafix:" + scalafix.map(f => s"\n          $f").mkString}
        |        sources:
        |          - $own
        |    description:
        |      cacheableState: []
        |      excludes:${if excludes.isEmpty then " []" else excludes.map(e => s"\n        - $e").mkString}
        |      hot: false
        |      keys: {}
        |      macroState: ordered
        |      modulePerFile: []
        |      out: $name/target/teq/out
        |      production:
        |        excludes: []
        |        sources: []
        |      release: false
        |      sources:
        |$sources    platform: jvm
        |    scalaVersion: 3.8.4
        |""".stripMargin

  val coreSource = "package core\nimport scala.collection.mutable\nobject Core { val x = 1 }\n"
  val coreFixed = "package core\nobject Core { val x = 1 }\n"
  val appSource = "package app\nimport core.Core\nobject App { var counter = Core.x }\n"

  /** A fresh copy of the build, `files` beside the lock (relative path, text). */
  def build(work: Path, name: String, files: (String, String)*): Path =
    val dir = work.resolve("verb-" + name)
    Sh.remove(dir)
    val all = Seq("teq.lock" -> lock, "core/src/core/Core.scala" -> coreSource, "app/src/app/App.scala" -> appSource,
      ".scalafix.conf" -> "rules = [DisableSyntax, NoUnusedImports]\nDisableSyntax.noVars = true\n") ++ files
    for (f, text) <- all.toMap do
      Files.createDirectories(dir.resolve(f).getParent)
      Files.writeString(dir.resolve(f), text)
    dir

  def run(teq: String, work: Path, bad: (String, String) => Unit): (Int, Int) =
    var pass = 0
    var fail = 0
    def fix(dir: Path, args: String*): Sh.Result = Sh((Vector(teq, "fix") ++ args)*).in(dir).timeout(300).check(false).run()
    def expect(name: String, cond: Boolean, why: => String): Unit =
      if cond then pass += 1 else { fail += 1; bad("verb/" + name, why) }
    def text(dir: Path, f: String): String = read(dir.resolve(f))
    val varLint = "app/src/app/App.scala:3:14: error: [DisableSyntax.var]: mutable state should be avoided"

    // --check: the lint, the file to rewrite, nothing written, exit 1.
    val c = build(work, "check")
    val rc = fix(c, "--check")
    expect("check", rc.status == 1 && rc.errAll().contains(varLint) && rc.errAll().contains("core/src/core/Core.scala: would be rewritten") && text(c, "core/src/core/Core.scala") == coreSource,
      s"exit ${rc.status}: ${rc.errAll()}")
    // A run: core's unused import removed through dotty's edit, the error left, exit 1; a second run changes nothing.
    val r = build(work, "run")
    val r1 = fix(r)
    expect("fix", r1.status == 1 && text(r, "core/src/core/Core.scala") == coreFixed && text(r, "app/src/app/App.scala") == appSource && r1.errAll().contains(varLint), s"exit ${r1.status}: ${r1.errAll()}")
    val r2 = fix(r)
    expect("idempotent", r2.status == 1 && text(r, "core/src/core/Core.scala") == coreFixed && r2.errAll() == r1.errAll(), s"exit ${r2.status}: ${r2.errAll()}")
    // The jars' cache written as `teq compile` writes it, for the next run.
    val j = build(work, "jarcache")
    val cache = work.resolve("jarcache-dir")
    val rj = Sh(teq, "fix").in(j).withEnv("TEQ_CACHE_DIR" -> cache.toString).timeout(300).check(false).run()
    val cached = Files.exists(cache) && Files.walk(cache).iterator.asScala.exists(_.toString.endsWith(".jarcache"))
    expect("jar cache", rj.status == 1 && cached, s"exit ${rj.status}, a .jarcache under $cache: $cached: ${rj.errAll()}")
    // A module: only its files.
    val m = build(work, "module")
    val rm = fix(m, "core")
    expect("module", rm.status == 0 && text(m, "core/src/core/Core.scala") == coreFixed && !rm.errAll().contains("App.scala"), s"exit ${rm.status}: ${rm.errAll()}")
    // A file, named from a directory under the root: its lints alone, its path printed from there.
    val f = build(work, "file")
    val rf = Sh(teq, "fix", "src/app/App.scala").in(f.resolve("app")).timeout(300).check(false).run()
    expect("file", rf.status == 1 && rf.errAll().contains("src/app/App.scala:3:14: error") && text(f, "core/src/core/Core.scala") == coreSource, s"exit ${rf.status}: ${rf.errAll()}")
    // A file under a root that is no input: one the project excludes, one that is no source; each refused, exit 2.
    val ex = build(work, "excluded", "teq.lock" -> lockOf(List("core/src/core/Skipped.scala")),
      "core/src/core/Skipped.scala" -> "package core\nobject Skipped { var y = 1 }\n", "core/src/core/readme.txt" -> "notes\n")
    val rex = fix(ex, "core/src/core/Skipped.scala", "--check")
    expect("excluded file", rex.status == 2 && rex.errAll().contains("core/src/core/Skipped.scala is no source of the projects selected"), s"exit ${rex.status}: ${rex.errAll()}")
    val rtx = fix(ex, "core/src/core/readme.txt", "--check")
    expect("no source", rtx.status == 2 && rtx.errAll().contains("core/src/core/readme.txt is no source of the projects selected"), s"exit ${rtx.status}: ${rtx.errAll()}")
    // A generator's output beneath a root of core's own: typed, never edited, and no file to name (docs/CLI.md).
    val one = "package rules\nimport teq.fix.*\nfinal class One extends SyntacticRule(\"One\"):\n  override def fix(using doc: SyntacticDocument): Patch =\n    doc.tree.collect { case t: Lit.Int if t.syntax == \"1\" => Patch.replaceTree(t, \"2\") }.asPatch\n"
    val buildInfo = "          - kind: buildInfo\n            output: core/src/core/generated/BuildInfo.scala\n            package: generated\n            object: BuildInfo\n            static: {number: 1}"
    val gen = build(work, "generated", "teq.lock" -> lockOf(Nil, buildInfo), "core/src/core/Core.scala" -> "package core\nobject Core { val x = 1 }\n",
      "rules/One.scala" -> one, ".scalafix.conf" -> "rules = [\"file:rules/One.scala\"]\n")
    val rgen = fix(gen, "core")
    val info = gen.resolve("core/src/core/generated/BuildInfo.scala")
    val kept = Files.exists(info) && !read(info).contains("2")
    expect("generated untouched", rgen.status == 0 && kept && text(gen, "core/src/core/Core.scala").contains("val x = 2"), s"exit ${rgen.status}, BuildInfo kept: $kept: ${rgen.errAll()}")
    val rgf = fix(gen, "core/src/core/generated/BuildInfo.scala")
    expect("generated not named", rgf.status == 2 && rgf.errAll().contains("BuildInfo.scala is no source of the projects selected"), s"exit ${rgf.status}: ${rgf.errAll()}")
    // A relative `--config` from a directory under the root: its `file:` rules beside it, though the run works from the
    // build's root.
    // The record lists each document of each job with the lints that job gave it, before a file's lints are merged.
    val recordFile = work.resolve("verb-record.txt")
    val rrec = Sh(teq, "fix").in(build(work, "record")).withEnv("TEQ_FIX_RECORD" -> recordFile.toString).timeout(300).check(false).run()
    val recLines = if Files.exists(recordFile) then read(recordFile).linesIterator.toVector else Vector()
    val appDoc = recLines.exists(l => l.startsWith("doc job=") && l.contains(" key=app") && l.contains("/verb-record/.scalafix.conf ") && l.contains(" round=1 lints=1 ") && l.endsWith(" path=app/src/app/App.scala"))
    val appLint = recLines.exists(l => l.startsWith("lint job=") && l.contains(" round=1 line=3 col=14 ") && l.contains(" severity=error id=DisableSyntax.var path=app/src/app/App.scala message=\""))
    expect("record lints", rrec.status == 1 && appDoc && appLint, s"exit ${rrec.status}: ${recLines.filter(l => l.startsWith("doc") || l.startsWith("lint")).mkString("\n")}")
    // A configuration sbt-scalafix skips is left out unless named; one with a configuration file of its own runs under
    // it, the rest under the root's, both runs in the record.
    val skip = build(work, "scalafix-skip", "teq.lock" -> lockWith(project("app", List("core")) + project("core", Nil, scalafix = List("skip: true"))))
    val rskip = fix(skip, "--check")
    expect("scalafix skip", rskip.status == 1 && rskip.errAll().contains(varLint) && !rskip.errAll().contains("Core.scala"), s"exit ${rskip.status}: ${rskip.errAll()}")
    val rnamed = fix(skip, "core/compile")
    expect("scalafix skip named", rnamed.status == 0 && text(skip, "core/src/core/Core.scala") == coreFixed, s"exit ${rnamed.status}: ${rnamed.errAll()}")
    val own = build(work, "scalafix-config", "teq.lock" -> lockWith(project("app", List("core"), scalafix = List("config: app/.scalafix.conf", "skip: false")) + project("core", Nil)),
      "app/.scalafix.conf" -> "rules = [NoUnusedImports]\n")
    val ownRecord = work.resolve("verb-scalafix-config.txt")
    val rown = Sh(teq, "fix").in(own).withEnv("TEQ_FIX_RECORD" -> ownRecord.toString).timeout(300).check(false).run()
    val ownDocs = if Files.exists(ownRecord) then read(ownRecord).linesIterator.filter(_.startsWith("doc job=")).toVector else Vector()
    val underOwn = ownDocs.exists(l => l.contains("/verb-scalafix-config/app/.scalafix.conf ") && l.endsWith(" path=app/src/app/App.scala"))
    val underRoot = ownDocs.exists(l => l.contains("/verb-scalafix-config/.scalafix.conf ") && l.endsWith(" path=core/src/core/Core.scala"))
    expect("scalafix config", rown.status == 0 && text(own, "core/src/core/Core.scala") == coreFixed && !rown.errAll().contains("DisableSyntax") && underOwn && underRoot,
      s"exit ${rown.status}, app's document under its file: $underOwn, core's under the root's: $underRoot: ${rown.errAll()}\n${ownDocs.mkString("\n")}")
    val rflag = fix(own, "--check", "--config", ".scalafix.conf")
    expect("scalafix config flag", rflag.status == 1 && rflag.errAll().contains(varLint), s"exit ${rflag.status}: ${rflag.errAll()}")
    // The configuration files' parts are one run: a rule one of them lacks refuses it before another's edit is written,
    // and two that share a file must edit it alike, else nothing is written; with no part, an empty record.
    val emptyList = "package app\nobject App { val xs: List[Int] = List() }\n"
    def parts(name: String, lock: String, files: (String, String)*) = build(work, name, (Seq("teq.lock" -> lock, "a.conf" -> "rules = [EmptyCollectionsUnified]\n") ++ files)*)
    val lacking = parts("parts-lacking", lockWith(project("app", Nil, scalafix = List("config: a.conf", "skip: false")) + project("core", Nil, scalafix = List("config: b.conf", "skip: false"))),
      "b.conf" -> "rules = [MissingRule]\n", "app/src/app/App.scala" -> emptyList)
    val rlack = fix(lacking)
    expect("parts lacking", rlack.status == 2 && rlack.errAll().contains("unknown rule MissingRule") && text(lacking, "app/src/app/App.scala") == emptyList, s"exit ${rlack.status}: ${rlack.errAll()}")
    val shared = parts("parts-shared", lockWith(project("a", Nil, scalafix = List("config: a.conf", "skip: false"), root = "shared/src") + project("b", Nil, scalafix = List("config: b.conf", "skip: false"), root = "shared/src")),
      "b.conf" -> "rules = [DisableSyntax]\n", "shared/src/app/App.scala" -> emptyList)
    val rshared = fix(shared)
    expect("parts shared", rshared.status == 2 && rshared.errAll().contains("shared/src/app/App.scala: the configurations that share the file edit it differently") && text(shared, "shared/src/app/App.scala") == emptyList,
      s"exit ${rshared.status}: ${rshared.errAll()}")
    val skipped = build(work, "parts-none", "teq.lock" -> lockWith(project("app", List("core"), scalafix = List("skip: true")) + project("core", Nil, scalafix = List("skip: true"))))
    val skippedRecord = work.resolve("verb-parts-none.txt")
    Files.writeString(skippedRecord, "doc job=0 key=app/compile path=stale\n")
    val rskipped = Sh(teq, "fix", "--check").in(skipped).withEnv("TEQ_FIX_RECORD" -> skippedRecord.toString).timeout(300).check(false).run()
    expect("parts none", rskipped.status == 0 && read(skippedRecord).isEmpty, s"exit ${rskipped.status}, the record ${read(skippedRecord).take(200)}: ${rskipped.errAll()}")
    // `--rules` chooses among the rules the configuration discovers: a `file:` entry's rule by its name, and a shipped
    // rule alone with the `file:` rule left out.
    val counter = "package rules\nimport teq.fix.*\nfinal class Counter extends SyntacticRule(\"Counter\"):\n  override def fix(using doc: SyntacticDocument): Patch =\n    doc.tokens.find(_.text == \"counter\").map(t => Patch.lint(Diagnostic(\"found\", \"a counter\", t.pos))).asPatch\n"
    val sel = build(work, "rules-selection", ".scalafix.conf" -> "rules = [DisableSyntax, \"file:rules/Counter.scala\"]\nDisableSyntax.noVars = true\n", "rules/Counter.scala" -> counter)
    val rsel = fix(sel, "--rules", "Counter")
    expect("rules selection", rsel.status == 1 && rsel.errAll().contains("app/src/app/App.scala:3:18: error: [Counter.found]") && !rsel.errAll().contains("DisableSyntax"), s"exit ${rsel.status}: ${rsel.errAll()}")
    val rsel2 = fix(sel, "--rules", "DisableSyntax")
    expect("rules selection shipped", rsel2.status == 1 && rsel2.errAll().contains("[DisableSyntax.var]") && !rsel2.errAll().contains("Counter"), s"exit ${rsel2.status}: ${rsel2.errAll()}")
    val rc2 = build(work, "relative-config", "core/.scalafix.conf" -> "rules = [\"file:rules/R.scala\"]\n",
      "core/rules/R.scala" -> "package rules\nimport teq.fix.*\nfinal class R extends SyntacticRule(\"R\"):\n  override def fix(using doc: SyntacticDocument): Patch = Patch.empty\n")
    val rrc = Sh(teq, "fix", "--config", ".scalafix.conf", "--check").in(rc2.resolve("core")).timeout(300).check(false).run()
    expect("relative config", rrc.status == 0, s"exit ${rrc.status}: ${rrc.errAll()}")
    // A file no project of the selection holds.
    val o = build(work, "outside", "notes/Other.scala" -> "object Other\n")
    val ro = fix(o, "notes/Other.scala")
    expect("outside", ro.status == 2 && ro.errAll().contains("is no source of the projects selected"), s"exit ${ro.status}: ${ro.errAll()}")
    // Refusals, exit 2 each: an unknown module, an unknown rule, an unknown setting, a HOCON construct not read.
    val u = build(work, "unknown")
    val ru = fix(u, "nosuch")
    expect("unknown module", ru.status == 2 && ru.errAll().contains("no project nosuch"), s"exit ${ru.status}: ${ru.errAll()}")
    val rr = fix(u, "--rules", "Nope,DisableSyntax")
    expect("unknown rule", rr.status == 2 && rr.errAll().contains("unknown rule Nope"), s"exit ${rr.status}: ${rr.errAll()}")
    val s = build(work, "setting", ".scalafix.conf" -> "rules = [DisableSyntax]\nDisableSyntax.noVar = true\n")
    val rs = fix(s)
    expect("unknown setting", rs.status == 2 && rs.errAll().contains(".scalafix.conf:2:23: DisableSyntax.noVar: no such setting"), s"exit ${rs.status}: ${rs.errAll()}")
    val h = build(work, "hocon", ".scalafix.conf" -> "include url(\"https://example.com/x.conf\")\nrules = [DisableSyntax]\n")
    val rh = fix(h)
    expect("unsupported hocon", rh.status == 2 && rh.errAll().contains(".scalafix.conf:1:1: `include url(\"https://example.com/x.conf\")`: teq reads no URL"), s"exit ${rh.status}: ${rh.errAll()}")
    // The configuration's includes and substitutions as typesafe-config resolves them: an included file's regex list
    // extended by the including file's reference to it; an included file's error and its unread setting at their
    // places there.
    val inc = build(work, "include", ".scalafix.conf" -> "include \"base.conf\"\nDisableSyntax.regex = ${DisableSyntax.regex} [{ id = counter, pattern = \"counter\" }]\n",
      "base.conf" -> "rules = [DisableSyntax]\nDisableSyntax.regex = [{ id = core, pattern = \"Core\" }]\n")
    val ri = fix(inc)
    expect("include", ri.status == 1 && ri.errAll().contains("[DisableSyntax.core]") && ri.errAll().contains("app/src/app/App.scala:3:18: error: [DisableSyntax.counter]"), s"exit ${ri.status}: ${ri.errAll()}")
    val ie = build(work, "include-error", ".scalafix.conf" -> "include \"base.conf\"\n", "base.conf" -> "rules = [DisableSyntax]\nDisableSyntax.noVars = ${missing}\n")
    val rie = fix(ie)
    expect("include error", rie.status == 2 && rie.errAll().contains("base.conf:2:24: the substitution ${missing} has no value"), s"exit ${rie.status}: ${rie.errAll()}")
    val iu = build(work, "include-setting", ".scalafix.conf" -> "include \"base.conf\"\n", "base.conf" -> "rules = [DisableSyntax]\nDisableSyntax.noVar = true\n")
    val riu = fix(iu)
    expect("include setting", riu.status == 2 && riu.errAll().contains("base.conf:2:23: DisableSyntax.noVar: no such setting"), s"exit ${riu.status}: ${riu.errAll()}")
    // The severity filters, scalafix's LintConfig: an error made a warning leaves exit 0; the error filter wins.
    val w = build(work, "warning", ".scalafix.conf" -> "rules = [DisableSyntax]\nDisableSyntax.noVars = true\nlint.warning.includes = \"DisableSyntax\"\n")
    val rw = fix(w)
    expect("lint.warning", rw.status == 0 && rw.errAll().contains("app/src/app/App.scala:3:14: warning: [DisableSyntax.var]"), s"exit ${rw.status}: ${rw.errAll()}")
    val e = build(work, "error", ".scalafix.conf" -> "rules = [DisableSyntax]\nDisableSyntax.noVars = true\nlint.warning.includes = \".*\"\nlint.error.includes = \"DisableSyntax.var\"\n")
    val re = fix(e)
    expect("lint.error before lint.warning", re.status == 1 && re.errAll().contains(varLint), s"exit ${re.status}: ${re.errAll()}")
    // A program that does not type: no analysis, exit 2, nothing written.
    val t = build(work, "typing", "core/src/core/Broken.scala" -> "package core\nimport scala.collection.mutable\nobject Broken { val y: Int = \"no\" }\n")
    val rt = fix(t)
    expect("typing error", rt.status == 2 && rt.errAll().contains("Broken.scala") && text(t, "core/src/core/Core.scala") == coreSource, s"exit ${rt.status}: ${rt.errAll()}")
    // Two rules editing one tree differently: a conflict naming both, nothing written.
    val rename = "package rules\nimport teq.fix.*\nfinal class ListToNil extends SyntacticRule(\"ListToNil\"):\n  override def fix(using doc: SyntacticDocument): Patch =\n    doc.tree.collect { case t @ Term.Apply(Term.Name(\"List\"), Nil) => Patch.replaceTree(t, \"Nil\") }.asPatch\n"
    val k = build(work, "conflict", "core/src/core/Core.scala" -> "package core\nobject Core { val x: List[Int] = List() }\n", "rules/ListToNil.scala" -> rename,
      ".scalafix.conf" -> "rules = [EmptyCollectionsUnified, \"file:rules/ListToNil.scala\"]\n")
    val rk = fix(k, "core")
    expect("conflict", rk.status == 2 && rk.errAll().contains("the edits of EmptyCollectionsUnified and ListToNil overlap") && text(k, "core/src/core/Core.scala").contains("List()"), s"exit ${rk.status}: ${rk.errAll()}")
    // A rule's edit of core that app, which reads core, no longer types over (scalac's E007): round two types app too,
    // and its error refuses the whole plan, nothing written.
    val change = "package rules\nimport teq.fix.*\nfinal class Change extends SyntacticRule(\"Change\"):\n  override def fix(using doc: SyntacticDocument): Patch =\n    doc.tree.collect { case t: Lit.Int if t.syntax == \"1\" => Patch.replaceTree(t, \"\\\"hi\\\"\") }.asPatch\n"
    val plainCore = "package core\nobject Core { val x = 1 }\n"
    val dp = build(work, "dependent", "core/src/core/Core.scala" -> plainCore, "app/src/app/App.scala" -> "package app\nobject App { val x: Int = core.Core.x }\n",
      "rules/Change.scala" -> change, ".scalafix.conf" -> "rules = [\"file:rules/Change.scala\"]\n")
    val rdp = fix(dp)
    expect("dependent retyped", rdp.status == 2 && rdp.errAll().contains("does not type after the rules' edits") && rdp.errAll().contains("App.scala") && text(dp, "core/src/core/Core.scala") == plainCore,
      s"exit ${rdp.status}: ${rdp.errAll()}")
    // A file two jobs read (core selected twice): the rule fails in the second, which refuses the run.
    val failing = "package rules\nimport teq.fix.*\nfinal class Fail extends SyntacticRule(\"Fail\"):\n  private var seen = false\n  override def fix(using doc: SyntacticDocument): Patch =\n    if seen then throw new IllegalStateException(\"second context failed\")\n    seen = true\n    Patch.empty\n"
    val sh = build(work, "shared", "rules/Fail.scala" -> failing, ".scalafix.conf" -> "rules = [\"file:rules/Fail.scala\"]\n")
    val rsh = fix(sh, "core", "core", "--check")
    expect("shared file failure", rsh.status == 2 && rsh.errAll().contains("the rule Fail failed: java.lang.IllegalStateException: second context failed"), s"exit ${rsh.status}: ${rsh.errAll()}")
    // The verb over several workers (fix-speed's table, row 5.5): the lock's two projects, and a file two jobs read (core
    // selected twice), with the shipped rules, each job with 34 editable files so that its rounds run on the pool. At
    // eight workers, the barrier holding four of them, each run gives the one-worker run's bytes (both streams, the exit,
    // every file it leaves), and its record is a parallel one by `Parallel.recordProblems`: every round's documents and
    // generations complete, evaluations overlapping; the same run serialized (`TEQ_FIX_PLANT=serial`) fails that check.
    val many = (0 until 33).flatMap { i =>
      Seq(f"core/src/core/C$i%02d.scala" -> s"package core\nimport scala.collection.mutable\nobject C$i { val x = $i }\n",
        f"app/src/app/A$i%02d.scala" -> s"package app\nimport core.C$i\nobject A$i { var y = C$i.x }\n")
    }
    for (name, args) <- Seq("both" -> Seq[String](), "twice" -> Seq("core", "core")) do
      def go(tag: String, w: Int, env: (String, String)*) =
        val d = build(work, s"workers-$name-$tag", many*)
        val record = d.resolve("record.txt")
        val res = Sh((Vector(teq, "fix") ++ args)*).in(d).withEnv((Seq("TEQ_THREADS" -> w.toString, "TEQ_FIX_RECORD" -> record.toString) ++ env)*).timeout(300).check(false).run()
        val files = ("core/src/core/Core.scala" +: "app/src/app/App.scala" +: many.map(_._1)).map(f => f -> text(d, f)).toMap
        (res.status, res.outAll(), res.errAll(), files, if Files.exists(record) then read(record).linesIterator.toVector else Vector())
      val one = go("1", 1)
      val eight = go("8", 8, "TEQ_FIX_BARRIER" -> "4")
      val serialized = go("serial", 8, "TEQ_FIX_PLANT" -> "serial")
      val fixed = eight._4("core/src/core/C00.scala") == "package core\nobject C0 { val x = 0 }\n"
      expect(s"workers/$name", one._1 == eight._1 && one._2 == eight._2 && one._3 == eight._3 && one._4 == eight._4 && (one._1 == 0 || one._1 == 1) && fixed,
        s"exit ${one._1} and ${eight._1}; streams alike ${one._2 == eight._2 && one._3 == eight._3}, files alike ${one._4 == eight._4}, core's import removed $fixed:\n${diff(one._3, eight._3)}")
      val problems = Parallel.recordProblems(eight._5, 8, 4)
      val rounds = eight._5.filter(_.startsWith("round ")).map(Parallel.fieldsOf)
      expect(s"workers/$name/record", problems.isEmpty && rounds.size >= 2 && rounds.forall(_.getOrElse("docs", "0").toInt >= 33),
        (problems ++ rounds.map(r => s"round ${r.getOrElse("job", "?")}/${r.getOrElse("round", "?")}: ${r.getOrElse("docs", "?")} documents")).mkString("\n  "))
      expect(s"workers/$name/serialized-refused", Parallel.recordProblems(serialized._5, 8, 1).nonEmpty, "the record's check accepts the serialized run")
    // scalafix's `ImportPatchOps`: an added import goes after the file's leading imports, else right of
    // `fallbackToken`'s token: inside a package's braces, before the leading comment of a package's first statement
    // (scalafix's PatchSuite case), after a package clause's line; at the start of a file of comments alone. One the
    // file has written at top level is not added, so that round two adds nothing.
    val adder = "package rules\nimport teq.fix.*\nfinal class Add extends SyntacticRule(\"Add\"):\n  override def fix(using doc: SyntacticDocument): Patch =\n    Patch.addGlobalImport(\"scala.collection.mutable\") + Patch.addGlobalImport(\"scala.collection.immutable\")\n"
    val gi = build(work, "global", "rules/Add.scala" -> adder, ".scalafix.conf" -> "rules = [\"file:rules/Add.scala\"]\n",
      "core/src/core/Braces.scala" -> "package braces {\n  object X { val x = 1 }\n}\n",
      "core/src/core/Doc.scala" -> "package foo.bar\n\n/**\n * API docs here\n */\nobject A\n",
      "core/src/core/Flat.scala" -> "package flat\nobject F\n",
      "core/src/core/Empty.scala" -> "// empty\n")
    val rgi = fix(gi, "core")
    val both = "import scala.collection.mutable\nimport scala.collection.immutable"
    val placed = Seq(
      "Braces.scala" -> s"package braces {\n$both\n  object X { val x = 1 }\n}\n",
      "Doc.scala" -> s"package foo.bar\n\n$both\n/**\n * API docs here\n */\nobject A\n",
      "Flat.scala" -> s"package flat\n$both\nobject F\n",
      "Empty.scala" -> s"\n$both// empty\n",
      "Core.scala" -> "package core\nimport scala.collection.mutable\nimport scala.collection.immutable\nobject Core { val x = 1 }\n")
    val wrong = placed.filter((f, want) => text(gi, "core/src/core/" + f) != want).map((f, _) => s"$f:\n${text(gi, "core/src/core/" + f)}")
    expect("global imports", rgi.status == 0 && wrong.isEmpty, s"exit ${rgi.status}: ${rgi.errAll()}\n${wrong.mkString("\n")}")
    // A tree kept past its document: the rule's next document finds it refused, the rule failed, exit 2.
    val keep = "package rules\nimport teq.fix.*\nfinal class Keep extends SyntacticRule(\"Keep\"):\n  private var first: Option[Tree] = None\n  override def fix(using doc: SyntacticDocument): Patch =\n    first match\n      case None =>\n        first = Some(doc.tree)\n        Patch.empty\n      case Some(t) => Patch.lint(Diagnostic(\"\", t.syntax, doc.tree.pos))\n"
    val st = build(work, "stale", "rules/Keep.scala" -> keep, ".scalafix.conf" -> "rules = [\"file:rules/Keep.scala\"]\n")
    val rst = fix(st)
    expect("stale tree", rst.status == 2 && rst.errAll().contains("the rule Keep failed") && rst.errAll().contains("another document"), s"exit ${rst.status}: ${rst.errAll()}")
    // A position past a character beyond the BMP: `null` is byte 46, UTF-16 unit 44; the keyword's lint (a token) and the
    // regex's (an offset of the interpreter's String) stand at the same column.
    val n = build(work, "offsets", "core/src/core/Core.scala" -> "package core\nobject Core { val x = \"😀\"; val y: String = null }\n",
      ".scalafix.conf" -> "rules = [DisableSyntax]\nDisableSyntax.noNulls = true\nDisableSyntax.regex = [{ id = nullText, pattern = \"nul+\" }]\n")
    val rn = fix(n, "core")
    val nulls = rn.errAll().linesIterator.filter(_.startsWith("core/src/core/Core.scala:2:")).toVector
    expect("non-BMP offsets", rn.status == 1 && nulls.size == 2 && nulls.forall(_.startsWith("core/src/core/Core.scala:2:47:")), s"exit ${rn.status}: ${rn.errAll()}")
    // A write that fails leaves every original as it was: core's directory is read-only.
    if !System.getProperty("os.name").toLowerCase.contains("win") && System.getProperty("user.name") != "root" then
      val x = build(work, "readonly")
      val dir = x.resolve("core/src/core")
      Sh("chmod", "a-w", dir.toString).check(false).run()
      val rx = fix(x)
      Sh("chmod", "u+w", dir.toString).check(false).run()
      expect("write failure", rx.status == 2 && rx.errAll().contains("cannot write") && text(x, "core/src/core/Core.scala") == coreSource, s"exit ${rx.status}: ${rx.errAll()}")
    // The SemanticDB service (tests/semanticdb/TABLE.md, ruling 1): a semantic rule's questions answered from a typing of
    // the round's own, made at the round's first question; round two's from its own typing of round one's edits; a run
    // whose semantic rules ask nothing types nothing for it, and one with none starts no service; an answer the service
    // refuses or fails is the asking rule's failure, nothing written. The probe reads the document's header: its MD5,
    // the round's text's, and its uri, the path from where the run works.
    val probe = "package teq.fix.probe\nimport teq.fix.*\nfinal class Probe extends SemanticRule(\"Probe\"):\n  override def fix(using doc: SemanticDocument): Patch =\n    val shown = Native.docSymbols(doc.gen).map(b => if b >= 32 && b < 127 then b.toChar else ' ').mkString\n    val md5 = \"[0-9A-F]{32}\".r.findFirstIn(shown).getOrElse(\"none\")\n    Patch.lint(Diagnostic(\"\", s\"semanticdb $md5 ${shown.contains(doc.path)}\", doc.tree.pos))\n"
    def md5(s: String): String = java.security.MessageDigest.getInstance("MD5").digest(s.getBytes("UTF-8")).map(b => f"${b & 0xff}%02X").mkString
    def recorded(dir: Path, args: Seq[String], env: (String, String)*): (Sh.Result, Vector[String]) =
      val record = dir.resolve("record.txt")
      val res = Sh((Vector(teq, "fix") ++ args)*).in(dir).withEnv((("TEQ_FIX_RECORD" -> record.toString) +: env)*).timeout(300).check(false).run()
      (res, if Files.exists(record) then read(record).linesIterator.toVector else Vector())
    def served(rec: Vector[String]): Vector[(String, String, String)] =
      rec.filter(_.startsWith("serve ")).map(Parallel.fieldsOf).map(f => (f.getOrElse("round", "?"), f.getOrElse("typed", "?"), f.getOrElse("queries", "?")))
    val plain = "package core\nobject Core { val x = 1 }\n"
    val sv = build(work, "service", "core/src/core/Core.scala" -> plain, "rules/Probe.scala" -> probe, ".scalafix.conf" -> "rules = [\"file:rules/Probe.scala\"]\n")
    val (rsv, svRecord) = recorded(sv, Seq("core"))
    expect("service", rsv.status == 1 && rsv.errAll().contains(s"[Probe]: semanticdb ${md5(plain)} true") && served(svRecord) == Vector(("1", "1", "1")) && text(sv, "core/src/core/Core.scala") == plain,
      s"exit ${rsv.status}: ${rsv.errAll()}\n${svRecord.filter(l => l.startsWith("serve") || l.startsWith("done")).mkString("\n")}")
    val listed = "package core\nobject Core { val x: List[Int] = List() }\n"
    val rounds = Seq("core/src/core/Core.scala" -> listed, "rules/Probe.scala" -> probe, ".scalafix.conf" -> "rules = [EmptyCollectionsUnified, \"file:rules/Probe.scala\"]\n")
    val sr = build(work, "service-rounds", rounds*)
    val (rsr, srRecord) = recorded(sr, Seq("core"))
    val edited = text(sr, "core/src/core/Core.scala")
    expect("service/rounds", rsr.status == 1 && edited != listed && rsr.errAll().contains(s"[Probe]: semanticdb ${md5(edited)} true") && served(srRecord) == Vector(("1", "1", "1"), ("2", "1", "1")),
      s"exit ${rsr.status}: ${rsr.errAll()}\n${srRecord.filter(_.startsWith("serve")).mkString("\n")}")
    val idle = build(work, "service-idle", ".scalafix.conf" -> "rules = [NoUnusedImports]\n")
    val (ridle, idleRecord) = recorded(idle, Seq("core"))
    expect("service/idle", ridle.status == 0 && served(idleRecord).nonEmpty && served(idleRecord).forall((_, typed, queries) => typed == "0" && queries == "0") && text(idle, "core/src/core/Core.scala") == coreFixed, s"exit ${ridle.status}: ${ridle.errAll()}\n${idleRecord.filter(_.startsWith("serve")).mkString("\n")}")
    val none = build(work, "service-none", ".scalafix.conf" -> "rules = [DisableSyntax]\nDisableSyntax.noVars = true\n")
    val (rnone, noneRecord) = recorded(none, Seq())
    expect("service/none", rnone.status == 1 && served(noneRecord).isEmpty, s"exit ${rnone.status}: ${noneRecord.filter(_.startsWith("serve")).mkString("\n")}")
    val asker = "package teq.fix.probe\nimport teq.fix.*\nfinal class Ask extends SemanticRule(\"Ask\"):\n  override def fix(using doc: SemanticDocument): Patch =\n    Native.symbolsAt(doc.gen, 0, 7)\n    Patch.empty\n"
    val sa = build(work, "service-symbols", "core/src/core/Core.scala" -> plain, "rules/Ask.scala" -> asker, ".scalafix.conf" -> "rules = [\"file:rules/Ask.scala\"]\n")
    val rsa = fix(sa, "core")
    expect("service/symbols refused", rsa.status == 2 && rsa.errAll().contains("the rule Ask failed") && rsa.errAll().contains("not extracted yet"), s"exit ${rsa.status}: ${rsa.errAll()}")
    // The round's texts (the pass over c14fd75e1, Swap.scala): a file written between the document's dispatch and its
    // first question, its own or a dependency's the job does not edit, leaves the service's typing as the round read
    // the program: the answer's MD5 the round's text's, a dependency broken on the disk no failure.
    val swap = "package teq.fix.probe\nimport teq.fix.*\nimport java.nio.file.{Files, Paths}\nfinal class Swap extends SemanticRule(\"Swap\"):\n  override def fix(using doc: SemanticDocument): Patch =\n    val old = doc.input.text\n    val path = Paths.get(doc.path)\n    Files.writeString(path, old.replace(\"= 1\", \"= 2\"))\n    try\n      val shown = Native.docSymbols(doc.gen).map(b => if b >= 32 && b < 127 then b.toChar else ' ').mkString\n      Patch.lint(Diagnostic(\"\", \"semantic-md5=\" + \"[0-9A-F]{32}\".r.findFirstIn(shown).getOrElse(\"none\"), doc.tree.pos))\n    finally Files.writeString(path, old)\n"
    val ss = build(work, "service-snapshot", "core/src/core/Core.scala" -> plain, "rules/Swap.scala" -> swap, ".scalafix.conf" -> "rules = [\"file:rules/Swap.scala\"]\n")
    val rss = fix(ss, "core")
    expect("service/snapshot", rss.status == 1 && rss.errAll().contains(s"semantic-md5=${md5(plain)}") && text(ss, "core/src/core/Core.scala") == plain, s"exit ${rss.status}: ${rss.errAll()}")
    val breaker = "package teq.fix.probe\nimport teq.fix.*\nimport java.nio.file.{Files, Paths}\nfinal class Break extends SemanticRule(\"Break\"):\n  override def fix(using doc: SemanticDocument): Patch =\n    val core = Paths.get(\"core/src/core/Core.scala\")\n    val old = Files.readString(core)\n    Files.writeString(core, \"package core\\nobject Broken { val y: Int = \\\"no\\\" }\\n\")\n    try\n      Native.docSymbols(doc.gen)\n      Patch.lint(Diagnostic(\"\", \"answered\", doc.tree.pos))\n    finally Files.writeString(core, old)\n"
    val sb = build(work, "service-snapshot-dependency", "core/src/core/Core.scala" -> plain, "rules/Break.scala" -> breaker, ".scalafix.conf" -> "rules = [\"file:rules/Break.scala\"]\n")
    val rsb = fix(sb, "app")
    expect("service/snapshot-dependency", rsb.status == 1 && rsb.errAll().contains("[Break]: answered") && text(sb, "core/src/core/Core.scala") == plain, s"exit ${rsb.status}: ${rsb.errAll()}")
    val sf = build(work, "service-failure", rounds*)
    val (rsf, _) = recorded(sf, Seq("core"), "TEQ_FIX_PLANT" -> "query-failure")
    expect("service/failure", rsf.status == 2 && rsf.errAll().contains("the rule Probe failed") && rsf.errAll().contains("the extraction failed") && text(sf, "core/src/core/Core.scala") == listed,
      s"exit ${rsf.status}: ${rsf.errAll()}")
    (pass, fail)

/** The rules' phase over several workers (docs/FIX.md, "The run"): a generated corpus every shipped rule reports on and
  * edits (lints of two rules at one position, suppressions, an atomic patch dropped by one, an import round two removes),
  * run by `teq compiler fix` at 1, 4, 8 and 16 workers under the forced schedules (`TEQ_FIX_SCHEDULE`); each run's bytes
  * (the files it leaves, both streams with the trace's document lines and without its timing line, the exit) are the
  * one-worker run's, and its execution record (`TEQ_FIX_RECORD`) is a parallel one: every document of every round
  * dispatched once and done once with its slot's generation, by workers of the run's, at least `TEQ_FIX_BARRIER` of them
  * in rule evaluation before any finished, no round serial. Worker counts and times are no evidence; the record's
  * intervals are. The mutation controls (`TEQ_FIX_PLANT`) must fail: `serial` the record's check, `completion` the
  * bytes. A run with a project rule (`file:`) stays serial for the whole run: its record says so and its rule keeps its
  * state across documents, jobs and rounds (Count's visits, Stale's refusal of a patch of another document). */
object Parallel:
  /** Whether the engine runs the rules' phase in parallel; while it did not, the record's check had to refuse its runs
    * (the witness's negative control, which the `serial` plant keeps). */
  val engineParallel = true
  val counts = Vector(1, 4, 8, 16)
  val schedules = Vector("natural", "reverse", "shuffle:1", "shuffle:2", "shuffle:3")
  val conf: String =
    """|rules = [DisableSyntax, CatsImplicitsImport, TypelevelAs, EmptyCollectionsUnified, NoUnusedImports]
       |DisableSyntax.noVars = true
       |DisableSyntax.noNulls = true
       |DisableSyntax.noReturns = true
       |DisableSyntax.noWhileLoops = true
       |DisableSyntax.noAsInstanceOf = true
       |DisableSyntax.noIsInstanceOf = true
       |DisableSyntax.noUniversalEquality = true
       |DisableSyntax.noDefaultArgs = true
       |DisableSyntax.noSemicolons = true
       |DisableSyntax.regex = [
       |  { id = nullText, pattern = "null", message = "no null" }
       |  { id = mapConst, pattern = "List\\(1\\)\\.map\\(_ => 2\\)", message = "a map to a constant" }
       |]
       |lint.error.includes = ".*"
       |""".stripMargin

  /** The corpus's file `i`: its size and what it holds vary with `i`, so that documents take unequal times. */
  def source(i: Int): String =
    val name = f"F$i%03d"
    val used = i % 2 == 1
    val extra = (0 until (i % 5) * 6).map(k => s"  def d$k(a: Int = $k) = a == $k || a != ${k + 1}\n").mkString
    s"""|package par
        |import scala.collection.mutable.ListBuffer
        |import scala.collection.immutable.Nil
        |object $name:
        |  var counter = $i
        |  val xs = List()
        |  val ys: List[Int] = Nil
        |  val zs = List() // scalafix:ok EmptyCollectionsUnified
        |  def eq(a: Int) = a == $i || a != 0
        |  def cast(a: Any) = a.asInstanceOf[Int]
        |  def isIt(a: Any) = a.isInstanceOf[String]
        |  val n: String = null
        |  val mapped = List(1).map(_ => 2)
        |  // scalafix:off DisableSyntax.var
        |  var hidden = 0
        |  // scalafix:on
        |  def loop(): Unit = while counter < 0 do counter += 1
        |  def r(): Int = return 1
        |  val lb = ${if used then "ListBuffer(1)" else "1"}; val semi = 2
        |$extra""".stripMargin

  final case class Outcome(status: Int, out: String, err: String, files: Map[String, String], record: Vector[String])

  /** A copy of `files` under a directory of its own, the run's `teq compiler fix` over them, what it leaves. */
  def runOn(teq: String, work: Path, tag: String, files: Seq[(String, String)], confText: String, threads: Int, env: Seq[(String, String)], check: Boolean = false, extra: Seq[String] = Seq()): Outcome =
    val dir = work.resolve("parallel-" + tag)
    Sh.remove(dir)
    for (f, text) <- files do
      Files.createDirectories(dir.resolve(f).getParent)
      Files.writeString(dir.resolve(f), text)
    Files.writeString(dir.resolve("fix.conf"), confText)
    val record = dir.resolve("record.txt")
    val args = Vector(teq, "compiler", "fix") ++ files.map(_._1).filter(f => f.startsWith("src/") && (f.endsWith(".scala") || f.endsWith(".sc"))).sorted ++ Vector("--config", "fix.conf", "--threads", threads.toString) ++ extra ++ (if check then Vector("--check") else Vector())
    val r = Sh(args*).in(dir).withEnv((Seq("TEQ_FIX_TRACE" -> "1", "TEQ_FIX_RECORD" -> record.toString) ++ env)*).timeout(300).check(false).run()
    // The trace's last line holds the run's times, and a typing attempt's line its times and workers; the document lines
    // are the bytes' (they follow the consumption order).
    val err = r.errAll().linesIterator.filterNot(l => (l.startsWith("fix: ") && l.contains(" ms in all")) || l.startsWith("fix: typing ")).mkString("\n")
    val left = files.map((f, _) => f -> read(dir.resolve(f))).toMap
    Outcome(r.status, r.outAll(), err, left, if Files.exists(record) then read(record).linesIterator.toVector else Vector())

  def fieldsOf(line: String): Map[String, String] =
    line.split(" ").toVector.drop(1).flatMap(kv => kv.split("=", 2).toVector match
      case Vector(k, v) => Some(k -> v)
      case _ => None
    ).toMap

  /** What the record shows against a parallel run of `workers` with `barrier` workers held until they all began. */
  def recordProblems(rec: Vector[String], workers: Int, barrier: Int): Vector[String] =
    val problems = Vector.newBuilder[String]
    rec.find(_.startsWith("run ")).map(fieldsOf) match
      case None => problems += "no run line"
      case Some(run) =>
        if run.get("eligible") != Some("1") then problems += s"the run is not eligible: ${run}"
        if run.get("workers") != Some(workers.toString) then problems += s"the run's workers ${run.get("workers")}, expected $workers"
    if rec.exists(_.startsWith("barrier-timeout")) then problems += "a barrier timed out"
    // The rounds: from each `round` line to its `round-end`.
    var i = 0
    var rounds = 0
    while i < rec.length do
      if rec(i).startsWith("round ") then
        rounds += 1
        val head = fieldsOf(rec(i))
        val n = head.getOrElse("docs", "0").toInt
        val first = head.getOrElse("first_gen", "0").toInt
        val body = rec.drop(i + 1).takeWhile(l => !l.startsWith("round-end"))
        val where = s"round ${head.getOrElse("job", "?")}/${head.getOrElse("round", "?")}"
        if body.exists(_.startsWith("serial")) && n > 1 && workers > 1 then problems += s"$where: ${body.find(_.startsWith("serial")).get}"
        val dispatched = body.filter(_.startsWith("dispatch ")).map(fieldsOf)
        val done = body.filter(_.startsWith("done ")).map(fieldsOf)
        for (what, ls) <- Seq("dispatched" -> dispatched, "done" -> done) do
          val slots = ls.map(_.getOrElse("slot", "-1").toInt)
          if slots.sorted != (0 until n).toVector then problems += s"$where: the slots $what are ${slots.sorted.mkString(",")}, expected 0..${n - 1} once each"
          for l <- ls do
            val slot = l.getOrElse("slot", "-1").toInt
            if l.getOrElse("gen", "-1").toInt != first + slot then problems += s"$where: slot $slot $what with generation ${l.get("gen")}"
            val w = l.getOrElse("worker", "-1").toInt
            if w < 0 || w >= workers then problems += s"$where: slot $slot $what by worker $w of $workers"
        // Evaluations overlapping, read from the recorded intervals (not from the workers' ids).
        if n > 1 && workers > 1 then
          val spans = done.map(d => (d.getOrElse("start_us", "0").toLong, d.getOrElse("finish_us", "0").toLong))
          val events = (spans.map((a, _) => (a, 1)) ++ spans.map((_, b) => (b, -1))).sortBy(e => (e._1, e._2))
          var running = 0
          var peak = 0
          for (_, d) <- events do
            running += d
            peak = peak.max(running)
          if peak < 2 then problems += s"$where: its $n documents evaluated one at a time"
        // Several workers in rule evaluation before any finished.
        if n >= barrier && barrier > 1 && done.nonEmpty then
          val firstFinish = done.map(_.getOrElse("finish_us", "0").toLong).min
          val begun = done.filter(_.getOrElse("start_us", "0").toLong < firstFinish)
          val by = begun.map(_.getOrElse("worker", "-1")).distinct.size
          if by < barrier then problems += s"$where: $by workers began before the first document finished, expected $barrier"
        i += body.length + 1
      i += 1
    if rounds == 0 then problems += "no round"
    problems.result()

  def run(teq: String, work: Path, bad: (String, String) => Unit): (Int, Int) =
    var pass = 0
    var fail = 0
    def expect(name: String, cond: Boolean, why: => String): Unit =
      if cond then pass += 1 else { fail += 1; bad("parallel/" + name, why) }
    def sameBytes(a: Outcome, b: Outcome): Option[String] =
      if a.status != b.status then Some(s"exit ${b.status}, the one-worker run's ${a.status}")
      else if a.out != b.out then Some("stdout differs:\n" + diff(a.out, b.out))
      else if a.err != b.err then Some("stderr differs:\n" + diff(a.err, b.err))
      else a.files.find((f, t) => b.files.get(f) != Some(t)).map((f, t) => s"$f differs:\n" + diff(t, b.files.getOrElse(f, "")))
    val files = (0 until 48).map(i => f"src/par/F$i%03d.scala" -> source(i))
    val base = runOn(teq, work, "base", files, conf, 1, Seq())
    expect("base", base.status == 1 && base.err.contains("[DisableSyntax.var]") && base.files.exists((f, t) => !t.contains("immutable.Nil")), s"the one-worker run: exit ${base.status}\n${base.err.linesIterator.take(20).mkString("\n")}")
    for w <- counts.drop(1); s <- schedules do
      val o = runOn(teq, work, s"w$w-${s.replace(':', '-')}", files, conf, w, Seq("TEQ_FIX_SCHEDULE" -> s, "TEQ_FIX_BARRIER" -> "4"))
      expect(s"w$w/$s/bytes", sameBytes(base, o).isEmpty, sameBytes(base, o).getOrElse(""))
      val problems = recordProblems(o.record, w, 4)
      if engineParallel then expect(s"w$w/$s/record", problems.isEmpty, problems.mkString("\n  "))
      else expect(s"w$w/$s/record-refused", problems.nonEmpty, "the record's check accepts a serial engine's run")
    // The mutation controls: a serialized engine fails the record's check, a concatenation in completion order the bytes.
    if engineParallel then
      val serialized = runOn(teq, work, "plant-serial", files, conf, 8, Seq("TEQ_FIX_PLANT" -> "serial", "TEQ_FIX_BARRIER" -> "4"))
      expect("plant/serial", recordProblems(serialized.record, 8, 4).nonEmpty, "the record's check accepts a serialized run")
      val completion = runOn(teq, work, "plant-completion", files, conf, 8, Seq("TEQ_FIX_PLANT" -> "completion", "TEQ_FIX_SCHEDULE" -> "reverse"))
      expect("plant/completion", sameBytes(base, completion).nonEmpty, "the bytes of a run planned in completion order are the one-worker run's")
    // A refused run: the edits leave a program that does not type; every file as it was, at every count.
    val refusedFiles = files.take(8) :+ ("src/par/Refused.scala" -> "package par\nobject Refused:\n  def f(x: Nil.type) = x\n  val v = f(Nil)\n")
    val refused = counts.map(w => runOn(teq, work, s"refused-$w", refusedFiles, conf, w, Seq()))
    expect("refused", refused.head.status == 2 && refused.head.files == refusedFiles.toMap, s"exit ${refused.head.status}, files kept ${refused.head.files == refusedFiles.toMap}")
    for (o, w) <- refused.zip(counts).drop(1) do expect(s"refused/w$w", sameBytes(refused.head, o).isEmpty, sameBytes(refused.head, o).getOrElse(""))
    // A project rule keeps one rule thread for the whole run: Count's visits go on across documents and rounds, Stale's
    // patch of the first document refused in the second, at every count.
    val count = "package rules\nimport teq.fix.*\nfinal class Count extends SyntacticRule(\"Count\"):\n  private var n = 0\n  override def fix(using doc: SyntacticDocument): Patch =\n    n += 1\n    Patch.lint(Diagnostic(\"count\", n.toString, doc.tree.pos))\n"
    val stale = read(Paths.get("tests/fix/support/Stale.scala"))
    val stateFiles = Seq("src/A.scala" -> "object A { val old = 1; val l = List() }\n", "src/B.scala" -> "object B { val now = 2 }\n", "rules/Count.scala" -> count, "rules/Stale.scala" -> stale)
    val countConf = "rules = [EmptyCollectionsUnified, \"file:rules/Count.scala\"]\n"
    val staleConf = "rules = [\"file:rules/Stale.scala\"]\n"
    val counted = counts.map(w => runOn(teq, work, s"count-$w", stateFiles, countConf, w, Seq()))
    val countLints = "\\[Count\\.count\\]: (\\d+)".r.findAllMatchIn(counted.head.err).map(_.group(1)).toVector
    expect("count", countLints == Vector("3", "2"), s"Count's lints ${countLints.mkString(",")}, expected 3 (A, round two) and 2 (B)")
    for (o, w) <- counted.zip(counts).drop(1) do
      expect(s"count/w$w", sameBytes(counted.head, o).isEmpty, sameBytes(counted.head, o).getOrElse(""))
      expect(s"count/w$w/serial", o.record.exists(l => l.startsWith("run ") && fieldsOf(l).get("eligible") == Some("0")), o.record.take(3).mkString("\n"))
    val staled = counts.map(w => runOn(teq, work, s"stale-$w", stateFiles, staleConf, w, Seq()))
    expect("stale", staled.head.status == 2 && staled.head.err.contains("another document"), s"exit ${staled.head.status}\n${staled.head.err.linesIterator.take(5).mkString("\n")}")
    for (o, w) <- staled.zip(counts).drop(1) do expect(s"stale/w$w", sameBytes(staled.head, o).isEmpty, sameBytes(staled.head, o).getOrElse(""))
    (pass, fail)

/** The jobs' typing over several workers (docs/FIX.md, "The run"): each job and round typed as `teq compiler check`
  * types its program, its workers by compile's rule, a parallel attempt that gives way typed again at one worker
  * within its round. Every run's bytes (the files it leaves, both streams without the trace's timing lines, the exit)
  * are the one-worker run's, refusals (exit 2) compared as their own class; and its execution record's `typing` lines
  * say, per job, round and attempt, the count asked, whether the typing forked and its outcome (`one`, `joined`,
  * `retried`, `refused`); under `TEQ_FIX_TYPING_WORK=1` its `typing-item` lines say which typing worker typed each item
  * of the queue and when, the witness that several workers typed at once, read from the items' own intervals. The
  * witness corpus runs over typing {1,4,8,16} × rules {1,4,8,16} (`TEQ_FIX_TYPING_THREADS`, the rules' count
  * `--threads`); the plant `TEQ_FIX_PLANT=serial-typing`, which types every job at one worker and leaves the rules' pool
  * alone, must fail the typing check while it passes the rules' (`Parallel.recordProblems`), and `TEQ_SERIAL_ITEMS=1`,
  * one item at a time across forked workers, the items' check. Every rule worker started before a typing is ready
  * before it begins (`TEQ_FIX_BOOT_DELAY` holding their bootstraps), so that no bootstrap's typing runs beside it.
  * `--threads` reaches both verbs in both spellings; a refused mapping is recorded as one worker's typing in place.
  * The default getter's case (`tests/workers/default_getter_sealed`): in the assertion-enabled build a forked typing of a
  * method with a default argument aborts at a check of the parallel typer's (pinned as tests/workers.sh's
  * `default_getter_sealed`); there a run that aborts so, the witness corpus's among them, is pending on that pin. The counter macro library of
  * tests/fix/support/typing/counter, whose state a run changes, gives a parallel attempt way at 4, 8 and 16 in round
  * one, in round two and in a later job: each run the one-worker run's bytes, the project rule `Print`'s configuration
  * printed once, a refusal (`ZBad.scala`) every file as it was. The library is read from its products on the class path
  * (`lib/`, built by the suite), as a dependency's are: a macro of the program itself expands before the fork. The merge's semantic answers (tests/fix/support/typing:
  * `marks`, `withheld`, `records`, `blocks`, `phase-errors`, `round2`, and `NoUnusedImports/macro_expansion_block`) are
  * the one-worker typing's at every count. */
object Typing:
  /** Whether the engine types a job over several workers; while it did not, the typing check had to refuse its runs
    * (the witness's negative control, which the `serial-typing` plant keeps). */
  val engineParallelTyping = true
  val counts = Vector(1, 4, 8, 16)
  val fixtures: Path = Paths.get("tests/fix/support/typing")

  /** The witness corpus's file `i`: `Parallel.source(i)`, which every shipped rule reports on and edits, and objects of
    * methods enough that several workers' items overlap. */
  def witnessSource(i: Int): String =
    val objects = (0 until 4).map { o =>
      val defs = (0 until 30).map(k => s"  def m$k(a: Int, b: String): Int = if a > $k then (b.length + a * ${k + 1}) % 7 else m${(k + 1) % 30}(a + 1, b + \"$k\")\n").mkString
      s"object W${i}x$o:\n$defs"
    }.mkString
    Parallel.source(i) + objects

  /** A file of non-ASCII text, an edit after a character of two UTF-16 units, and a script's document. */
  val unicode = "src/par/Unicode.scala" -> "package par\nobject Ünïcode:\n  val ŝ = \"ñ😀\"; val xs = List()\n  var v = 1\n"
  val script = "src/par/Script.sc" -> "val ys = List()\nvar w = 2\nprintln(ys.size + w)\n"

  /** A run that ended in an allocation the system refused, the address space spent by an allocation and no thread's
    * start: Rust's "memory allocation of … failed", glibc's "out of memory" (a thread's destructors registered), an
    * arena's "no memory for a cell directory", and the signal stack Rust's std maps as a started thread begins
    * ("failed to allocate an alternative stack"), refused after the start succeeded and before any code of teq's. */
  def allocationRefused(r: Sh.Result): Boolean =
    val err = r.errAll()
    r.status == 134 && (err.contains("memory allocation of") || err.contains("out of memory") || err.contains("no memory for a cell directory")
      || err.contains("failed to allocate an alternative stack"))

  /** The files of a fixture directory, under `src/<prefix>/`. */
  def program(dir: String, prefix: String): Seq[(String, String)] =
    Files.list(fixtures.resolve(dir)).iterator.asScala.toVector.sorted.map(f => s"src/$prefix/${f.getFileName}" -> read(f))

  final case class Attempt(job: Int, round: Int, attempt: Int, asked: Int, effective: Int, mappingRefused: Boolean, workerRefused: Boolean, forked: Boolean, outcome: String, start: Long, why: String)

  /** The record's `typing` lines; `why=` is the rest of its line. */
  def attemptsOf(rec: Vector[String]): Vector[Attempt] =
    rec.filter(_.startsWith("typing ")).map { l =>
      val (head, why) = l.indexOf(" why=") match
        case -1 => (l, "")
        case at => (l.take(at), l.drop(at + 5))
      val f = Parallel.fieldsOf(head)
      def n(k: String) = f.getOrElse(k, "-1").toInt
      Attempt(n("job"), n("round"), n("attempt"), n("asked"), n("effective"), f.get("mapping_refused") == Some("1"), f.get("worker_refused") == Some("1"), f.get("forked") == Some("1"), f.getOrElse("outcome", ""), f.getOrElse("start_us", "-1").toLong, why)
    }

  final case class Item(job: Int, round: Int, attempt: Int, worker: Int, index: Int, t0: Long, t1: Long, file: String)

  /** The record's `typing-item` lines (`TEQ_FIX_TYPING_WORK=1`); `file=` is the rest of its line. */
  def itemsOf(rec: Vector[String]): Vector[Item] =
    rec.filter(_.startsWith("typing-item ")).map { l =>
      val (head, file) = l.indexOf(" file=") match
        case -1 => (l, "")
        case at => (l.take(at), l.drop(at + 6))
      val f = Parallel.fieldsOf(head)
      def n(k: String) = f.getOrElse(k, "-1").toLong
      Item(n("job").toInt, n("round").toInt, n("attempt").toInt, n("worker").toInt, n("index").toInt, n("t0_us"), n("t1_us"), file)
    }

  /** What the record shows against typing at `asked` workers: each job's and round's attempts, the terminal one last,
    * and a `round` line only after a terminal attempt that typed; `retried` the job-rounds whose parallel attempt is to
    * give way. */
  def typingProblems(rec: Vector[String], asked: Int, retried: Set[(Int, Int)] = Set()): Vector[String] =
    val problems = Vector.newBuilder[String]
    val attempts = attemptsOf(rec)
    if attempts.isEmpty then problems += "no typing line"
    val rounds = rec.filter(_.startsWith("round ")).map(Parallel.fieldsOf).map(f => (f.getOrElse("job", "-1").toInt, f.getOrElse("round", "-1").toInt))
    for (jr, as) <- attempts.groupBy(a => (a.job, a.round)).toVector.sortBy(_._1) do
      val where = s"job ${jr._1} round ${jr._2}"
      val seq = as.sortBy(_.attempt)
      if seq.map(_.attempt) != (1 to seq.size).toVector then problems += s"$where: attempts ${seq.map(_.attempt).mkString(",")}"
      val first = seq.head
      if first.asked != asked || first.effective != asked then problems += s"$where: asked ${first.asked}, effective ${first.effective}, $asked asked"
      if asked == 1 then
        if seq.size != 1 || first.forked || first.outcome != "one" then problems += s"$where: at one worker ${seq.mkString("; ")}"
      else if retried(jr) then
        if seq.size != 2 || !first.forked || first.outcome != "retried" || first.why.isEmpty then problems += s"$where: expected a parallel attempt that gave way, ${seq.mkString("; ")}"
        else if seq(1).asked != 1 || seq(1).forked || seq(1).outcome != "one" then problems += s"$where: the attempt after the give-way ${seq(1)}"
      else if seq.size != 1 || !first.forked || first.outcome != "joined" then problems += s"$where: expected one joined attempt, ${seq.mkString("; ")}"
    for jr <- rounds if !attempts.exists(a => (a.job, a.round) == jr) do problems += s"job ${jr._1} round ${jr._2}: documents with no typing line"
    problems.result()

  /** The witness that several typing workers typed at once: in each job-round's joined attempt, items of two workers
    * at least, and two of them in progress at one moment, read from the recorded intervals. */
  def workProblems(rec: Vector[String]): Vector[String] =
    val items = itemsOf(rec)
    val joined = attemptsOf(rec).filter(_.outcome == "joined")
    if joined.isEmpty then Vector("no joined attempt")
    else joined.flatMap { a =>
      val mine = items.filter(i => i.job == a.job && i.round == a.round && i.attempt == a.attempt)
      val workers = mine.map(_.worker).distinct.size
      val events = (mine.map(i => (i.t0, 1)) ++ mine.map(i => (i.t1, -1))).sortBy(e => (e._1, e._2))
      var running = 0
      var peak = 0
      for (_, d) <- events do
        running += d
        peak = peak.max(running)
      val where = s"job ${a.job} round ${a.round}"
      (if workers < 2 then Vector(s"$where: ${mine.size} items typed by $workers worker(s)") else Vector()) ++
        (if peak < 2 then Vector(s"$where: its items typed one at a time") else Vector())
    }

  /** The pool's readiness before every typing: each rule worker started before a typing began (`worker-start`) answered
    * `Ready` (its `worker` line) before it, so that no worker's bootstrap, whose one-worker typing writes the interpreter's
    * process-wide watch, runs beside a typing. */
  def barrierProblems(rec: Vector[String]): Vector[String] =
    val started = rec.filter(_.startsWith("worker-start ")).map(Parallel.fieldsOf).map(f => f.getOrElse("w", "-1").toInt -> f.getOrElse("at_us", "-1").toLong).toMap
    val ready = rec.filter(_.startsWith("worker ")).map(Parallel.fieldsOf).map(f => f.getOrElse("w", "-1").toInt -> f.getOrElse("ready_us", "-1").toLong).toMap
    val attempts = attemptsOf(rec)
    if attempts.isEmpty then Vector("no typing line")
    else if started.size < 2 then Vector(s"${started.size} worker(s) started: no typing after a later worker's start")
    else attempts.flatMap { a =>
      started.toVector.sorted.collect {
        case (w, at) if at < a.start && ready.get(w).forall(_ > a.start) =>
          s"job ${a.job} round ${a.round} attempt ${a.attempt} began at ${a.start} us, worker $w (started at $at) ready at ${ready.get(w).fold("never")(_.toString)}"
      }
    }

  def run(teq: String, work: Path, bad: (String, String) => Unit): (Int, Int) =
    var pass = 0
    var fail = 0
    def expect(name: String, cond: Boolean, why: => String): Unit =
      if cond then pass += 1 else { fail += 1; bad("typing/" + name, why) }
    def sameBytes(a: Parallel.Outcome, b: Parallel.Outcome): Option[String] =
      if a.status != b.status then Some(s"exit ${b.status}, the one-worker run's ${a.status}")
      else if a.out != b.out then Some("stdout differs:\n" + diff(a.out, b.out))
      else if a.err != b.err then Some("stderr differs:\n" + diff(a.err, b.err))
      else a.files.find((f, t) => b.files.get(f) != Some(t)).map((f, t) => s"$f differs:\n" + diff(t, b.files.getOrElse(f, "")))
    /** The typing check of a run at `asked` typing workers: passed by the parallel engine, refused by a serial one. */
    def typing(name: String, o: Parallel.Outcome, asked: Int, retried: Set[(Int, Int)] = Set(), witness: Boolean = false): Unit =
      val problems = typingProblems(o.record, asked, retried) ++ (if witness && asked > 1 then workProblems(o.record) else Vector())
      if engineParallelTyping then expect(name, problems.isEmpty, problems.mkString("\n  "))
      else if asked > 1 then expect(name + "/refused", problems.nonEmpty, "the typing check accepts a serial engine's run")
    // The assertion-enabled build aborts a forked typing of a method with a default argument (a worker changes the
    // default getter's record after a publication sealed it): the parallel typer's, pinned as tests/workers.sh's
    // `default_getter_sealed` for its own part. A run that aborts so there is pending on that pin, counted apart; any
    // other abort fails.
    val assertions = Sh(teq, "--version").check(false).run().outAll().trim.endsWith("assertions")
    var pending = 0
    def pinned(name: String, o: Parallel.Outcome): Boolean =
      val hit = assertions && o.status == 134 && o.err.contains("after a publication sealed it")
      if hit then
        pending += 1
        println(s"PENDING typing/$name: a default getter's record changed after its publication sealed it (tests/workers.sh's default_getter_sealed)")
      hit
    def typed(tag: String, files: Seq[(String, String)], conf: String, t: Int, r: Int, env: Seq[(String, String)] = Seq(), extra: Seq[String] = Seq()): Parallel.Outcome =
      Parallel.runOn(teq, work, "typing-" + tag, files, conf, r, (if t == r then Seq() else Seq("TEQ_FIX_TYPING_THREADS" -> t.toString)) ++ env, extra = extra)

    // The witness corpus over typing × rules: the bytes the 1×1 run's, the typing's record parallel where it asks
    // several workers, the rules' record parallel where they do.
    val corpus = (0 until 24).map(i => f"src/par/F$i%03d.scala" -> witnessSource(i)) :+ unicode :+ script
    val base = typed("t1-r1", corpus, Parallel.conf, 1, 1)
    expect("base", base.status == 1 && base.err.contains("[DisableSyntax.var]") && base.files("src/par/Unicode.scala").contains("List.empty") && base.files("src/par/Script.sc").contains("List.empty"),
      s"the one-worker run: exit ${base.status}\n${base.err.linesIterator.take(20).mkString("\n")}")
    for t <- counts; r <- counts if (t, r) != (1, 1) do
      val env = Seq("TEQ_FIX_TYPING_WORK" -> "1") ++ (if r > 1 then Seq("TEQ_FIX_BARRIER" -> (r min 4).toString) else Seq())
      val o = typed(s"t$t-r$r", corpus, Parallel.conf, t, r, env)
      if !pinned(s"t$t/r$r", o) then
        expect(s"t$t/r$r/bytes", sameBytes(base, o).isEmpty, sameBytes(base, o).getOrElse(""))
        typing(s"t$t/r$r/typing", o, t, witness = true)
        val rules = Parallel.recordProblems(o.record, r, r min 4)
        expect(s"t$t/r$r/rules", rules.isEmpty, rules.mkString("\n  "))
    // The control: the typing serial, the rules' pool parallel. The rules' check accepts it; the typing check must not.
    val planted = typed("plant-serial-typing", corpus, Parallel.conf, 8, 8, Seq("TEQ_FIX_PLANT" -> "serial-typing", "TEQ_FIX_TYPING_WORK" -> "1", "TEQ_FIX_BARRIER" -> "4"))
    expect("plant/serial-typing/bytes", sameBytes(base, planted).isEmpty, sameBytes(base, planted).getOrElse(""))
    expect("plant/serial-typing/rules", Parallel.recordProblems(planted.record, 8, 4).isEmpty, Parallel.recordProblems(planted.record, 8, 4).mkString("\n  "))
    expect("plant/serial-typing/refused", (typingProblems(planted.record, 8) ++ workProblems(planted.record)).nonEmpty, "the typing check accepts a run whose typing was serial")
    // The control the items alone tell: one item at a time across the workers (`TEQ_SERIAL_ITEMS=1`), the workers'
    // assignment unchanged, so that the attempt says it forked and the items name several workers.
    val serialItems = typed("serial-items", corpus, Parallel.conf, 8, 8, Seq("TEQ_SERIAL_ITEMS" -> "1", "TEQ_FIX_TYPING_WORK" -> "1", "TEQ_FIX_BARRIER" -> "4"))
    if !pinned("serial-items", serialItems) then
      expect("serial-items/bytes", sameBytes(base, serialItems).isEmpty, sameBytes(base, serialItems).getOrElse(""))
      expect("serial-items/refused", workProblems(serialItems.record).nonEmpty, "the items' check accepts a run whose items were typed one at a time")
    // The pool's readiness: round one's documents done by the lead while the others' bootstraps are held
    // (`TEQ_FIX_BOOT_DELAY`), round two's typing waits for every one of them.
    val ready = (0 until 4).map(i => f"src/rd/R$i.scala" -> s"package rd\nimport scala.Nil\nobject R$i { val empty: List[Int] = Nil }\n")
    val readyRun = typed("ready", ready, "rules = [EmptyCollectionsUnified, NoUnusedImports]\n", 4, 4, Seq("TEQ_FIX_BOOT_DELAY" -> "1500"))
    val readyRounds = readyRun.record.count(_.startsWith("round "))
    if engineParallelTyping then expect("ready", readyRounds == 2 && barrierProblems(readyRun.record).isEmpty, s"$readyRounds rounds\n  " + barrierProblems(readyRun.record).mkString("\n  "))
    else expect("ready/refused", barrierProblems(readyRun.record).nonEmpty, "the readiness check accepts the unchanged engine's run")
    if engineParallelTyping then
      // The two requests apart: typing at eight, the rules' pool at one.
      val apart = Parallel.runOn(teq, work, "typing-apart", corpus, Parallel.conf, 8, Seq("TEQ_FIX_RULE_THREADS" -> "1"))
      if !pinned("apart", apart) then
        expect("apart/bytes", sameBytes(base, apart).isEmpty, sameBytes(base, apart).getOrElse(""))
        expect("apart/typing", typingProblems(apart.record, 8).isEmpty, typingProblems(apart.record, 8).mkString("\n  "))
        expect("apart/rules", apart.record.exists(l => l.startsWith("run ") && Parallel.fieldsOf(l).get("workers") == Some("1")), apart.record.take(3).mkString("\n"))
      // A refused mapping (Linux, where `ulimit -v` bounds the address space): the overlays do not fit in 16 GiB at four
      // workers, so the typer types in place by one worker, the fork's path not taken though `TEQ_FORK=1` asks for it;
      // the record says what happened (`mapping_refused=1 effective=1 forked=0 outcome=one`), and the files are as one
      // worker leaves them.
      if Sh("uname", "-s").check(false).run().outAll().trim == "Linux" then
        val dir = work.resolve("typing-mapping")
        Sh.remove(dir)
        val files = program("records", "records")
        for (f, text) <- files do
          Files.createDirectories(dir.resolve(f).getParent)
          Files.writeString(dir.resolve(f), text)
        Files.writeString(dir.resolve("fix.conf"), "rules = [NoUnusedImports]\n")
        val record = dir.resolve("record.txt")
        val inputs = files.map(_._1).sorted.mkString(" ")
        val r = Sh("bash", "-c", s"ulimit -v 16777216 && exec '$teq' compiler fix $inputs --config fix.conf --threads 4").in(dir)
          .withEnv("TEQ_FORK" -> "1", "TEQ_FIX_RULE_THREADS" -> "1", "TEQ_FIX_RECORD" -> record.toString).timeout(300).check(false).run()
        val attempts = if Files.exists(record) then attemptsOf(read(record).linesIterator.toVector) else Vector()
        val one = typed("mapping-one", files, "rules = [NoUnusedImports]\n", 1, 1)
        expect("mapping/record", attempts.nonEmpty && attempts.forall(a => a.mappingRefused && a.asked == 4 && a.effective == 1 && !a.forked && a.outcome == "one"),
          s"exit ${r.status}: ${attempts.mkString("; ")}\n${r.errAll().linesIterator.take(5).mkString("\n")}")
        expect("mapping/files", r.status == one.status && files.forall((f, _) => read(dir.resolve(f)) == one.files(f)), s"exit ${r.status}, the one-worker run's ${one.status}")
        // The address space bounded a GiB at a time (the push pass of 139c515e8, its reproducer as a sweep): from 6 GiB
        // until the fork is joined, at 4, 8 and 16 workers, a run never aborts. Each gives the one-worker run's exit,
        // lints and files (compile's note of a refused mapping aside), and its record the typer's answer: the mappings
        // refused (`mapping_refused=1`, one worker typing in place), a worker given no thread after the fork
        // (`worker_refused=1`, the attempt given way, then one worker's `outcome=one`), or the fork joined; and each
        // count's sweep meets a worker given no thread, the window between the two, at least once.
        for w <- Vector(4, 8, 16) do
          val sweep = work.resolve(s"typing-limit-$w")
          Sh.remove(sweep)
          for (f, text) <- files do
            Files.createDirectories(sweep.resolve(f).getParent)
            Files.writeString(sweep.resolve(f), text)
          Files.writeString(sweep.resolve("fix.conf"), "rules = [NoUnusedImports]\n")
          val base = Sh("bash", "-c", s"exec '$teq' compiler fix $inputs --config fix.conf --check --threads 1").in(sweep).timeout(300).check(false).run()
          val wrong = Vector.newBuilder[String]
          var spent = 0
          var (gib, joined, refusedWorker) = (6, false, false)
          while !joined && gib <= 64 do
            val record = sweep.resolve(s"record-$gib.txt")
            val r = Sh("bash", "-c", s"ulimit -v ${gib.toLong * 1024 * 1024} && exec '$teq' compiler fix $inputs --config fix.conf --check --threads $w").in(sweep)
              .withEnv("TEQ_FIX_RECORD" -> record.toString).timeout(300).check(false).run()
            val err = r.errAll().linesIterator.filterNot(_.startsWith("teq: no memory mapping for the type store's overlays")).mkString("\n")
            val as = if Files.exists(record) then attemptsOf(read(record).linesIterator.toVector) else Vector()
            val kept = files.forall((f, t) => read(sweep.resolve(f)) == t)
            if allocationRefused(r) then spent += 1
            else if r.status != base.status || err != base.errAll().linesIterator.mkString("\n") || !kept then wrong += s"$gib GiB: exit ${r.status} (one worker's ${base.status}), lints alike ${err == base.errAll().linesIterator.mkString("\n")}, files kept $kept: ${r.errAll().linesIterator.take(3).mkString(" | ")}"
            else if as.isEmpty then wrong += s"$gib GiB: no typing line"
            else
              val last = as.last
              val pathOk =
                if as.head.workerRefused then as.size == 2 && as.head.outcome == "retried" && !last.forked && last.outcome == "one"
                else if as.head.mappingRefused then as.size == 1 && !last.forked && last.outcome == "one"
                else as.size == 1 && last.forked && last.outcome == "joined"
              if !pathOk then wrong += s"$gib GiB: ${as.mkString("; ")}"
              refusedWorker |= as.head.workerRefused
              joined = !as.head.mappingRefused && !as.head.workerRefused && last.outcome == "joined"
            gib += 1
          expect(s"limit/w$w", wrong.result().isEmpty && joined, (if joined then "" else s"the fork never joined up to ${gib - 1} GiB\n  ") + wrong.result().mkString("\n  "))
          if spent > 0 then println(s"typing/limit/w$w: $spent run(s) ended in an allocation the system refused (no start's), counted apart")
          expect(s"limit/w$w/worker-refused", refusedWorker, s"no limit from 6 to ${gib - 1} GiB gave a worker no thread after the fork")
        // The starts of the typing's threads, each refused (the push pass of 95a753abf, its probe as cells): the body
        // phase's workers, the inlining phase's and the merge's crew, over a program of inline calls at four workers.
        // The limits where each is met lie in the GiB below the first that joins the fork, a few MiB apart and moving
        // with the schedule: a GiB at a time to that first one, 32 MiB at a time across the GiB below it, then 8 MiB at a
        // time over the 384 MiB from just below the highest limit that refused the body phase's start, again (four
        // times at most) until every start was met. Each run gives the one-worker run's exit, lints and files,
        // its record the path taken (the mappings refused; a start refused, named in the reason, the attempt given
        // way and one worker's `outcome=one` after; the fork joined); a run that ends in an allocation the system
        // refused (Rust's "memory allocation of … failed", the address space spent by an allocation and no thread's
        // start) is counted apart, any other end fails.
        val inlineFiles = program("inline", "inl")
        val cells = work.resolve("typing-starts")
        Sh.remove(cells)
        for (f, text) <- inlineFiles do
          Files.createDirectories(cells.resolve(f).getParent)
          Files.writeString(cells.resolve(f), text)
        Files.writeString(cells.resolve("fix.conf"), "rules = [NoUnusedImports]\n")
        val inlineInputs = inlineFiles.map(_._1).sorted.mkString(" ")
        val inlineBase = Sh("bash", "-c", s"exec '$teq' compiler fix $inlineInputs --config fix.conf --check --threads 1").in(cells).timeout(300).check(false).run()
        val starts = Map("a typing worker" -> "body", "a worker of the inlining phase" -> "inlining", "a merge thread" -> "merge")
        val wrong = Vector.newBuilder[String]
        val met = scala.collection.mutable.Set.empty[String]
        var allocations = 0
        // The highest limit of the 32 MiB pass where the body phase's start was refused: the inlining phase's and the
        // merge's lie above it, by a few MiB and the schedule.
        var bodyAt = 0L
        /** One run under `mib` MiB: whether it joined the fork. */
        def cell(mib: Long): Boolean =
          val record = cells.resolve("record.txt")
          Files.deleteIfExists(record)
          val r = Sh("bash", "-c", s"ulimit -v ${mib * 1024} && exec '$teq' compiler fix $inlineInputs --config fix.conf --check --threads 4").in(cells)
            .withEnv("TEQ_FIX_RECORD" -> record.toString, "TEQ_FIX_RULE_THREADS" -> "1").timeout(300).check(false).run()
          val err = r.errAll().linesIterator.filterNot(_.startsWith("teq: no memory mapping for the type store's overlays")).mkString("\n")
          val as = if Files.exists(record) then attemptsOf(read(record).linesIterator.toVector) else Vector()
          if allocationRefused(r) then
            allocations += 1
            false
          else if r.status != inlineBase.status || err != inlineBase.errAll().linesIterator.mkString("\n") || !inlineFiles.forall((f, t) => read(cells.resolve(f)) == t) then
            wrong += s"$mib MiB: exit ${r.status} (one worker's ${inlineBase.status}): ${r.errAll().linesIterator.take(3).mkString(" | ")}"
            false
          else if as.isEmpty then
            wrong += s"$mib MiB: no typing line"
            false
          else
            val (first, last) = (as.head, as.last)
            if first.workerRefused then
              starts.find((why, _) => first.why.startsWith("the system gave " + why + " no thread")) match
                case Some((_, site)) =>
                  met += site
                  if site == "body" then bodyAt = bodyAt max mib
                case None => wrong += s"$mib MiB: a refusal of no start named: ${first.why}"
              if as.size != 2 || first.outcome != "retried" || last.forked || last.outcome != "one" then wrong += s"$mib MiB: ${as.mkString("; ")}"
              false
            else if first.mappingRefused then
              if as.size != 1 || last.forked || last.outcome != "one" then wrong += s"$mib MiB: ${as.mkString("; ")}"
              false
            else
              if as.size != 1 || !last.forked || last.outcome != "joined" then wrong += s"$mib MiB: ${as.mkString("; ")}"
              true
        var joinedAt = 0L
        var g = 6L
        while joinedAt == 0 && g <= 64 do
          if cell(g * 1024) then joinedAt = g
          g += 1
        var sweeps = 0
        if joinedAt > 0 then
          for mib <- ((joinedAt - 1) * 1024) until (joinedAt * 1024) by 32 do cell(mib)
          sweeps = 1
          val from = if bodyAt > 0 then bodyAt - 64 else (joinedAt - 1) * 1024
          while sweeps < 5 && !Set("body", "inlining", "merge").subsetOf(met) do
            for mib <- from until (from + 384) by 8 do cell(mib)
            sweeps += 1
        expect("starts", joinedAt > 0 && wrong.result().isEmpty, (if joinedAt == 0 then "the fork never joined up to 64 GiB\n  " else "") + wrong.result().mkString("\n  "))
        expect("starts/met", Set("body", "inlining", "merge").subsetOf(met), s"the starts met: ${met.toVector.sorted.mkString(", ")}, over the GiB below $joinedAt GiB ($sweeps sweeps, $allocations allocations refused)")
        if allocations > 0 then println(s"typing/starts: $allocations run(s) ended in an allocation the system refused (no start's), counted apart")

    // The counter macro library (`counter/lib`, its products built here): a run of `next` changes `Counter.n`, which
    // other runs share, so a parallel attempt gives way and the round types again at one worker; `Edit.scala`'s
    // rewrite types the program again in round two, which gives way again. The project rule `Print` prints
    // `configured` once whatever the attempts.
    val products = work.resolve("typing-counter-products")
    Sh.remove(products)
    val built = Sh(teq, "compiler", "check", fixtures.resolve("counter/lib").toString, "--products", products.toString).timeout(300).check(false).run()
    expect("counter/products", built.status == 0, built.errAll())
    val cp = Seq("--classpath", products.toString)
    val print = "rules/Print.scala" -> read(Paths.get("tests/fix/support/Print.scala"))
    val counterConf = "rules = [EmptyCollectionsUnified, NoUnusedImports, \"file:rules/Print.scala\"]\n"
    val editFixed = "package p\nobject Edit { val empty: List[Int] = List.empty }\n"
    val files = program("counter/use", "p") :+ print
    val runs = counts.map(t => typed(s"counter-$t", files, counterConf, t, t, extra = cp))
    val one = runs.head
    expect("counter", one.status == 0 && one.files("src/p/Edit.scala") == editFixed && one.err.linesIterator.count(_ == "configured") == 1,
      s"exit ${one.status}, Edit.scala ${one.files("src/p/Edit.scala")}, configured ${one.err.linesIterator.count(_ == "configured")} times\n${one.err.linesIterator.take(10).mkString("\n")}")
    for (o, t) <- runs.zip(counts).drop(1) do
      expect(s"counter/w$t/bytes", sameBytes(one, o).isEmpty, sameBytes(one, o).getOrElse(""))
      typing(s"counter/w$t/typing", o, t, retried = Set((0, 1), (0, 2)))
    // The retry's refusal: the program with `ZBad.scala` gives way, its one-worker typing refuses it, exit 2, the
    // error printed once, every file as it was.
    val badFiles = program("counter/use", "p") ++ program("counter/bad", "p") :+ print
    val bads = counts.map(t => typed(s"bad-$t", badFiles, counterConf, t, t, extra = cp))
    expect("bad", bads.head.status == 2 && bads.head.files == badFiles.toMap && bads.head.err.linesIterator.count(_.contains("error: type mismatch")) == 1 && bads.head.err.linesIterator.count(_ == "configured") == 1,
      s"exit ${bads.head.status}, files kept ${bads.head.files == badFiles.toMap}\n${bads.head.err}")
    for (o, t) <- bads.zip(counts).drop(1) do
      expect(s"bad/w$t/bytes", sameBytes(bads.head, o).isEmpty, sameBytes(bads.head, o).getOrElse(""))
      typing(s"bad/w$t/typing", o, t, retried = Set((0, 1)))

    // A later job: the verb over two projects, `a` and `b`, b's program the counter's; each run the one-worker run's
    // bytes, b's job given way in both its rounds, a's joined in both.
    val twoLock = Verb.lockWith(Verb.project("a", Nil) + Verb.project("b", Nil, files = List("lib-products")))
    val aFiles = Seq("a/src/a/A.scala" -> "package a\nimport scala.Nil\nobject A { val empty: List[Int] = Nil }\n", "a/src/a/B.scala" -> "package a\nobject B { val b = 1 }\n")
    val bFiles = program("counter/use", "p").map((f, t) => f.replaceFirst("^src/", "b/src/") -> t)
    val twoFiles = Seq("teq.lock" -> twoLock, ".scalafix.conf" -> "rules = [EmptyCollectionsUnified, NoUnusedImports]\n",
      "lib/macros.scala" -> read(fixtures.resolve("counter/lib/macros.scala"))) ++ aFiles ++ bFiles
    def verb(tag: String, args: Seq[String], env: (String, String)*): Parallel.Outcome =
      val d = work.resolve("typing-verb-" + tag)
      Sh.remove(d)
      for (f, text) <- twoFiles do
        Files.createDirectories(d.resolve(f).getParent)
        Files.writeString(d.resolve(f), text)
      // b's class path: the library's products, a directory of the build's tree, built for its JVM projects.
      Sh(teq, "compiler", "check", "lib/macros.scala", "--products", "lib-products", "--target", "jvm", "--std=scala-library").in(d).timeout(300).check(true).run()
      val record = d.resolve("record.txt")
      val r = Sh((Vector(teq, "fix") ++ args)*).in(d).withEnv((Seq("TEQ_FIX_RECORD" -> record.toString) ++ env)*).timeout(300).check(false).run()
      val err = r.errAll().linesIterator.filterNot(_.startsWith("fix: ")).mkString("\n")
      Parallel.Outcome(r.status, r.outAll(), err, twoFiles.map((f, _) => f -> read(d.resolve(f))).toMap, if Files.exists(record) then read(record).linesIterator.toVector else Vector())
    val verbs = counts.map(t => verb(s"w$t", Seq(), "TEQ_THREADS" -> t.toString))
    val vOne = verbs.head
    expect("later-job", vOne.status == 0 && vOne.files("a/src/a/A.scala") == "package a\nobject A { val empty: List[Int] = List.empty }\n" && vOne.files("b/src/p/Edit.scala") == editFixed,
      s"exit ${vOne.status}\n${vOne.err}")
    for (o, t) <- verbs.zip(counts).drop(1) do
      expect(s"later-job/w$t/bytes", sameBytes(vOne, o).isEmpty, sameBytes(vOne, o).getOrElse(""))
      // The job b's: the one whose attempts give way.
      val bJob = attemptsOf(o.record).find(_.outcome == "retried").map(_.job).getOrElse(-1)
      typing(s"later-job/w$t/typing", o, t, retried = Set((bJob, 1), (bJob, 2)))
      if engineParallelTyping then
        val jobs = attemptsOf(o.record).map(_.job).distinct
        expect(s"later-job/w$t/jobs", jobs.size == 2 && bJob == jobs.max, s"jobs ${jobs.mkString(",")}, the one given way $bJob")
    // A small first job whose round the lead ends alone while the others' bootstraps are held: the later job's typing
    // waits for every one of them.
    if engineParallelTyping then
      val held = verb("ready-4", Seq(), "TEQ_THREADS" -> "4", "TEQ_FIX_BOOT_DELAY" -> "1500")
      expect("later-job/ready/bytes", sameBytes(vOne, held).isEmpty, sameBytes(vOne, held).getOrElse(""))
      expect("later-job/ready", barrierProblems(held.record).isEmpty, barrierProblems(held.record).mkString("\n  "))
    // `teq fix --threads n`: compile's option, the one request of both counts.
    if engineParallelTyping then
      val flag = verb("flag-8", Seq("--threads", "8"))
      expect("flag/bytes", sameBytes(vOne, flag).isEmpty, sameBytes(vOne, flag).getOrElse(""))
      val bJob = attemptsOf(flag.record).find(_.outcome == "retried").map(_.job).getOrElse(-1)
      expect("flag/typing", typingProblems(flag.record, 8, Set((bJob, 1), (bJob, 2))).isEmpty, typingProblems(flag.record, 8, Set((bJob, 1), (bJob, 2))).mkString("\n  "))
      expect("flag/rules", flag.record.exists(l => l.startsWith("run ") && Parallel.fieldsOf(l).get("workers") == Some("8")), flag.record.take(3).mkString("\n"))
      val flagEq = verb("flag-eq-8", Seq("--threads=8"))
      expect("flag/eq/bytes", sameBytes(vOne, flagEq).isEmpty, sameBytes(vOne, flagEq).getOrElse(""))
      expect("flag/eq/typing", attemptsOf(flagEq.record).nonEmpty && attemptsOf(flagEq.record).forall(a => a.attempt > 1 || a.asked == 8), attemptsOf(flagEq.record).mkString("\n  "))
      for bad <- Seq("0", "x") do
        val o = verb(s"flag-$bad", Seq("--threads", bad))
        expect(s"flag/$bad", o.status == 2 && o.files == twoFiles.toMap && o.err.contains("--threads"), s"exit ${o.status}\n${o.err}")
      // `teq compiler fix`, whose one job is built from what the flags leave: both spellings reach it.
      for (spelling, args) <- Seq("space" -> Seq("--threads", "8"), "eq" -> Seq("--threads=8")) do
        val o = typed(s"compiler-flag-$spelling", files, counterConf, 1, 1, extra = cp ++ args)
        expect(s"compiler-flag/$spelling/bytes", sameBytes(one, o).isEmpty, sameBytes(one, o).getOrElse(""))
        expect(s"compiler-flag/$spelling/typing", typingProblems(o.record, 8, Set((0, 1), (0, 2))).isEmpty, typingProblems(o.record, 8, Set((0, 1), (0, 2))).mkString("\n  "))
      for bad <- Seq("0", "x") do
        val o = typed(s"compiler-flag-$bad", files, counterConf, 1, 1, extra = cp ++ Seq("--threads", bad))
        expect(s"compiler-flag/$bad", o.status == 2 && o.files == files.toMap && o.err.contains("--threads"), s"exit ${o.status}\n${o.err}")

    // The merge's semantic answers, each the one-worker typing's at every count.
    val unusedConf = "rules = [NoUnusedImports]\n"
    val bothConf = "rules = [EmptyCollectionsUnified, NoUnusedImports]\n"
    val mebDir = Paths.get("tests/fix/NoUnusedImports/macro_expansion_block")
    val meb = Files.list(mebDir).iterator.asScala.toVector.sorted.filter(f => f.toString.endsWith(".scala") && f.getFileName.toString != "output.scala").map(f => s"src/meb/${f.getFileName}" -> read(f))
    val mebConf = header(read(mebDir.resolve("input.scala")), mebDir.toAbsolutePath)
    val semantic = Seq(
      ("marks", program("marks", "marks"), unusedConf, 0),
      ("withheld", program("withheld", "withheld"), unusedConf, 2),
      ("records", program("records", "records"), unusedConf, 0),
      ("blocks", program("blocks", "blocks"), bothConf, 0),
      ("phase-errors", program("phase-errors", "pe"), bothConf, 2),
      ("round2", program("round2", "p"), bothConf, 0),
      ("macro_expansion_block", meb, mebConf, -1))
    for (name, files, conf, status) <- semantic do
      val runs = counts.map(t => typed(s"$name-$t", files, conf, t, t, Seq("TEQ_FIX_TYPING_WORK" -> "1")))
      val one = runs.head
      expect(name, (status == -1 || one.status == status) && (one.status != 2 || one.files == files.toMap), s"exit ${one.status}, expected $status\n${one.err.linesIterator.take(10).mkString("\n")}")
      for (o, t) <- runs.zip(counts).drop(1) do
        expect(s"$name/w$t/bytes", sameBytes(one, o).isEmpty, sameBytes(one, o).getOrElse(""))
        typing(s"$name/w$t/typing", o, t)
    // What each answer is: no unused import where scalac 3.8.4 finds none (a wildcard and a hiding selector, a given and
    // a name, marked by readers in other files); the analysis withheld after an error, the run refused; the imports of a
    // file with an expansion kept as unverified; the CRLF file's braced packages one document; `import scala.Nil` removed
    // in round two.
    if engineParallelTyping then
      // The readers' workers: `A.scala`'s and `B.scala`'s items typed by two workers, each a reader of `Shared.scala`'s
      // imports through another file's member (the queue hands each worker a first file of its own).
      for t <- counts.drop(1) do
        val o = typed(s"marks-workers-$t", program("marks", "marks"), unusedConf, t, t, Seq("TEQ_FIX_TYPING_WORK" -> "1"))
        val by = itemsOf(o.record).filter(i => i.file.endsWith("marks/A.scala") || i.file.endsWith("marks/B.scala")).map(i => i.file.takeRight(7) -> i.worker).distinct
        expect(s"marks/w$t/readers", by.size == 2 && by.map(_._2).distinct.size == 2, s"the readers' items: ${by.mkString(", ")}")
    val marks = typed("marks-check", program("marks", "marks"), unusedConf, 1, 1)
    expect("marks/answer", marks.status == 0 && !marks.err.contains("NoUnusedImports") && marks.files == program("marks", "marks").toMap, marks.err)
    val records = typed("records-check", program("records", "records"), unusedConf, 1, 1)
    expect("records/answer", records.err.linesIterator.count(_.contains("[NoUnusedImports.unverified]")) == 2 && records.files == program("records", "records").toMap, records.err)
    val round2 = typed("round2-check", program("round2", "p"), bothConf, 1, 1)
    expect("round2/answer", round2.files("src/p/Input.scala") == "package p\nobject Example { val empty: List[Int] = List.empty }\n", round2.files("src/p/Input.scala"))
    // The default getter's case (the push pass of 5c171b26f), its fixture tests/workers.sh's: two files, a method with a
    // default argument, typed by several workers: the bytes the one-worker run's; in the assertion-enabled build
    // pending on the pin until the parallel typer's publication of the getter is fixed.
    val getter = Files.list(Paths.get("tests/workers/default_getter_sealed")).iterator.asScala.toVector.sorted.map(f => s"src/dg/${f.getFileName}" -> read(f))
    val getterOne = typed("getter-1", getter, unusedConf, 1, 1)
    expect("getter", getterOne.status == 0, s"exit ${getterOne.status}\n${getterOne.err.linesIterator.take(5).mkString("\n")}")
    for t <- counts.drop(1) do
      val o = typed(s"getter-$t", getter, unusedConf, t, t)
      if !pinned(s"getter/w$t", o) then
        expect(s"getter/w$t/bytes", sameBytes(getterOne, o).isEmpty, sameBytes(getterOne, o).getOrElse(""))
        typing(s"getter/w$t/typing", o, t)
    if pending > 0 then println(s"typing: $pending pending on tests/workers.sh's default_getter_sealed")
    (pass, fail)
