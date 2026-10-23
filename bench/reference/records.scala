// The records of the reference projects' gate lines (docs/DEVELOPING.md, "The project records"): what a line's run
// may show of teq's departures from the oracle, as a floor a landing may only lower, never a licence. A record is
// its provenance (how it was made: `baseline`, the first record, from master's binary; `record`, a run's
// improvement; `migrate <reason>`, a new pin or new generated inputs; with the revision, the binary, the pin and the
// digest of the inputs it holds for), its scope (the units the run covers, each exactly once: a compile's
// configurations, a test run's projects, a run's assertions) and its debt (the departures accepted, a sorted
// multiset). Two checks hold a landing to it, both of which must pass:
//
// - admission (`Ratchet.admit`), independent of any run: the head's record against the accepted one, master's
//   committed file. For the same inputs the scope is the accepted one and the debt a sub-multiset of the accepted
//   debt; a first record is a baseline; new inputs take a migration with its reason; a record removed fails. The gate
//   runs it over every record (bench/reference/ratchet.scala's `check`), so that a record widened beside the code
//   that needs it fails whatever the run shows.
// - the verdict (`Ratchet.judge`): a run's observation against the head's record. Every unit of the scope covered
//   once, nothing outside it, no problem of completeness (a run cut short, killed or timed out, output without its
//   end), and the observed debt a sub-multiset of the record's: a departure outside it fails the line, printed in
//   full (the first ten), and one of the record's the run no longer shows is listed as gone, the line passing.
//
// The vulpix runner of dotty's tests (compiler/test/dotty/tools/vulpix/ParallelTesting.scala, 3.8.4) is the model
// of the identity: a negative test's errors are compared by their count and their positions, a missing output or a
// crash is a failure of its own. A diagnostic here is its whole block as teq prints it (the location, the
// severity, every line of the message, the source line and the caret), the checkout's root normalised away; a
// diagnostic that moves is a difference.
//
// An improvement is persisted by the landing that makes it: `--record` stages the run's debt as a proposal beside
// the record (`<record>.proposed`), validates it against the record it replaces by the admission's rule and only then
// moves it into place; a failed run proposes nothing.
import java.nio.file.{Files, Path, Paths, StandardCopyOption}
import java.nio.charset.StandardCharsets.UTF_8
import java.security.MessageDigest

// How a record was made: `baseline`, `record` or `migrate` (with its reason), the revision of teq and the binary's
// version line, the pin's commit and the digest of the inputs the record holds for (`Records.inputs`).
final case class Provenance(kind: String, reason: String, revision: String, binary: String, pin: String, inputs: String)

final case class Record(provenance: Provenance, scope: Vector[String], debt: Vector[String]):
  def sorted: Record = copy(scope = scope.sorted, debt = debt.sorted)

// What a run showed: the units it covered (as many times as it covered each), its departures, and what makes it no
// evidence at all (a run cut short, killed, timed out, its output incomplete).
final case class Observation(scope: Vector[String], debt: Vector[String], problems: Vector[String])

// The run against its record: the debt gone and the debt new, the scope's units not covered, covered twice or not
// the record's, and the observation's problems.
final case class Verdict(gone: Vector[String], added: Vector[String], scope: Vector[String], problems: Vector[String]):
  def passed: Boolean = added.isEmpty && scope.isEmpty && problems.isEmpty

// A kind of record: its file under bench/<project>/ and the words of its scope and its debt (a line's first word in
// a text record, a member's name in a JSON one).
final case class Shape(file: String, scope: String, debt: String, noun: String):
  def what: String = noun + "s"
  def counted(n: Int): String = s"$n $noun${if n == 1 then "" else "s"}"
  def json: Boolean = file.endsWith(".json")
  // A baseline's or a migration's evidence beside the record: the run's normalised transcript, gzipped (no name, no
  // time in its header: the same transcript gives the same bytes).
  def evidence: String = file.substring(0, file.lastIndexOf('.')) + ".evidence.gz"

object Shape:
  // A compile line's: its configurations and its errors.
  val Compile = Shape("errors.txt", "key", "error", "error")
  // A test line's deviations from the oracle's inventory (tests.json, which stays the oracle's): its projects and
  // its departures, blocked suites among them.
  val Tests = Shape("deviations.json", "projects", "deviations", "deviation")
  // A run line's: its named assertions and the known violations among them.
  val Run = Shape("run.json", "assertions", "violations", "violation")
  val all = Vector(Compile, Tests, Run)

object Multiset:
  def counts(items: Vector[String]): Map[String, Int] = items.groupMapReduce(identity)(_ => 1)(_ + _)
  // What `a` holds beyond `b`, each item as many times as `a` holds it more often, sorted.
  def minus(a: Vector[String], b: Vector[String]): Vector[String] =
    val cb = counts(b)
    counts(a).toVector.sortBy(_._1).flatMap((k, n) => Vector.fill((n - cb.getOrElse(k, 0)).max(0))(k))
  def within(a: Vector[String], b: Vector[String]): Boolean = minus(a, b).isEmpty

object Records:
  def sha256(bytes: Array[Byte]): String = MessageDigest.getInstance("SHA-256").digest(bytes).map(b => f"${b & 0xff}%02x").mkString

  // The digest of a record's inputs: each file of the project's directory it holds for, by its name and its bytes'
  // digest (the bytes staged for it where a recording stages them), an absent file named as absent.
  def inputs(here: Path, files: Seq[String], staged: Map[String, Array[Byte]] = Map.empty): String =
    sha256(files.map { f =>
      val p = here.resolve(f)
      staged.get(f).orElse(Option.when(Files.isRegularFile(p))(Files.readAllBytes(p))).fold(s"$f absent\n")(b => s"$f ${sha256(b)}\n")
    }.mkString.getBytes(UTF_8))

  // The inputs each kind of record holds for: the pin (with the overlay's digests) and the generated inputs' digests,
  // and for the deviations the oracle's inventory too.
  def inputsOf(here: Path, shape: Shape, staged: Map[String, Array[Byte]] = Map.empty): String =
    inputs(here, if shape == Shape.Tests then Seq("pin.json", "generated.json", "tests.json") else Seq("pin.json", "generated.json"), staged)

  def gzip(text: String): Array[Byte] =
    val bytes = new java.io.ByteArrayOutputStream()
    val gz = new java.util.zip.GZIPOutputStream(bytes)
    gz.write(text.getBytes(UTF_8))
    gz.close()
    bytes.toByteArray

  // An item on one line: a backslash, a newline, a carriage return and a tab escaped.
  def escape(s: String): String = s.flatMap {
    case '\\' => "\\\\"
    case '\n' => "\\n"
    case '\r' => "\\r"
    case '\t' => "\\t"
    case c => c.toString
  }
  def unescape(s: String): String =
    val b = new StringBuilder
    var i = 0
    while i < s.length do
      if s(i) == '\\' && i + 1 < s.length then
        b += (s(i + 1) match
          case 'n' => '\n'
          case 'r' => '\r'
          case 't' => '\t'
          case c => c)
        i += 2
      else
        b += s(i)
        i += 1
    b.toString

  private val provenanceWords = Vector("kind", "reason", "revision", "binary", "pin", "inputs")

  def render(shape: Shape, r: Record): String =
    val p = r.provenance
    val fields = Vector(p.kind, p.reason, p.revision, p.binary, p.pin, p.inputs)
    if shape.json then
      Json.write(Json.Obj(provenanceWords.zip(fields).map((k, v) => k -> Json.str(v)) ++ Vector(
        shape.scope -> Json.Arr(r.scope.sorted.map(Json.str)), shape.debt -> Json.Arr(r.debt.sorted.map(Json.str)))), indent = 1) + "\n"
    else
      val head = s"# The accepted ${shape.what} of a reference project's line (bench/reference/records.scala; docs/DEVELOPING.md, \"The project records\").\n"
      head + provenanceWords.zip(fields).filter((k, v) => k != "reason" || v.nonEmpty).map((k, v) => s"$k ${escape(v)}\n").mkString +
        r.scope.sorted.map(s => s"${shape.scope} ${escape(s)}\n").mkString + r.debt.sorted.map(d => s"${shape.debt} ${escape(d)}\n").mkString

  // A record's text, or the reason it does not read.
  def parse(shape: Shape, text: String): Either[String, Record] =
    try
      if shape.json then
        val j = Json.parse(text)
        def field(k: String) = j.get(k).map(_.str).getOrElse("")
        val p = Provenance(field("kind"), field("reason"), field("revision"), field("binary"), field("pin"), field("inputs"))
        Right(Record(p, j(shape.scope).elements.map(_.str), j(shape.debt).elements.map(_.str)).sorted).flatMap(checked)
      else
        val fields = scala.collection.mutable.Map.empty[String, String]
        val (scope, debt) = (Vector.newBuilder[String], Vector.newBuilder[String])
        var bad = Option.empty[String]
        for (line, n) <- text.linesIterator.zipWithIndex if line.nonEmpty && !line.startsWith("#") && bad.isEmpty do
          val (word, rest) = line.indexOf(' ') match
            case -1 => (line, "")
            case i => (line.substring(0, i), unescape(line.substring(i + 1)))
          if word == shape.scope then scope += rest
          else if word == shape.debt then debt += rest
          else if provenanceWords.contains(word) && !fields.contains(word) then fields(word) = rest
          else bad = Some(s"line ${n + 1}: no line starts `$word` here")
        bad.toLeft(()).flatMap { _ =>
          def field(k: String) = fields.getOrElse(k, "")
          checked(Record(Provenance(field("kind"), field("reason"), field("revision"), field("binary"), field("pin"), field("inputs")), scope.result(), debt.result()).sorted)
        }
    catch case e: Exception => Left(e.getMessage)

  private def checked(r: Record): Either[String, Record] =
    val p = r.provenance
    if !Set("baseline", "record", "migrate").contains(p.kind) then Left(s"its kind is `${p.kind}`, not baseline, record or migrate")
    else if p.kind == "migrate" && p.reason.trim.isEmpty then Left("a migration without its reason")
    else if p.inputs.isEmpty then Left("no digest of its inputs")
    else if r.scope.distinct.size != r.scope.size then Left(s"a unit of its scope twice: ${r.scope.diff(r.scope.distinct).head}")
    else Right(r)

  // The record of a kind in the project's directory: None when there is no file; a file that does not read stops the
  // script.
  def read(here: Path, shape: Shape): Option[Record] =
    val f = here.resolve(shape.file)
    if !Files.isRegularFile(f) then None
    else parse(shape, Files.readString(f)).fold(why => Log.die(s"$f does not read: $why"), Some(_))

  // A recording published as one: the record, a baseline's or a migration's evidence (the run's transcript,
  // normalised, `Shape.evidence`) and the files staged with it (the generated inputs' digests a baseline or a
  // migration writes), each written beside its file as `<file>.proposed`, the record read back and admitted against
  // the record it replaces by the ratchet's rule (`Ratchet.admit`, for the inputs the staged files give) and only then
  // each moved into its place, the record last; the problems when it was refused, every proposal removed and nothing
  // changed.
  def publish(here: Path, shape: Shape, proposed: Record, evidence: Option[String], staged: Vector[(String, Array[Byte])] = Vector.empty): Vector[String] =
    val files = staged ++ evidence.map(e => shape.evidence -> gzip(e)).toVector :+ (shape.file -> render(shape, proposed).getBytes(UTF_8))
    val proposals = files.map((f, _) => here.resolve(f + ".proposed"))
    def drop(): Unit = proposals.foreach(Files.deleteIfExists)
    try
      for ((_, bytes), at) <- files.zip(proposals) do Files.write(at, bytes)
      val back = parse(shape, Files.readString(proposals.last))
      val problems = back match
        case Left(why) => Vector(s"the proposal does not read back: $why")
        case Right(r) if r != proposed.sorted => Vector("the proposal reads back as another record")
        case Right(r) => Ratchet.admit(read(here, shape), Some(r), inputsOf(here, shape, staged.toMap), evidence.nonEmpty || Files.isRegularFile(here.resolve(shape.evidence)))
      if problems.nonEmpty then
        drop()
        problems
      else
        for ((f, _), at) <- files.zip(proposals) do Files.move(at, here.resolve(f), StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE)
        Vector.empty
    catch case e: java.io.IOException =>
      drop()
      Vector(s"the proposal was not written: ${e.getMessage}")

