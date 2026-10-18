//> using file ../tools/script
// The given searches of tapir 1.13.29's tuple joins, whose targets hold open type variables: the endpoints of
// tests/cases/given_open_variable_tapir_joins.scala at several input widths, each program typed at one worker under
// `--profile` by every binary named, and the rows of their searches. Run from the repository's root:
//
//   ./teq interp bench/join-searches.scala -- [--widths 4,6,8] [--endpoints 1] [--timeout 120] [--out <dir>] [--scalac] <teq>...
//
// A program of width w has a value-class path segment and w-1 query inputs of distinct types, then
// `.out(stringBody)`, in `--endpoints` endpoints (default 1) that take the case's forms in turn: an inferred val, a
// generic function's argument, a chain ending in `.mapIn`, a lambda's result, a chain ending in `.serverLogic`. Each
// binary types each program with `--std scala-library --target jvm --all-mains --threads 1 --profile`, `--timeout`
// seconds at most (default 120), and one row is printed per binary and width: the type phase, the given searches'
// count and own time, those whose target holds open type variables (count, own time, inclusive), the divergence
// checks' measure of the target (a dash where the binary has no such row), and the own time of `Join.Fold.step` and
// `TupleFoldInstances.t1` with their instantiations and failures. `--scalac` adds scalac 3.8.4's typer time on the
// same programs (scala-cli with `-Yprofile-enabled -Ystop-after:typer`: a cold JVM's, not comparable with teq's in
// absolute terms). The programs, every run's log and the rows (rows.txt) go under --out (default
// out/join-searches). Status 2 when the tapir jars are not in the coursier cache (tests/support/jars.sh), 1 when a
// run fails other than by its time limit.
import java.nio.file.{Files, Path, Paths}

@main def joinSearches(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/join-searches.scala [--widths 4,6,8] [--endpoints <n>] [--timeout <seconds>] [--out <dir>] [--scalac] <teq>...")
  val widths = args.option("widths").getOrElse("4,6,8").split(',').toVector.map(w => w.toIntOption.getOrElse(args.fail(s"a width is a number, not $w")))
  val endpoints = args.option("endpoints").map(n => n.toIntOption.filter(_ > 0).getOrElse(args.fail(s"--endpoints takes a positive count, not $n"))).getOrElse(1)
  val limit = args.option("timeout").map(n => n.toIntOption.filter(_ > 0).getOrElse(args.fail(s"--timeout takes seconds, not $n"))).getOrElse(120)
  val out = Paths.get(args.option("out").getOrElse("out/join-searches"))
  val scalac = args.flag("scalac")
  val binaries = args.positionals
  if binaries.isEmpty then args.fail("name a binary")
  widths.find(w => w < 2 || w > Joins.types.length + 1).foreach(w => args.fail(s"a width from 2 to ${Joins.types.length + 1}, not $w"))
  binaries.find(b => !Files.isExecutable(Paths.get(b))).foreach(b => args.fail(s"no binary $b"))
  Files.createDirectories(out)
  val jarLine = out.resolve("jars.scala")
  Files.writeString(jarLine, "// jars: scala-library tapir-core-jvm magnolia123-jvm sttp-model-core-jvm sttp-shared-core-jvm sttp-shared-ws-jvm\n")
  val jars = Sh("bash", "-c", ". tests/support/jars.sh; jars_of \"$1\"; printf '%s\\n%s\\n' \"$JARS_MISSING\" \"$JARS_CP\"", "jars", jarLine.toString).run().lines
  if jars.headOption.exists(_.trim.nonEmpty) then
    System.err.println(s"join-searches: not in the coursier cache:${jars.head}")
    Script.exit(2)
  val classpath = jars.lift(1).getOrElse("")
  for w <- widths do Files.writeString(out.resolve(s"join$w.scala"), Joins.program(w, endpoints))
  var failed = false
  val rows = Vector.newBuilder[String]
  def emit(row: String): Unit =
    println(row)
    rows += row
  for teq <- binaries do
    val name = Sh(teq, "--version").run().out.trim.split(' ').last
    for w <- widths do
      val log = out.resolve(s"$name-join$w.log")
      val r = Sh(teq, "compiler", "check", out.resolve(s"join$w.scala").toString, "--classpath", classpath, "--std", "scala-library",
        "--target", "jvm", "--all-mains", "--threads", "1", "--profile").mergeErr.stdout(log).timeout(limit).check(false).run()
      if r.timedOut then emit(f"$name%-24s $w%5d  timeout after $limit s")
      else
        if !r.ok then
          System.err.println(s"join-searches: $name failed on width $w (status ${r.status}, $log)")
          failed = true
        emit(f"$name%-24s $w%5d  ${Joins.row(Files.readString(log))}")
  if scalac then
    for w <- widths do
      val log = out.resolve(s"scalac-join$w.log")
      val r = Sh("scala-cli", "compile", out.resolve(s"join$w.scala").toString, "-S", "3.8.4", "--server=false", "--classpath", classpath,
        "--scalac-option", "-Yprofile-enabled", "--scalac-option", "-Ystop-after:typer").mergeErr.stdout(log).timeout(limit).check(false).run()
      val typer = Files.readString(log).linesIterator.collectFirst { case l if l.startsWith("typer,run ns = ") => l.stripPrefix("typer,run ns = ").takeWhile(_.isDigit) }
      if r.timedOut then emit(f"${"scalac-3.8.4"}%-24s $w%5d  timeout after $limit s")
      else typer match
        case Some(ns) if r.ok => emit(f"${"scalac-3.8.4"}%-24s $w%5d  typer ${ns.toLong / 1000000} ms")
        case _ =>
          System.err.println(s"join-searches: scalac gave no typer time on width $w ($log)")
          failed = true
  Files.writeString(out.resolve("rows.txt"), rows.result().mkString("", "\n", "\n"))
  Script.exit(if failed then 1 else 0)
}

