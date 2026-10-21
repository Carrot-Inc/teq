//> using file ../tools/script
// `teq fix` (docs/FIX.md, docs/CLI.md): every case of tests/fix/<Rule>/<Case> (tests/fix/REFERENCES.md) run by
// `teq compiler fix` over a copy of its input.scala, its header comment the configuration as scalafix's testkit reads
// it (a `file:` entry made absolute from the case's directory), its `jars` the class path (tests/support/jars.sh):
// the file it leaves is the case's output.scala (its input where it has none), the lints it prints are its
// expected.txt, and those lints are the fixture's own assertions (`// assert: <id>` on a line, `/* assert: <id>` with
// a caret line and a message), no more and no fewer; the exit is 1 where an error remains, else 0; a second run over
// what the first left changes nothing and prints the same; `--check` over the input changes nothing and exits 1
// where there is an edit or a lint. A script's case is `input.sc` and `output.sc`. A case with a `refused.txt` is one the run refuses: exit 2, that text among what it
// prints, every file as it was. Then the verb's own cases (`verb`, below). `--record` writes each expected.txt from
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
      problems ++= checkAsserts(input, parsed)
      val expectedStatus = if parsed.exists(_.severity == "error") then 1 else 0
      if r1.status != expectedStatus then problems += s"exit ${r1.status}, expected $expectedStatus"
      val expectedFile = c.resolve("expected.txt")
      if record && problems.isEmpty then
        if lints.nonEmpty then Files.writeString(expectedFile, lints) else Files.deleteIfExists(expectedFile)
      else
        val want = if Files.exists(expectedFile) then read(expectedFile) else ""
        if lints != want then problems += s"the lints differ from expected.txt:\n${diff(want, lints)}"
      // A second run over what the first left changes nothing and prints what the first printed of it.
      val dir = work.resolve(name)
      val r2 = command(teq, name, inputName, extras, confFile, classpath, false, work).run()
      if read(dir.resolve(inputName)) != out1 then problems += "a second run changed the file again"
      if out1 == input && r2.errAll().replace(work.toString + "/", "") != lints then problems += "a second run printed other lints"
      // `--check` changes nothing.
      val (rc, outc) = runOn(true)
      val checkStatus = if out1 != input || lints.linesIterator.exists(_.matches(".*: (error|warning|info): \\[.*")) then 1 else 0
      if outc != input then problems += "--check changed the file"
      if rc.status != checkStatus then problems += s"--check exited ${rc.status}, expected $checkStatus"
    if problems.isEmpty then pass += 1 else bad(name, problems.mkString("\n  "))

  if only.forall(o => "verb".startsWith(o) || o.startsWith("verb")) then
    val (vp, _) = Verb.run(teq, work, bad)
    pass += vp
  println(s"fix: $pass passed, $fail failed")
  Script.exit(if fail == 0 then 0 else 1)
}

def read(p: Path): String = new String(Files.readAllBytes(p), java.nio.charset.StandardCharsets.UTF_8)

def command(teq: String, name: String, input: String, extras: Vector[Path], conf: Path, classpath: Option[String], check: Boolean, dir: Path): Sh.Cmd =
  val files = (name + "/" + input) +: extras.map(e => name + "/" + e.getFileName)
  val base = Vector(teq, "compiler", "fix") ++ files ++ Vector("--config", conf.toString) ++ classpath.toVector.flatMap(cp => Vector("--classpath", cp)) ++ (if check then Vector("--check") else Vector())
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
       |""".stripMargin + project("app", List("core")) + project("core", Nil, coreExcludes, coreGenerators) +
      """|repositories:
         |  - id: maven-central
         |    url: https://repo1.maven.org/maven2/
         |""".stripMargin

  def project(name: String, deps: List[String], excludes: List[String] = Nil, generators: String = ""): String =
    val cp = deps.map(d => s"          - {configuration: compile, project: $d}\n").mkString
    val sources = (deps.map(d => s"        - $d/src\n") :+ s"        - $name/src\n").mkString
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
        |        resources: []
        |        sources:
        |          - $name/src
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
    val h = build(work, "hocon", ".scalafix.conf" -> "include \"other.conf\"\nrules = [DisableSyntax]\n")
    val rh = fix(h)
    expect("unsupported hocon", rh.status == 2 && rh.errAll().contains(".scalafix.conf:1:1: `include` is not supported"), s"exit ${rh.status}: ${rh.errAll()}")
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
    (pass, fail)