object Ratchet:
  // The head's record (`candidate`) against the accepted one (master's committed file, or the record a proposal would
  // replace), for the head's inputs, the evidence of a baseline or a migration beside it or not: the problems, none
  // when it is admitted.
  def admit(accepted: Option[Record], candidate: Option[Record], inputs: String, evidence: Boolean): Vector[String] =
    (accepted, candidate) match
      case (None, None) => Vector.empty
      case (Some(a), None) => Vector(s"the record is removed: the accepted one holds ${a.debt.size} ${if a.debt.size == 1 then "item" else "items"}")
      case (_, Some(c)) if c.provenance.inputs != inputs =>
        Vector(s"the record holds for the inputs ${c.provenance.inputs.take(12)}, the head's are ${inputs.take(12)}: a new pin or new generated inputs take a migration (--migrate <reason>)")
      case (None, Some(c)) =>
        if c.provenance.kind != "baseline" then Vector(s"a first record is a baseline (--baseline), not a ${c.provenance.kind}")
        else if !evidence then Vector("a baseline without its evidence beside it")
        else Vector.empty
      case (Some(a), Some(c)) if a == c => Vector.empty
      case (Some(a), Some(c)) if a.provenance.inputs == c.provenance.inputs =>
        val out = Vector.newBuilder[String]
        val wider = Multiset.minus(c.debt, a.debt)
        if c.provenance.kind == "migrate" && wider.nonEmpty then
          // The compiler now places or prints an accepted error elsewhere: every item the head adds pairs with one it
          // loses by the message (the position and the printed lines apart), and the debt does not grow.
          if !evidence then out += "a migration without its evidence beside it"
          val unmatched = Multiset.minus(wider.map(message), Multiset.minus(a.debt, c.debt).map(message))
          if unmatched.nonEmpty then
            out += s"a migration over the accepted record's inputs adds ${unmatched.size} ${if unmatched.size == 1 then "item" else "items"} no lost item matches by its message:"
            unmatched.take(10).foreach(w => out += s"  + ${Records.escape(w)}")
          if c.debt.size > a.debt.size then out += s"a migration over the accepted record's inputs grows the debt from ${a.debt.size} to ${c.debt.size}"
        else if c.provenance.kind != "record" then out += s"a ${c.provenance.kind} over the accepted record's inputs: a step of the record is a record (--record), its debt within the accepted debt"
        if c.scope != a.scope then
          val (lost, added) = (a.scope.diff(c.scope), c.scope.diff(a.scope))
          out += s"the scope is not the accepted one:${if lost.nonEmpty then s" ${lost.size} left out (${lost.take(3).mkString(", ")})" else ""}${if added.nonEmpty then s" ${added.size} added (${added.take(3).mkString(", ")})" else ""}"
        if (c.provenance.kind != "migrate" || wider.isEmpty) && wider.nonEmpty then
          out += s"the record widens the accepted one by ${wider.size}:"
          wider.take(10).foreach(w => out += s"  + ${Records.escape(w)}")
        out.result()
      case (Some(_), Some(c)) =>
        if c.provenance.kind != "migrate" then Vector(s"the inputs are not the accepted record's: a ${c.provenance.kind} cannot change them, a migration (--migrate <reason>) does")
        else if !evidence then Vector("a migration without its evidence beside it")
        else Vector.empty

  // An item's message: its first line without the source position, the key and the printed lines apart.
  def message(item: String): String =
    item.takeWhile(_ != '\n').replaceAll("""\S+:\d+:\d+:\s*""", "").trim

  // A run's observation against the head's record.
  def judge(record: Record, seen: Observation): Verdict =
    val scope = Vector.newBuilder[String]
    val covered = Multiset.counts(seen.scope)
    for u <- record.scope do
      covered.getOrElse(u, 0) match
        case 0 => scope += s"$u: not covered"
        case 1 => ()
        case n => scope += s"$u: covered $n times"
    for u <- seen.scope.distinct.sorted if !record.scope.contains(u) do scope += s"$u: not the record's"
    Verdict(Multiset.minus(record.debt, seen.debt), Multiset.minus(seen.debt, record.debt), scope.result(), seen.problems)

  // The verdict's lines, `<project>: <what>: ...` and `FAIL <project>: <what>: ...`, the new debt in full (the first ten),
  // the gone listed (the first fifty); whether it passed.
  def report(project: String, what: String, record: Record, v: Verdict, shape: Shape): Boolean =
    def counted(n: Int) = shape.counted(n)
    for p <- v.problems do println(s"FAIL $project: $what: $p")
    for s <- v.scope do println(s"FAIL $project: $what: $s")
    if v.added.nonEmpty then
      println(s"FAIL $project: $what: ${counted(v.added.size)} new")
      v.added.take(10).foreach(a => println(a.linesIterator.map("  " + _).mkString("\n")))
      if v.added.size > 10 then println(s"  ... and ${v.added.size - 10} more")
    if v.gone.nonEmpty then
      println(s"$project: $what: gone ${v.gone.size} of the record's ${record.debt.size} (--record keeps the improvement):")
      v.gone.take(50).foreach(g => println(s"  - ${Records.escape(g)}"))
      if v.gone.size > 50 then println(s"  ... and ${v.gone.size - 50} more")
    if v.passed then println(s"$project: $what: ${counted(record.debt.size - v.gone.size)}, within the record's ${record.debt.size}${if v.gone.isEmpty then "" else s", gone ${v.gone.size}"}")
    v.passed

// The absolute roots a transcript names, each replaced by a name of its own, the longest first: the directory teq
// ran in (paths under it become relative to it, as teq prints the ones it reports), the checkout's, the line's
// directory of logs and caches, the coursier cache, the JDK and the home directory. What a record holds names no
// machine.
final case class Roots(pairs: Vector[(String, String)]):
  // The first name of a root wins (the directory teq ran in over the checkout when they are one).
  private val ordered = pairs.filter(_._1.length > 1).distinctBy(_._1).sortBy(-_._1.length)
  def apply(text: String): String =
    ordered.foldLeft(text) { case (t, (root, name)) => t.replace(root + "/", if name.isEmpty then "" else name + "/").replace(root, if name.isEmpty then "." else name) }