object Joins:
  val types = Vector("Int", "Option[Int]", "Option[String]", "Option[Boolean]", "Long", "Double", "String", "Option[Long]",
    "Boolean", "Option[Double]", "Float", "Option[Float]", "Short", "Option[Short]", "Byte", "Option[Byte]")

  def program(width: Int, endpoints: Int): String =
    val chain = "endpoint.in(path[String](\"id\").map(Id.apply)(_.value))" +
      types.take(width - 1).zipWithIndex.map((t, i) => s".in(query[$t](\"q$i\"))").mkString + ".out(stringBody)"
    val forms = Vector(chain, s"keep($chain)", s"$chain.mapIn(x => x)(x => x)", s"eval(_ => $chain)", s"$chain.serverLogic[Identity](_ => Right(\"\"))")
    val vals = (0 until endpoints).map(k => s"  val e$k = ${forms(k % forms.length)}\n").mkString
    "import sttp.tapir.*\nimport sttp.shared.Identity\nfinal case class Id(value: String) extends AnyVal\nobject Joins:\n" +
      "  def keep[A](a: A): A = a\n  def eval[A](f: Int => A): A = f(0)\n" + vals

  // The figures of one profiled run, a dash for each the log has not.
  def row(log: String): String =
    val lines = log.linesIterator.toVector
    def after(prefix: String): Option[String] = lines.collectFirst { case l if l.startsWith(prefix) => l.stripPrefix(prefix) }
    def words(s: String): Vector[String] = s.trim.split("\\s+").toVector
    val phase = after("profile: type phase ").map(s => s.takeWhile(_ != ','))
    val searches = after("  given search ").map(words).filter(_.length >= 3).map(w => s"${w(0)} searches ${w(1)} ${w(2)}")
    val open = after("    not applicable: open type variables in the target: ").map(_.replace(" (", ", ").replace(")", "").replace(" self,", " own,"))
    val measure = after("  divergence checks' measure of the target ").map(words).filter(_.length >= 3).map(w => s"${w(1)} ${w(2)}")
    def candidate(name: String): Option[String] =
      lines.collectFirst { case l if l.startsWith(s"    $name (") && l.contains(" shape checks, ") => l.stripPrefix(s"    $name (") }.flatMap { s =>
        val inner = s.stripSuffix(")")
        inner.split("; ").toVector match
          case Vector(counts, own) =>
            val c = counts.split(", ").toVector
            Some(s"$own (${c.lift(1).getOrElse("-")}, ${c.lift(2).getOrElse("-")})")
          case _ => None
      }
    s"type ${phase.getOrElse("-")}; given ${searches.getOrElse("-")}; open targets ${open.getOrElse("-")}; " +
      s"measure ${measure.getOrElse("-")}; step ${candidate("sttp.tapir.typelevel.TupleOps.Join.Fold.step").getOrElse("-")}; " +
      s"t1 ${candidate("sttp.tapir.typelevel.TupleFoldInstances.t1").getOrElse("-")}"