object Roots:
  def of(cwd: Path, checkout: Path, out: Path): Roots =
    def both(p: Path, name: String) =
      val a = p.toAbsolutePath.normalize.toString
      val real = try p.toRealPath().toString catch case _: Exception => a
      Vector(a -> name, real -> name)
    val home = Paths.get(System.getProperty("user.home"))
    val coursier = Option(System.getenv("COURSIER_CACHE")).filter(_.nonEmpty).map(Paths.get(_)).getOrElse(home.resolve(".cache/coursier/v1"))
    val java = Option(System.getenv("JAVA_HOME")).filter(_.nonEmpty).map(Paths.get(_)).toVector
    Roots(both(cwd, "") ++ both(checkout, "$CHECKOUT") ++ both(out, "$OUT") ++ both(coursier, "$COURSIER") ++ java.flatMap(both(_, "$JAVA_HOME")) ++ both(home, "$HOME"))

// teq's transcripts as observations. A diagnostic is the record teq writes of it (`--diagnostics`: src/task/daemon.rs
// `shown`, src/source.rs `record_shown`, as dotty's vulpix's `TestReporter` collects the `Diagnostic`s and parses no
// text): its configuration, its severity, its whole message and the lines shown for it, its header, the message's
// further lines, the source line and the caret; its identity is that block, whatever its message holds, and in the
// transcript its lines are the record's, never one of teq's. src/task/daemon.rs's `compile` and `built` print a
// configuration's notes (`teq: <key>: <note>`), warnings and errors, then its summary: `<key>: <n> errors`, `<key>:
// checked in <ms> ms`, `<key>: <n> classes, <n> written in <ms> ms`; a build that failed outright `teq: <key>: <why>`
// and no summary, a generator's failure with its command's whole output, which teq records whole (`failed`, severity
// `failure`), as it records each line a passing generator printed (`noted`, severity `note`). A summary of n errors
// claims its configuration's errors recorded before it; a configuration that failed outright is its failure's record,
// every line of it its identity, whatever the command printed, and a line of teq's no record holds is never one. A
// one-shot build's `<n> errors found` claims the errors recorded of no configuration's. A compiler's crash is no
// outcome a record holds: a build's thread that panicked, a resident or the daemon that ended under the build, a
// panic's message, an error counted without its diagnostic are problems of completeness, as dotty's vulpix keeps a
// compiler crash apart from a test's expected errors (compiler/test/dotty/tools/vulpix/ParallelTesting.scala,
// `onComplete`: a `Failure` of the compilation is a fatal crash, whatever errors were expected), and so are a record
// the output does not show and a run that wrote no records. The verb running teq ends the transcript with `# exit <status>`, the end marker, once teq has exited.
// A test run's output is read by its channels, each for what it carries, as dotty's vulpix keeps the compiler's
// diagnostics apart from a test program's output (ParallelTesting.scala, `RunTest.verifyOutput`: `runMain`'s output
// compared on its own): `teq test` writes its own lines on stderr, a build's diagnostics, notes and count, its failures
// (`teq: <key>: ...`), each configuration's summary or end with nothing run (src/task/daemon.rs `built`,
// `test_configuration`, `test_js_configuration`); on stdout each run's stream, what the suites, their frameworks, the
// runner's JVM and node print and teq's report of each event, whose names and messages are the suites'; the builds'
// diagnostics in their records (`--diagnostics`); and the run's outcomes and the suites that ran in its events
// (`--events`). The drivers collect each apart and the judgment reads the diagnostics from their records, the counts
// and summaries from stderr past the records' lines (`scan` with `test`), and the outcomes and the suites from the
// events, never a stream a run's text shares nor a count: nothing a suite names, throws or prints, and nothing a
// message holds, is ever a diagnostic, a count or a summary.
object Transcript:
  private val ansi = "\u001b\\[[0-9;]*m"
  private val errorsLine = "^([^ :]+/[^ :]+): ([0-9]+) errors?$".r
  private val okLine = "^([^ :]+/[^ :]+): (?:checked|[0-9]+ classes, [0-9]+ written) in [0-9]+ ms(?:, the first build)?$".r
  private val teqLine = "^teq: ([^ :]+/[^ :]+): (.*)$".r
  private val leftOut = "^teq: compile left out ([^ ,]+),.*$".r
  private val endMarker = "^# exit (-?[0-9]+)$".r
  private val panic = "^thread '[^']*' panicked at .*".r
  // The outright failures that are a crash of teq's (src/task/daemon.rs, src/task/resident.rs).
  private val crash = "^teq: ([^ :]+/[^ :]+): (the build's thread panicked|the resident of .* ended|the daemon is ending).*".r

  // A build that failed outright in a generator (src/task/generators.rs: `<kind> generator <n> of <key>: <why>`), the
  // one failure of a configuration's build without a summary a record holds; any other line of teq's of a configuration
  // that has no summary is a note (a test run's) or a failure no record holds (a compile's).
  private val generatorFailure = "^[a-z]+ generator [0-9]+ of [^ :]+/[^ :]+: .*".r
  // A configuration's tests' end (src/task/daemon.rs `test_configuration`, `test_js_configuration`): its run's summary
  // (`summary`: `<key>: <n> suites, <p> passed, <f> failed[, <n> ignored, ...] in <ms> ms (built in <ms> ms)`), its
  // runner's failure (`<key>: the run failed: <why>`) or its end with nothing run.
  val runSummary = "^([^ ]+)/test: ([0-9]+) suites?, ([0-9]+) passed, ([0-9]+) failed".r
  private val runFailed = "^([^ ]+)/test: the run failed: .*".r
  val nothingToRun = "^([^ ]+)/test: (?:no suite to run|no suite depends on what changed|no test framework)$".r
  // A one-shot build's count of its errors (src/main.rs, `teq compiler`'s and `teq build`'s).
  private val foundLine = "^([0-9]+) errors? found$".r
  private def testsEnd(line: String): Option[String] =
    runSummary.findFirstMatchIn(line).orElse(runFailed.findFirstMatchIn(line)).orElse(nothingToRun.findFirstMatchIn(line)).map(_.group(1))
  // The informational notes teq prints in a configuration's name (`teq: <configuration>: <note>`), each with the line
  // that emits it; every other line of that shape in a test run is a failure of the run's. The parallel attempt's
  // giving way to one worker (src/main.rs `give_way_note`, `teq: a parallel attempt gave way to one worker: ...`) names
  // no configuration and so is never taken for one.
  val notes: Vector[(scala.util.matching.Regex, String)] = Vector(
    "^every suite, .*".r -> "src/task/daemon.rs test_configuration and test_js_configuration: why a run takes every suite",
    "^no memory mapping for the type store's overlays; one worker types the build$".r ->
      "src/types.rs MAPPING_REFUSED: the overlays' fallback to one worker, carried by the resident's answer (src/watch.rs) and printed by src/task/daemon.rs for a build's notes")
  def informational(line: String): Boolean = notes.exists(_._1.matches(line))

  def mark(status: Int): String = s"# exit $status\n"

  // What a transcript's builds showed: the configurations summarised or failed outright (`scope`), their errors by
  // configuration (`<key> <block>`) and outright failures (`<key> teq: <its whole block>`), and the problems; with
  // `test`, a test run's stderr or a run's transcript (`teq run`'s, `teq build`'s): a build's notes are no failure, a
  // configuration's tests' end ends what came before it.
  // Beside them, what the run's own lines say past the records' (no diagnostic's message ever one of them): each
  // configuration's summary with its counts by status (`summaries`), each end with nothing run (`idle`) and teq's
  // crashes (`crashes`).
  final case class Scan(scope: Vector[String], debt: Vector[String], problems: Vector[String], failing: Boolean, ended: Option[Int],
      summaries: Vector[(String, Map[String, Int])] = Vector.empty, idle: Vector[String] = Vector.empty, crashes: Vector[String] = Vector.empty)

  // A `teq compile` transcript: its configurations (each summarised once), its errors by configuration
  // (`<key> <block>`), a build that failed outright (`<key> teq: <its whole block>`), and the problems of a run that is
  // no evidence.
  def compile(text: String, records: Option[String], roots: Roots): Observation =
    val s = scan(text, records.fold(Vector.empty)(shown), roots, test = false)
    val problems = Vector.newBuilder[String] ++= s.problems
    if records.isEmpty then problems += unrecorded
    s.ended match
      case None => problems += "no end marker: the output is incomplete"
      case Some(status) =>
        val owed = if s.debt.isEmpty && !s.failing then 0 else 1
        if status == 124 then problems += "teq timed out"
        else if status != owed then problems += s"teq exits $status, where its diagnostics give $owed"
    if s.scope.isEmpty then problems += "no configuration was compiled"
    Observation(s.scope, s.debt, problems.result().distinct)

  // A `--diagnostics` file as a run left it: None where the binary wrote none. A verb deletes it before the run.
  def recorded(file: Path): Option[String] = Option.when(Files.isRegularFile(file))(Files.readString(file))

  // A run whose binary wrote no `--diagnostics` file: no evidence of its builds' diagnostics.
  val unrecorded = "teq wrote no diagnostics (--diagnostics): a binary from before the records, or a run cut short"

  // A diagnostic as teq's `--diagnostics <file>` records it (src/task/daemon.rs `shown`, src/source.rs
  // `record_shown`): the configuration whose build showed it (none for a one-shot build), its severity, and the lines
  // shown for it, the producer's extent, which its message's text never sets.
  final case class Shown(key: String, severity: String, text: String):
    val lines: Vector[String] = Transcript.lines(text)
    def error: Boolean = severity == "error"
    // A configuration's failure as teq showed it (src/task/daemon.rs `failed`), a generator's with its command's output.
    def failure: Boolean = severity == "failure"
    // A program `teq run` started once its build passed (src/task/run.rs), its output after teq's own lines.
    def started: Boolean = severity == "run"

  // The records of a `--diagnostics` file, in the order shown.
  def shown(records: String): Vector[Shown] =
    records.linesIterator.filter(_.trim.nonEmpty).map { line =>
      val j = Json.parse(line)
      Shown(j.get("key").map(_.str).getOrElse(""), j("severity").str, j("shown").str)
    }.toVector

  // A text's lines, a transcript's and a record's payload alike (`Shown.lines`), so that a diagnostic's lines are
  // the lines its rendering takes in the transcript whatever separators its message holds (LF, CR LF), and its
  // payload is never read as one of teq's lines, as dotty's TestReporter keeps the `Diagnostic` itself apart from
  // what a run prints (TestReporter.scala, `doReport`).
  def lines(text: String): Vector[String] = text.linesIterator.map(_.replaceAll(ansi, "")).toVector

  def scan(text: String, records: Vector[Shown], roots: Roots, test: Boolean): Scan =
    val lines = Transcript.lines(text)
    val scope, debt, problems, idle, crashes = Vector.newBuilder[String]
    val summaries = Vector.newBuilder[(String, Map[String, Int])]
    // The errors since the last configuration's own lines, each as its first line, its end (exclusive) and its
    // configuration, the build's record's extent.
    var pending = Vector.empty[(Int, Int, String)]
    // The records not met yet, in the order shown; a program's start shows nothing of its own.
    var expected = records.filterNot(_.started)
    // The configuration whose line of teq's came last and the line: its block runs on until the next configuration's.
    var region = Option.empty[(String, Int)]
    var ended = Option.empty[Int]
    // Whether teq failed otherwise than by errors it printed: a crash, an error counted without its diagnostic.
    var failing = false
    def block(from: Int, until: Int): String = roots(lines.slice(from, until).reverse.dropWhile(_.trim.isEmpty).reverse.mkString("\n"))
    // The region ends where the next configuration's lines start: a configuration of its own that failed outright, or
    // a note of a configuration summarised after it (`own`); the errors pending before then are its own, or, without
    // a region, a problem.
    // At the output's end (`end`) a one-shot build's errors, recorded of no configuration's, need no count: its count
    // claims them where it prints one, and the errors of the library bodies a program reaches print none (src/main.rs).
    def endRegion(at: Int, own: Option[String], where: String, end: Boolean = false): Unit =
      region match
        case Some((key, from)) if !own.contains(key) =>
          val whole = block(from, at).stripPrefix(s"teq: $key: ")
          // A failure is its record (`failure`), never a line of teq's the text runs on from: a region is a note's.
          if !test && crash.findFirstIn(lines(from)).isEmpty then
            problems += s"$key: teq: ${whole.linesIterator.next()}: a line of teq's and no summary, no failure recorded: a note without its build, or a failure no record holds"
            failing = true
          // Every error is consumed by a summary that claims it; a region of a note (or any other line of teq's)
          // closing over errors no summary claimed reports them.
          val unclaimed = pending.filter(_._1 < at)
          if unclaimed.nonEmpty then
            problems += s"${unclaimed.size} errors after $key's line of teq's, claimed by no summary: errors without their configuration's end"
            failing = true
          pending = pending.filter(_._1 >= at)
          region = None
        case _ =>
          val before = pending.filter(p => p._1 < at && !(end && p._3.isEmpty))
          if before.nonEmpty then problems += s"${before.size} errors $where"
          pending = pending.filter(_._1 >= at)
          if own.isEmpty then region = None
    // A crash of teq's a line says (a build's thread that panicked, a resident or the daemon that ended): whether it is one.
    def crashed(line: String): Boolean =
      crash.findFirstMatchIn(line).exists { m =>
        val said = s"${m.group(1)}: teq crashed: ${roots(line.stripPrefix(s"teq: ${m.group(1)}: "))}"
        problems += said
        crashes += said
        failing = true
        true
      }
    // A configuration's failure as teq recorded it (`failure`), its extent the record's, whatever its lines hold: a
    // crash; a generator's (`<kind> generator <n> of <key>: ...` with its command's whole output), the configuration's
    // build failed outright, the whole block its identity; or a failure no record holds. It ends what came before it.
    def failure(r: Shown, at: Int): Unit =
      val first = r.lines.headOption.getOrElse("")
      val why = first.stripPrefix(s"teq: ${r.key}: ")
      endRegion(at, None, s"before ${r.key}'s failure")
      if !crashed(first) then
        if generatorFailure.findFirstIn(why).nonEmpty then
          scope += r.key
          debt += s"${r.key} teq: ${roots(r.lines.reverse.dropWhile(_.trim.isEmpty).reverse.mkString("\n")).stripPrefix(s"teq: ${r.key}: ")}"
        else
          problems += (if test then s"${r.key}: teq: ${roots(why)}: a failure of the test run's, which no record holds"
            else s"${r.key}: teq: ${roots(why)}: no summary and no generator failing: a failure no record holds")
          failing = true
    def step(line: String, i: Int): Unit =
      if ended.nonEmpty && line.trim.nonEmpty then problems += s"output after the end marker: ${roots(line)}"
      if panic.findFirstIn(line).nonEmpty then
        problems += s"teq panicked: ${roots(line)}"
        crashes += s"teq panicked: ${roots(line)}"
        failing = true
      crashed(line)
      if line.startsWith("teq: the daemon ended without an answer") then crashes += "teq's daemon ended without an answer"
      line match
        case endMarker(status) if ended.isEmpty =>
          endRegion(i, None, "after the last summary: the output stops within a configuration", end = true)
          ended = Some(status.toInt)
        case errorsLine(key, n) =>
          // A count claims its configuration's errors recorded before it (a record of no configuration's any count's).
          val claimed = pending.filter(p => p._3 == key || p._3.isEmpty)
          endRegion(claimed.headOption.fold(i)(_._1), Some(key), s"before $key's own")
          region = None
          scope += key
          if claimed.size != n.toInt then
            problems += (if claimed.isEmpty then s"$key: ${n.toInt} errors reported without a diagnostic" else s"$key: ${n.toInt} errors reported, ${claimed.size} printed")
            failing = true
          claimed.foreach((from, until, _) => debt += s"$key ${block(from, until)}")
          pending = pending.filterNot(claimed.contains)
        case okLine(key) =>
          endRegion(i, Some(key), s"before $key's summary of none")
          region = None
          scope += key
          pending = Vector.empty
        case foundLine(n) =>
          // A one-shot build's count (`teq compiler`'s, `teq build`'s) claims the errors recorded of no configuration's
          // before it, as a configuration's count claims its own.
          val claimed = pending.filter(_._3.isEmpty)
          endRegion(claimed.headOption.fold(i)(_._1), None, "before the build's count")
          if claimed.size != n.toInt then
            problems += s"${n.toInt} errors counted, ${claimed.size} printed: errors without their diagnostics"
            failing = true
          pending = pending.filterNot(claimed.contains)
        case leftOut(project) =>
          endRegion(i, None, s"before teq left $project out")
          problems += s"teq left $project out (a generator only the build tool runs): export again"
        case _ if test && testsEnd(line).nonEmpty =>
          // A configuration's tests' end, teq's own line on its stderr as the counts are: it ends the block before it,
          // and the errors before it no count claimed are a build's without its end.
          for m <- runSummary.findFirstMatchIn(line) do
            val others = ", ([0-9]+) (ignored|skipped|pending|canceled)".r.findAllMatchIn(line.drop(m.end)).map(o => o.group(2) -> o.group(1).toInt).toMap
            summaries += m.group(1) -> (Map("suites" -> m.group(2).toInt, "passed" -> m.group(3).toInt, "failed" -> m.group(4).toInt) ++ others).filter(_._2 > 0)
          for m <- nothingToRun.findFirstMatchIn(line) do idle += m.group(1)
          endRegion(i, None, s"before ${testsEnd(line).get}/test's end, claimed by no count: errors without their configuration's end")
        case teqLine(key, why) =>
          // In a test run every line of teq's in a configuration's name no record holds is judged on its own, whatever
          // came before it of the configuration's: an informational note, a crash (judged above), or a failure of the
          // run's, which no record holds; a generator's failure is its record (`failure`), never this line.
          if test && !informational(why) && crash.findFirstIn(line).isEmpty then
            problems += s"$key: teq: $why: a failure of the test run's, which no record holds"
            failing = true
          if !region.exists(_._1 == key) then
            endRegion(i, Some(key), s"before $key's line")
            region = Some((key, i))
        case _ =>
          if line.startsWith("teq: the daemon ended without an answer") then problems += "teq's daemon ended without an answer"
          else if line == "teq: nothing to compile" then problems += "teq compiled nothing"
    var i = 0
    while i < lines.length do
      // The next record's lines where they are shown: a diagnostic, its extent the build's record's, whatever its
      // message holds; its lines no line of teq's.
      val next = expected.headOption.filter(r => lines.slice(i, i + r.lines.length) == r.lines)
      for r <- next do
        if r.error then pending = pending :+ (i, i + r.lines.length, r.key)
        else if r.failure then failure(r, i)
        expected = expected.tail
        i += r.lines.length
      if next.isEmpty then
        step(lines(i), i)
        i += 1
    if ended.isEmpty then
      endRegion(lines.size, None, "after the last summary: the output stops within a configuration", end = true)
    // A record the output does not show where it comes: the two accounts of the build differ.
    if expected.nonEmpty then
      problems += s"${expected.size} diagnostics the build recorded and its output does not show in their order, the first ${roots(expected.head.lines.head)}"
      failing = true
    Scan(scope.result(), debt.result(), problems.result().distinct, failing, ended, summaries.result(), idle.result(), crashes.result().distinct)

  // A transcript's crashes of teq's: a panic, a build's thread that panicked, a resident or the daemon that ended, as
  // its own lines say them past its diagnostics' records (`scan`), never a diagnostic's message; none in the
  // transcript of a program that ran (`ran`), whose output is the program's.
  def crashes(text: String, records: Option[String], roots: Roots): Vector[String] =
    val all = records.fold(Vector.empty)(shown)
    if ran(all) then Vector.empty else scan(text, all, roots, test = true).crashes

  // Whether the records say a program ran (`teq run`'s, src/task/run.rs, once its build passed): the transcript
  // after teq's own lines is then the program's output, which is never the build's evidence, as dotty's vulpix
  // judges a program's output apart from the compiler's reporters (ParallelTesting.scala, `RunTest.verifyOutput`).
  def ran(records: Vector[Shown]): Boolean = records.exists(_.started)

  // The builds a scan saw fail, each configuration's items.
  def builds(s: Scan): Map[String, Vector[String]] = s.debt.groupBy(_.takeWhile(_ != ' '))

  // The errors of a one-shot build (`teq build`, which prints no configuration's summary), each its whole block as
  // the build recorded it (its header, the message's further lines, the source and the caret), normalised.
  def oneShot(records: Vector[Shown], roots: Roots): Vector[String] =
    records.filter(_.error).map(r => roots(r.lines.reverse.dropWhile(_.trim.isEmpty).reverse.mkString("\n")))

  // An observation with the line's own departures beside teq's diagnostics (a generated input not the expected
  // bytes, a generated file not written), each an item of the debt.
  def withDebt(o: Observation, items: Vector[String]): Observation = o.copy(debt = o.debt ++ items)

// The export's configurations (the lock teq's exporters write, `projects:` → `<project>:` with its `platform`, its
// `description` (`lib`, the stand-ins; `sources`, the compile sources the build selects) and its `configurations:` →
// `<configuration>:` with its `sources`, its `transitive` and its `classpath`, whose `{configuration: <c>, project:
// <p>}` entries are the configurations it builds on): the coverage a compile owes, every exported configuration of
// those asked for, its inputs its session's, derived from the export and never from a transcript; and what a run
// covered, a configuration covered by one the run summarised only where teq's planner would have that one's resident
// type it
// (src/task/daemon.rs `plan`, src/task/export.rs `serves`, `session_inputs`, `widest`): the configuration itself; or,
// where it is transitive (a configuration that is not, a Mill module's, is typed by its own resident alone), one of
// its platform whose session's inputs (the stand-ins, the closure's source roots, the description's selection of the
// compile sources honoured) hold its own, or its project's widest. A baseline or a migration of a run that covered
// less is refused before it publishes anything, as any run that covered less fails.
object Locks:
  final case class Configuration(sources: Vector[String], uses: Vector[String], transitive: Boolean, frameworks: Boolean = false)
  final case class Project(platform: String, lib: Option[String], selected: Option[Vector[String]], configurations: Map[String, Configuration])

  def read(lock: Path): Map[String, Project] =
    val out = scala.collection.mutable.LinkedHashMap.empty[String, Project]
    var inProjects = false
    var section, project, conf, list = ""
    def unquote(t: String) = t.stripPrefix("\"").stripSuffix("\"")
    def update(f: Project => Project): Unit = out(project) = f(out(project))
    def updateConf(f: Configuration => Configuration): Unit = update(p => p.copy(configurations = p.configurations.updated(conf, f(p.configurations(conf)))))
    val use = "^- \\{configuration: ([^,]+), project: ([^}]+)\\}$".r
    for line <- Files.readString(lock).linesIterator do
      val indent = line.takeWhile(_ == ' ').length
      val t = line.trim
      if t.nonEmpty then
        if indent == 0 then inProjects = t == "projects:"
        else if inProjects then
          if indent == 2 then
            project = unquote(t.stripSuffix(":"))
            out(project) = Project("", None, None, Map.empty)
            section = ""
          else if indent == 4 then
            section = t.takeWhile(_ != ':')
            if section == "platform" then update(_.copy(platform = t.drop("platform:".length).trim))
          else if section == "configurations" && indent == 6 then
            conf = unquote(t.takeWhile(_ != ':'))
            update(p => p.copy(configurations = p.configurations.updated(conf, Configuration(Vector.empty, Vector.empty, true))))
            list = ""
          else if section == "configurations" && indent == 8 then
            list = if t.endsWith("[]") then "" else t.takeWhile(_ != ':')
            if t == "transitive: false" then updateConf(_.copy(transitive = false))
          else if section == "configurations" && indent == 10 && list == "frameworks" && t.startsWith("- ") then
            updateConf(_.copy(frameworks = true))
          else if section == "configurations" && indent == 10 && t.startsWith("- ") then
            list match
              case "sources" => updateConf(c => c.copy(sources = c.sources :+ t.drop(2)))
              case "classpath" => t match
                case use(c, p) => updateConf(x => x.copy(uses = x.uses :+ s"${unquote(p.trim)}/${unquote(c.trim)}"))
                case _ => ()
              case _ => ()
          else if section == "description" && indent == 6 then
            list = t.takeWhile(_ != ':')
            if list == "lib" then update(_.copy(lib = Some(unquote(t.drop("lib:".length).trim))))
            if list == "sources" then update(_.copy(selected = Some(Vector.empty)))
          else if section == "description" && indent == 8 && list == "sources" && t.startsWith("- ") then
            update(p => p.copy(selected = p.selected.map(_ :+ t.drop(2))))
    out.toMap

  private def configuration(ps: Map[String, Project], key: String): Option[Configuration] =
    ps.get(key.takeWhile(_ != '/')).flatMap(_.configurations.get(key.dropWhile(_ != '/').drop(1)))

  // The configurations a configuration builds on, with it.
  def closure(ps: Map[String, Project], key: String): Vector[String] =
    val seen = scala.collection.mutable.LinkedHashSet.empty[String]
    def walk(k: String): Unit = if seen.add(k) then configuration(ps, k).foreach(_.uses.foreach(walk))
    walk(key)
    seen.toVector

  private def roots(ps: Map[String, Project], keys: Vector[String]): Vector[String] = keys.flatMap(k => configuration(ps, k).toVector.flatMap(_.sources)).distinct

  // A configuration's session's inputs (`Export::session_inputs`): the stand-ins, then the closure's source roots,
  // the compile part of them as the description selects it where it says.
  def inputs(ps: Map[String, Project], key: String): Set[String] =
    val project = key.takeWhile(_ != '/')
    val p = ps.get(project)
    val all = roots(ps, closure(ps, key))
    val selected = p.flatMap(_.selected) match
      case Some(sel) =>
        val compile = roots(ps, closure(ps, s"$project/compile"))
        sel ++ all.filter(r => !compile.contains(r) && !sel.contains(r))
      case None => all
    (p.flatMap(_.lib).toVector ++ selected).toSet

  // A project's widest configuration (`Export::widest`): its compile one where that is not transitive, else its test
  // one where it has one.
  def widest(ps: Map[String, Project], project: String): String =
    val confs = ps.get(project).map(_.configurations).getOrElse(Map.empty)
    if confs.get("compile").exists(!_.transitive) then s"$project/compile"
    else if confs.contains("test") then s"$project/test" else s"$project/compile"

  // Whether a resident of `resident` types `key`, as the planner has it.
  def covers(ps: Map[String, Project], resident: String, key: String): Boolean =
    def platform(k: String) = ps.get(k.takeWhile(_ != '/')).map(_.platform)
    resident == key || configuration(ps, key).forall(_.transitive) && platform(resident) == platform(key) &&
      (inputs(ps, key).subsetOf(inputs(ps, resident)) || widest(ps, key.takeWhile(_ != '/')) == resident)

  // The compile configurations a compile owes, of the projects asked for (every project when none): each project's,
  // whatever sources it lists of its own, as `teq compile` wants every one (src/task/daemon.rs `compile`), its inputs
  // being its session's (the stand-ins, the closure's roots, the description's selection of its compile sources).
  def owed(ps: Map[String, Project], projects: Vector[String]): Vector[String] =
    ps.toVector.flatMap((p, project) => project.configurations.get("compile").filter(_ => projects.isEmpty || projects.contains(p)).map(_ => s"$p/compile")).sorted

  // The projects whose tests a run of `teq test` owes (src/task/daemon.rs: every test configuration with a framework),
  // of the projects asked for (every project when none): the export's, never the run's.
  def testUnits(lock: Path, projects: Vector[String] = Vector.empty): Vector[String] =
    read(lock).toVector.collect { case (p, project) if project.configurations.get("test").exists(_.frameworks) && (projects.isEmpty || projects.contains(p)) => p }.sorted

  // Whether a failed configuration's resident types a unit's tests, as teq's planner has it.
  def typesTests(lock: Path): (String, String) => Boolean =
    val ps = read(lock)
    (key, unit) => covers(ps, key, s"$unit/test")

  // A compile's observation with the coverage the export owes it: each owed configuration no summarised one covers a
  // problem.
  def hold(o: Observation, lock: Path, projects: Vector[String] = Vector.empty): Observation =
    val ps = read(lock)
    val missing = owed(ps, projects).filterNot(k => o.scope.exists(r => covers(ps, r, k)))
    o.copy(problems = o.problems ++ missing.map(k => s"$k: the export holds it and no configuration the run compiled types it"))

// A test line's departures from the oracle's inventory (tests.json, which stays the oracle's own record, its cases'
// statuses and multiplicity whole): its scope the oracle's projects, each covered once, by its run (its summary in the
// transcript) or as blocked (none of its suites ran, and a build of its own failed, the configuration a test of its
// is typed in: a prerequisite compile, whose errors the compile line's record holds); its debt each case whose
// statuses are not the oracle's, its item the case with what it showed and what the oracle has, as many times as each
// side reports it (`<project>: <suite>: <case>: Failure (tests.json: Success)`, `... : did not run (...)`, `... :
// Success x2 (...)`), so that a known departure that shows otherwise is another item and one that disappears is gone
// only where the case ran as the oracle's; a blocked project its build's items as a compile's record holds them, each
// `<project>: blocked by <key> <block>` (another error, or a count without its diagnostic, being no such block), its
// cases none; a suite or a case of the run the oracle lacks, and a suite of the oracle's that did not run in a
// project that ran, an item of its own; and the problems of a run that is no evidence: a project neither run nor
// blocked, an exit no failing case and no blocked build explains (dotty's community runner holds a build to a
// successful run, CommunityBuildRunner.scala), a crash, a timeout, a daemon gone, output without its end.
final case class TestCase(project: String, suite: String, name: String):
  def item: String = if suite.isEmpty then s"$project: $name" else s"$project: $suite: $name"

object Deviations:
  def times(statuses: Vector[String]): String =
    if statuses.isEmpty then "did not run"
    else statuses.groupBy(identity).toVector.sortBy(_._1).map((st, all) => if all.size == 1 then st else s"$st x${all.size}").mkString(", ")

  def failed(status: String): Boolean = status == "Failure" || status == "Error"

  // The category of an event's status a summary counts it under.
  def category(status: String): String = if status == "Success" then "passed" else if failed(status) then "failed" else status.toLowerCase

  // The suites an invocation's events say ran (`teq test --events`, `task::events::Tally::ran_line`: the project, the
  // suite and `ran`, tab-separated), a suite that ran no case among them.
  def ran(text: String): Vector[(String, String)] =
    text.linesIterator.flatMap(_.split("\t", -1) match
      case Array(project, suite, "ran") => Some(project -> suite)
      case _ => None).toVector

  // The cases of an invocation's events (`teq test --events`, `task::events::Event::line`: the project, the suite, the
  // selector, the status, the thrown, tab-separated): a case is the test a `test <name>` selector names, another
  // selector as it is written; `qualified`, the test's name is the case under no suite (Mill's `<suite>.<test>`).
  def events(text: String, qualified: Boolean): Vector[(TestCase, String)] =
    text.linesIterator.filter(_.nonEmpty).flatMap(_.split("\t", -1) match
      case Array(project, suite, selector, status, _*) =>
        val test = if selector.startsWith("test ") then selector.drop(5) else selector
        Some(TestCase(project, if qualified then "" else suite, test) -> status)
      case _ => None).toVector

  // A test line's run judged, whatever the driver gathered it (lila's, the Scala.js projects', Mill's): one function of
  // the evidence, so that a rule holds for every driver alike.
  // - Builds: every build that failed in any invocation counts, each configuration's items (its errors' whole blocks,
  //   a generator's failure) once; a count without its diagnostic is a problem. A build of a unit of the oracle's
  //   blocks it where none of its cases ran; a build of no unit of the oracle's is its own departure.
  // - Executed cases: every event of every invocation (`--events`, the run's outcomes: its report of them on stdout,
  //   whose names and messages are the suites', is never read), with its framework's status and as many times as it
  //   came; an invocation asked for events that wrote none is a problem, as is an execution a unit's failed build
  //   contradicts (a case of its ran), cases of a unit that ran without its tests' completing (no summary on teq's
  //   stderr), a unit's tests completing twice, and a unit whose summary and events count its outcomes otherwise.
  // - Exits: each invocation's 0, or 1 where a case of its failed or a build in it failed; a timeout or another exit is a
  //   problem (dotty's community runner holds a build to a successful run, CommunityBuildRunner.scala).
  // - The departures: a blocked unit's builds' items (`<unit>: blocked by <item>`), its cases none; every other case
  //   whose statuses are not the oracle's, with what it showed and what the oracle has, as many times as each side
  //   reports it; a suite of the run the oracle lacks, a suite of the oracle's that did not run in a unit that ran.
  def judge(e: TestEvidence, roots: Roots): Observation =
    val problems = Vector.newBuilder[String] ++= e.problems
    val debt = Vector.newBuilder[String] ++= e.debt
    def unitOf(key: String) = key.takeWhile(_ != '/')
    val failedBuilds = scala.collection.mutable.LinkedHashMap.empty[String, Vector[String]]
    val run = Vector.newBuilder[(TestCase, String)]
    val summaries = Vector.newBuilder[String]
    val idle = Vector.newBuilder[String]
    val summarised = scala.collection.mutable.Map.empty[String, Map[String, Int]].withDefaultValue(Map.empty)
    // The suites the events say ran, by name (`Deviations.ran`).
    val named = Vector.newBuilder[(String, String)]
    for i <- e.invocations do
      // teq's own lines are its stderr's (`Transcript`), read past its builds' diagnostics, the records teq wrote of
      // them (`--diagnostics`): what the suites printed, on its stdout, and what a diagnostic's message holds, are
      // never one; every completion and count below is the scan's.
      if i.diagnostics.isEmpty then problems += s"${i.what}: ${Transcript.unrecorded}"
      val scan = Transcript.scan(i.err, i.diagnostics.fold(Vector.empty)(Transcript.shown), roots, test = true)
      idle ++= scan.idle
      problems ++= scan.problems.map(p => s"${i.what}: $p")
      val own = Transcript.builds(scan)
      for (key, items) <- own do
        failedBuilds.get(key) match
          case None => failedBuilds(key) = items
          case Some(before) if before == items => ()
          case Some(_) => problems += s"$key: its build failed otherwise in ${i.what}: one build, two failures"
      val done = scan.summaries
      summaries ++= done.map(_._1)
      for (u, counts) <- done do
        summarised(u) = (summarised(u).keySet ++ counts.keySet).map(k => k -> (summarised(u).getOrElse(k, 0) + counts.getOrElse(k, 0))).toMap
      val cases = i.events match
        case None =>
          problems += s"${i.what}: it wrote no events (--events)"
          Vector.empty
        case Some(text) => events(text, e.qualified)
      run ++= cases
      named ++= i.events.fold(Vector.empty)(Deviations.ran)
      i.status match
        case 0 => ()
        case 124 => problems += s"${i.what}: teq test timed out"
        case 1 if own.nonEmpty || cases.exists(c => failed(c._2)) => ()
        case 1 => problems += s"${i.what} exits 1, no case failing and no build failing in it: the run did not end as it should"
        case other => problems += s"${i.what} exits $other"
    val ran = run.result()
    val completions = summaries.result()
    for (u, all) <- completions.groupBy(identity).toVector.sortBy(_._1) if all.size > 1 do problems += s"$u: its tests completed ${all.size} times"
    // A summary's counts are its events', status by status: a case a summary counts that the events do not show is
    // missing from the run, a problem, never an improvement.
    def shown(c: Map[String, Int]) = c.toVector.sortBy(_._1).map((k, n) => s"$n $k").mkString(", ")
    // The suites that ran are the events' by name, each suite's own line (a suite that ran no case too) or a case of
    // its: a summary counting more suites than the events name is a suite missing from the run, never a count's word.
    val caseSuites = ran.map(c => (c._1.project, c._1.suite)).filter(_._2.nonEmpty).distinct
    val suitesRan = (named.result() ++ caseSuites).distinct
    for u <- summarised.keys.toVector.sorted do
      val byEvents = ran.filter(_._1.project == u).groupMapReduce(c => category(c._2))(_ => 1)(_ + _)
      val outcomes = summarised(u) - "suites"
      if outcomes != byEvents then problems += s"$u: its summary counts ${shown(outcomes)}, its events ${if byEvents.isEmpty then "none" else shown(byEvents)}"
      val counted = summarised(u).getOrElse("suites", 0)
      val names = suitesRan.count(_._1 == u)
      if names < counted then problems += s"$u: its summary counts $counted suites, its events name $names: a suite that did not run"
    val completed = (completions ++ e.completedWithoutRun ++ idle.result()).distinct
    // Every unit the export owes ended its tests somehow; a build of it that failed counts as its end, a build that did
    // not show nothing, so that a build omitted from a run is no improvement.
    for u <- e.owed if !completed.contains(u) && !failedBuilds.keys.exists(k => unitOf(k) == u || e.types(k, u)) do
      problems += s"$u: the export holds its tests, and the run shows neither their summary, nor nothing to run, nor its build's failure"
    val executed = ran.map(_._1.project).toSet ++ completed
    val builds = e.units.map(u => u -> failedBuilds.toVector.filter(k => unitOf(k._1) == u).flatMap(_._2)).filter(_._2.nonEmpty).toMap
    for (key, items) <- failedBuilds.toVector.sortBy(_._1) if !e.units.contains(unitOf(key)); item <- items do debt += s"$item (a build of no unit of tests.json's)"
    val isBlocked = builds.keySet.filter(u => !executed.contains(u))
    for u <- builds.keys.toVector.sorted if executed.contains(u) do
      problems += s"$u: a build of its failed and ${ran.count(_._1.project == u)} of its cases ran: an execution its build's failure contradicts"
    for u <- e.units if !completed.contains(u) && !builds.contains(u) && ran.exists(_._1.project == u) do
      problems += s"$u: ${ran.count(_._1.project == u)} of its cases ran without its tests' completing (no summary)"
    for u <- e.units if !executed.contains(u) && !isBlocked.contains(u) do problems += s"$u: its suites did not run, and no build of its failed"
    val (want, got) = (e.oracle.groupMap(_._1)(_._2), ran.groupMap(_._1)(_._2))
    for u <- isBlocked.toVector.sorted; item <- builds(u) do debt += s"$u: blocked by $item"
    for c <- (e.oracle.map(_._1) ++ ran.map(_._1)).distinct if !isBlocked.contains(c.project) do
      val (w, g) = (want.getOrElse(c, Vector.empty).sorted, got.getOrElse(c, Vector.empty).sorted)
      if w != g then debt += s"${c.item}: ${times(g)} (tests.json: ${if w.isEmpty then "none" else times(w)})"
    // The oracle holds the suites that report cases (`Tests.heldSuites`): a suite not the oracle's is one that reported
    // cases; one of the oracle's ran where the events name it, its own line or a case of it.
    val runSuites = e.listedSuites.getOrElse(suitesRan)
    for (u, s) <- e.listedSuites.getOrElse(caseSuites).distinct if !e.oracleSuites.contains((u, s)) do debt += s"$u: $s (a suite not the oracle's)"
    for (u, s) <- e.oracleSuites.distinct if completed.contains(u) && !runSuites.contains((u, s)) do debt += s"$u: $s (a suite that did not run)"
    Observation(completed.filter(e.units.contains) ++ isBlocked.toVector.sorted, debt.result(), problems.result().distinct)

// One invocation of `teq test` a driver made: what it was (`teq test`, `core.js --list`), its exit, what it printed on
// each channel, collected apart (`output` its stdout, the runs' streams, a listing's suites, which the judgment never
// reads; `err` its stderr, teq's own lines: `Transcript`), the records of its builds' diagnostics it wrote
// (`--diagnostics`, None where it wrote none), and the events it wrote (`--events`): None where it was asked for them
// and wrote none, Some("") where it was not asked (a listing).
final case class Invocation(what: String, status: Int, output: String, err: String, diagnostics: Option[String], events: Option[String])

object Invocation:
  // An invocation's two channels for a reader and a record's evidence: stdout, then stderr under a line naming it.
  def shown(out: String, err: String): String = s"$out# teq's stderr\n$err"

// A test line's run, the evidence its record judges (`Deviations.judge`), whatever the driver: its invocations; the
// oracle's units (lila's and the Scala.js projects' projects, Mill's modules) with their cases and suites; how an
// event names its case (`qualified`: Mill's `<suite>.<test>` under no suite); the units complete with nothing to run
// (Mill's module whose listing passed with no suite); the suites the run listed where the driver lists them (Mill's);
// the driver's own departures and problems (the Scala.js projects' generated files, Mill's listing against its run and
// its modules beyond the oracle's); and the units whose tests the run owes by the export (`Locks.testUnits`, Mill's
// modules it listed), each of which must show how its tests ended, a summary, a build that failed, or nothing to run,
// the build that failed its own or that of a resident teq's planner has type the unit's tests (`types`, a failed
// configuration and a unit: `Locks.covers` over the export, a test configuration typed by another's resident printing
// its failure under that resident's key).
final case class TestEvidence(invocations: Vector[Invocation], units: Vector[String], oracle: Vector[(TestCase, String)],
    oracleSuites: Vector[(String, String)], qualified: Boolean = false, completedWithoutRun: Vector[String] = Vector.empty,
    listedSuites: Option[Vector[(String, String)]] = None, debt: Vector[String] = Vector.empty, problems: Vector[String] = Vector.empty,
    owed: Vector[String] = Vector.empty, types: (String, String) => Boolean = (_, _) => false)

// A run line's named assertions (run.json, ruling of 2026-10-16: named assertions, never a digest of a page or a count
// of its colours): its scope the assertions the run names, each checked once; its debt each one the run did not hold,
// identified by its name and how (`<name>: failed`, `<name>: not reached`), so that a known violation the run no longer
// shows is gone, and one never reached must pass once a run reaches it; a run whose build failed is its block,
// `blocked: <the build that failed>` with the build's own items (its errors' whole blocks, as a compile's record holds
// them), every assertion unexecuted beside it (`not reached`), none allowed to fail.
object Assertions:
  private val outcome = "^assert (.+?): (ok|FAIL .*|not reached)$".r
  private val declaration = "^assert (\\S(?:.*\\S)?)$".r

  // The assertions a checker declares (`bench/smoke.mjs <scenario> --assertions`, a line `assert <name>` each), taken
  // from a listing that completed alone: an exit of 0 within its deadline, a line at least, every line a declaration
  // that reads as no outcome, no name twice; else why the listing declares nothing, before any evidence is made of a
  // run, as dotty's vulpix compares a run's output only once its execution succeeded, a failure or a timeout judged
  // apart (ParallelTesting.scala, `RunTest.verifyOutput`).
  def declared(status: Int, timedOut: Boolean, listing: String): Either[String, Vector[String]] =
    val lines = listing.linesIterator.toVector
    val malformed = lines.find(l => declaration.findFirstIn(l).isEmpty || outcome.findFirstIn(l).nonEmpty)
    val names = lines.collect { case declaration(n) => n }
    if timedOut then Left("the listing timed out")
    else if status != 0 then Left(s"the listing exits $status")
    else if lines.isEmpty then Left("the listing declares no assertion")
    else if malformed.nonEmpty then Left(s"the listing's line `${malformed.get}` declares no assertion")
    else if names.distinct.size != names.size then Left(s"the listing declares `${names.diff(names.distinct).head}` twice")
    else Right(names)

  // The `assert <name>: <outcome>` lines of a run's log (bench/smoke.mjs's, lila's run verb's), held to the names the
  // checker declares (`names`, its own account of what it checks, never the log's): each declared assertion once,
  // and no other, or the run is no evidence, as dotty's vulpix holds a run's output to its check file, every line
  // (ParallelTesting.scala, `RunTest.verifyOutput`), so that no line the log lost or gained shapes the record's scope.
  def observe(log: String, names: Vector[String], problems: Vector[String]): Observation =
    val seen = log.linesIterator.collect { case outcome(name, result) => name -> result }.toVector
    val none = if seen.isEmpty then Vector("the run named no assertion: it did not run to its end") else Vector.empty
    val (declared, said) = (Multiset.counts(names), Multiset.counts(seen.map(_._1)))
    val departures = if seen.isEmpty then Vector.empty else
      names.distinct.filter(n => !said.contains(n)).map(n => s"$n: declared by the checker, and the run shows no outcome of it") ++
        said.toVector.sortBy(_._1).collect {
          case (n, k) if !declared.contains(n) => s"$n: an outcome of no assertion the checker declares"
          case (n, k) if k != declared(n) => s"$n: $k outcomes, the checker declaring it ${declared(n)} times"
        }
    Observation(seen.map(_._1), seen.collect { case (n, r) if r != "ok" => s"$n: ${if r == "not reached" then "not reached" else "failed"}" }, problems ++ none ++ departures)

  // A run line's run judged, whatever the driver gathered it (lila's `teq run`, Mill's `teq build` and smoke): one
  // function of the evidence. The run is blocked by its build only where the build failed on its own errors by teq's
  // exit of 1 before any assertion could be checked: its block, with those errors, and every assertion unexecuted
  // (`not reached`), none allowed to fail. A build that failed otherwise (with no error of its, a timeout, a signal,
  // an exit of 0 that left nothing to run) is no block a record holds. A run whose build did not fail (lila's still
  // running at the readiness deadline among them) is judged by its assertions as it checked them.
  def judge(e: RunEvidence): Observation =
    e.failedBuild match
      case Some((1, errors)) if errors.nonEmpty =>
        Observation(e.names, e.names.map(n => s"$n: not reached") ++ errors.map(x => s"blocked: ${e.build}: $x"), e.problems)
      case Some((status, _)) =>
        val why = status match
          case 1 => "failed with no error of its"
          case 124 => "timed out"
          case 0 => "exits 0 and leaves nothing to run"
          case other => s"exits $other, not as a build failing on its errors does (1)"
        val seen = observe(e.shown, e.names, e.problems :+ s"${e.build} $why: no block a record holds")
        if seen.scope.isEmpty then seen.copy(scope = e.names, debt = e.names.map(n => s"$n: not reached")) else seen
      case None => checked(observe(e.shown, e.names, e.problems), e.checker)

  // The assertions a checker printed held to its exit and its verdict (bench/smoke.mjs: 0 and `smoke:` where every one
  // held, 1 and `FAIL smoke:` where one did not): a contradiction, no verdict or another exit is a problem, so that
  // a run is never judged by its assertions where its checker says otherwise.
  def checked(o: Observation, checker: Option[(Int, Option[Boolean])]): Observation = checker match
    case None => o
    case Some((status, verdict)) =>
      val held = o.debt.isEmpty
      val said = if held then "every assertion held" else "an assertion did not hold"
      val problems = Vector.newBuilder[String]
      if verdict.isEmpty then problems += s"the checker printed no verdict ($said)"
      if status != 0 && status != 1 then problems += s"the checker exits $status"
      else if (status == 0) != held then problems += s"the checker exits $status where $said"
      for v <- verdict if v != held do problems += s"the checker's verdict is ${if v then "a pass" else "a failure"} where $said"
      o.copy(problems = o.problems ++ problems.result())

// A run line's run as a driver gathers it (`Assertions.judge`): the assertions it names, the build of what it runs and,
// where that build failed before any assertion could be checked, its exit and its own errors (each its whole block,
// normalised), the assertions' outcomes as the run checked them (`assert <name>: <outcome>` lines, none where none
// was), the driver's problems (a crash), and where a checker of its own printed them (Mill's bench/smoke.mjs), its exit
// and its verdict (a pass, a failure, none printed).
final case class RunEvidence(names: Vector[String], build: String, failedBuild: Option[(Int, Vector[String])], shown: String, problems: Vector[String],
    checker: Option[(Int, Option[Boolean])] = None)

object RunEvidence:
  // A transcript of `teq run`'s or `teq build`'s: the errors' whole blocks of the builds that failed, as each driver's
  // evidence names a build's own errors, a configuration's (`teq run`'s) or a one-shot build's (`teq build`'s, which
  // names no configuration), and the scan's problems either way (`Transcript.scan`): a count without its diagnostics,
  // a record the output does not show or shows cut short, a crash. `teq run`'s program writes on teq's own two
  // channels, which it inherits once its build has passed (src/task/run.rs): a build that failed ran no program, so
  // its transcript is teq's alone and scanned whole; one whose program ran (`Transcript.ran`, the records' own
  // account) failed in nothing, and its transcript, the program's output after teq's lines, is never read.
  def errors(text: String, records: Option[String], roots: Roots): (Vector[String], Vector[String]) =
    if records.isEmpty then (Vector.empty, Vector(Transcript.unrecorded))
    else if Transcript.ran(Transcript.shown(records.get)) then (Vector.empty, Vector.empty)
    else
      val shown = Transcript.shown(records.get)
      val s = Transcript.scan(text, shown, roots, test = true)
      val built = Transcript.builds(s)
      (if built.nonEmpty then built.toVector.sortBy(_._1).flatMap(_._2) else Transcript.oneShot(shown, roots), s.problems)

// What a line does with its record: judges the run against it; or, from a run with no problem of completeness,
// steps it (`--record`: the run's debt as the record, admitted against the one it replaces, from a passing run
// alone), makes the first one (`--baseline`) or migrates it to new inputs (`--migrate <reason>`), the two with the
// run's normalised transcript as their evidence.
enum Mode:
  case Judge, Record, Baseline
  case Migrate(reason: String)

object Mode:
  // The mode a verb's options ask for: at most one of --record, --baseline and --migrate <reason>.
  def of(args: Args): Mode =
    val (record, baseline, migrate) = (args.flag("record"), args.flag("baseline"), args.option("migrate"))
    if Seq(record, baseline, migrate.nonEmpty).count(identity) > 1 then args.fail("--record, --baseline and --migrate exclude each other")
    if baseline then Mode.Baseline else if record then Mode.Record else migrate.fold(Mode.Judge)(r => Mode.Migrate(r))

object Lines:
  def provenance(kind: String, reason: String, here: Path, shape: Shape, binary: String, staged: Map[String, Array[Byte]]): Provenance =
    val pin = try Json.parse(Files.readString(here.resolve("pin.json")))("commit").str catch case _: Exception => ""
    Provenance(kind, reason, binary.split(' ').lift(2).getOrElse(""), binary, pin, Records.inputsOf(here, shape, staged))

  // The items of several runs' debts, each as many times as the run that shows it most often shows it.
  def most(debts: Vector[Vector[String]]): Vector[String] =
    debts.flatMap(d => Multiset.counts(d).toVector).groupMapReduce(_._1)(_._2)(_ max _).toVector.sortBy(_._1).flatMap((k, n) => Vector.fill(n)(k))

  // A line's runs (each its name, `compile run 1`, and its observation) held to the line's record of `shape`, and the
  // record's step the mode asks for, taken once every run held and every other check of the verb passed (`checked`):
  // a step of the record (--record) the most each run showed of each item, a baseline (--baseline) or a migration
  // (--migrate) the first run's debt with every run within it and its coverage once each, published with its
  // evidence (the first run's transcript, normalised) and the files staged with it (the generated inputs' digests),
  // all as one (`Records.publish`), or nothing written. Whether the line passes.
  def hold(project: String, what: String, here: Path, shape: Shape, runs: Vector[(String, Observation)], evidence: => String, mode: Mode,
      binary: String, checked: Boolean = true, staged: Vector[(String, Array[Byte])] = Vector.empty): Boolean =
    def publish(r: Record, step: String, withEvidence: Boolean): Boolean =
      val problems = Records.publish(here, shape, r, Option.when(withEvidence)(evidence), staged)
      for p <- problems do println(if p.startsWith("  ") then p else s"FAIL $project: $what: the ${r.provenance.kind} is refused: $p")
      if problems.isEmpty then println(s"$project: $what: $step ${shape.counted(r.debt.size)} in bench/$project/${shape.file}")
      problems.isEmpty
    mode match
      case Mode.Baseline | Mode.Migrate(_) =>
        val (kind, reason) = mode match
          case Mode.Migrate(r) => ("migrate", r)
          case _ => ("baseline", "")
        val first = runs.head._2
        val twice = first.scope.distinct.filter(u => first.scope.count(_ == u) > 1).map(u => s"$u: covered ${first.scope.count(_ == u)} times")
        val problems = runs.flatMap((name, o) => o.problems.map(p => s"$name: $p")) ++ twice.map(t => s"${runs.head._1}: $t")
        if problems.nonEmpty || !checked then
          problems.foreach(p => println(s"FAIL $project: $what: $p"))
          println(s"FAIL $project: $what: ${if problems.nonEmpty then "a run with these problems is no evidence" else "the verb's own checks failed"}: nothing recorded")
          false
        else
          val proposed = Record(provenance(kind, reason, here, shape, binary, staged.toMap), first.scope.sorted, first.debt.sorted)
          // Every run within the record it would publish (a second run, a regenerated compile).
          val within = runs.map((name, o) => Ratchet.report(project, name, proposed, Ratchet.judge(proposed, o), shape)).forall(identity)
          if !within then
            println(s"FAIL $project: $what: the runs are not all within the $kind the first makes: nothing recorded")
            false
          else publish(proposed, if kind == "baseline" then "recorded a baseline of" else s"recorded a migration ($reason) to", withEvidence = true)
      case Mode.Judge | Mode.Record =>
        Records.read(here, shape) match
          case None =>
            println(s"FAIL $project: $what: no record bench/$project/${shape.file}: master's binary records the first (--baseline)")
            false
          case Some(r) if r.provenance.inputs != Records.inputsOf(here, shape) =>
            println(s"FAIL $project: $what: bench/$project/${shape.file} holds for other inputs than the pin's and the generated inputs' (${r.provenance.inputs.take(12)}): a new pin or new generated inputs take a migration (--migrate <reason>)")
            false
          case Some(r) =>
            val verdicts = runs.map((name, o) => Ratchet.judge(r, o))
            val passed = runs.zip(verdicts).map { case ((name, _), v) => Ratchet.report(project, name, r, v, shape) }.forall(identity)
            if mode == Mode.Record then
              if !passed || !checked then
                println(s"FAIL $project: $what: a failed run records nothing")
                false
              else
                val debt = most(runs.map(_._2.debt))
                if debt == r.debt then { println(s"$project: $what: the record stands"); true }
                else publish(Record(provenance("record", "", here, shape, binary, Map.empty), r.scope, debt), "the record stepped to", withEvidence = false)
            else passed
