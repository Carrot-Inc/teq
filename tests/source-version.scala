//> using file ../tools/script
// The source version against scalac's (src/source_version.rs): each run of tests/source-version/runs.txt, a probe
// of tests/source-version/probes with scalac's options, is checked by teq with the flags they map to (or run, for an
// `interp` run) and its exit and diagnostics, normalized (`Norm`: one block per diagnostic, its severity, place and
// whole message, a migration warning apart from an ordinary one; a run's program output after), are compared with
// the capture of the scalac its line holds teq to, 3.8.4 unless the line names 3.10.0. Both scalacs' captures are
// pinned apart, under tests/source-version/expected/<tag>/<run>.txt, and are never one another's stand-in.
// tests/source-version/pending.txt lists the runs teq does not give as scalac yet, each failing the suite once it
// does; tests/source-version/wording.txt the runs compared without their messages, whose syntax errors teq words
// as its own (docs/internal/DIFFERENCES.md, "The parser's recovery": *wording*), their places and severities
// held. Run by tests/run_errors.sh; the last line gives the counts.
//   ./teq interp tests/source-version.scala [-- --teq <binary>] [--only <run>...]
//   ./teq interp tests/source-version.scala -- --capture 3.8.4|3.10.0 [--jobs n] [--only <run>...]
// The capture needs scala-cli and a JDK; each run is bounded to 5 minutes, scalac's whole output kept under
// out/source-version-capture/<tag>/. 3.10.0 runs over a coursier cache of its own (COURSIER_CACHE, by default
// ~/.cache/coursier-310), so that its scala-library never becomes the newest 3.x of the cache teq's JVM builds
// take theirs from.
import java.nio.file.{Files, Path, Paths}

final case class Run(name: String, probe: String, options: Vector[String], interp: Boolean, holds: String)

object Runs:
  val tags = Vector("3.8.4", "3.10.0")

  // `[interp] <probe> [<options>] [=> <tag>]`.
  def parse(line: String): Run =
    val (body, holds) = line.split("=>", 2) match
      case Array(b, t) => (b.trim, t.trim)
      case Array(b) => (b.trim, "3.8.4")
    if !tags.contains(holds) then Log.die(s"runs.txt: no scalac $holds: $line")
    val words = body.split("\\s+").toVector.filter(_.nonEmpty)
    val interp = words.headOption.contains("interp")
    val rest = if interp then words.tail else words
    val probe = rest.head
    val options = rest.tail
    val shown = options.map(_.stripPrefix("-source:")).mkString("_").replaceAll("[:=/]", "_")
    Run(if options.isEmpty then probe else s"$probe@$shown", probe, options, interp, holds)

  def read(path: Path): Vector[Run] =
    Files.readAllLines(path).toArray.toVector.map(_.toString.trim).filter(l => l.nonEmpty && !l.startsWith("#")).map(parse)

  // teq's flags for scalac's options.
  def teqFlags(options: Vector[String]): Vector[String] =
    def go(rest: List[String]): List[String] = rest match
      case "-source" :: v :: tail => "--source" :: v :: go(tail)
      case o :: tail if o.startsWith("-source:") => "--source" :: o.stripPrefix("-source:") :: go(tail)
      case o :: tail if o.startsWith("-language:") => "--language" :: o.stripPrefix("-language:") :: go(tail)
      case o :: tail if o.startsWith("-Wconf:") => "--wconf" :: o.stripPrefix("-Wconf:") :: go(tail)
      case "-Werror" :: tail => "--werror" :: go(tail)
      case "-feature" :: tail => "--feature" :: go(tail)
      case "-deprecation" :: tail => "--deprecation" :: go(tail)
      case o :: _ => Log.die(s"runs.txt: no teq flag for $o")
      case Nil => Nil
    go(options.toList).toVector

// The comparison's form: `exit <n>`, then each diagnostic as `<severity> <line>:<col>` (or `<severity>` alone for one
// of no place) with its message's lines under it, indented, in place order (teq presents its diagnostics by place,
// scalac by the phase that reports them, docs/internal/DIFFERENCES.md), then the program's output as `out
// <line>` and an uncaught exception's first line as `exception <line>`. scalac's columns are read from the caret line,
// 1-based, teq's from its place; scalac's `longer explanation` trailer, scala-cli's own lines, the summaries and
// `-Werror`'s "No warnings can be incurred" (teq's in its own words, its exit the same) are left out. teq names its flags where scalac names its options (`--source` for `-source`), and a setting teq refuses
// ends it with status 2, a mistake of the command line, where scalac's driver ends with 1: compared as 1.
object Norm:
  private val Box = """^-- (?:\[E\d+\] )?(?:[A-Za-z]+ )*?(Migration Warning|Warning|Error|Info): (.*?):(\d+):(\d+) -*\s*$""".r
  private val BareBox = """^-- (?:\[E\d+\] )?(?:[A-Za-z]+ )*?(Migration Warning|Warning|Error|Info): -*\s*$""".r
  private val Frame = """^(\d+ |\s+)\|(.*)$""".r
  private val Caret = """^\s*\|(\s*)\^+\s*$""".r
  private val Chatter = Vector("Compiling project", "Compiled project", "Error compiling project", "Compilation failed",
    "Downloading", "Downloaded", "Starting compilation server", "  scalac -help  gives more information", "Exception in thread")
  private val Summary = """^\d+ (warning|error)s? found$""".r

  // The diagnostics' blocks, each a head and its message's lines, in place order: the unplaced first, as scalac's.
  def sorted(lines: Vector[String]): Vector[String] =
    val blocks = Vector.newBuilder[Vector[String]]
    var cur = Vector.empty[String]
    for l <- lines do
      if l.startsWith("  ") then cur = cur :+ l
      else
        if cur.nonEmpty then blocks += cur
        cur = Vector(l)
    if cur.nonEmpty then blocks += cur
    def key(b: Vector[String]): (Int, Int) =
      val place = b.head.split(' ').last
      if place.contains(':') then (place.takeWhile(_ != ':').toIntOption.getOrElse(0), place.dropWhile(_ != ':').drop(1).toIntOption.getOrElse(0)) else (0, 0)
    blocks.result().zipWithIndex.sortBy { case (b, i) => (key(b), i) }.flatMap(_._1)

  private def severity(level: String): String = level match
    case "Migration Warning" => "migration warning"
    case l => l.toLowerCase

  // scalac's stderr (scala-cli's own lines among them) and the program's stdout.
  def scalac(exit: Int, err: String, out: String, file: String): Vector[String] =
    val lines = err.replace("\r", "").split("\n", -1).toVector
    val found = Vector.newBuilder[String]
    var i = 0
    while i < lines.length do
      val line = lines(i)
      val placed = Box.findFirstMatchIn(line).filter(_.group(2).endsWith(file))
      val bare = if placed.isEmpty then BareBox.findFirstMatchIn(line) else None
      if placed.isDefined || bare.isDefined then
        var j = i + 1
        var col: Option[Int] = None
        val msg = Vector.newBuilder[String]
        var inMessage = false
        while j < lines.length && Frame.matches(lines(j)) do
          lines(j) match
            case Caret(spaces) if col.isEmpty && !inMessage => col = Some(spaces.length + 1)
            case l @ Frame(gutter, text) =>
              // The source lines carry their number in the gutter; the message's lines, after the caret, do not.
              if gutter.trim.isEmpty && (col.isDefined || placed.isEmpty) then
                inMessage = true
                msg += text
          j += 1
        var m = msg.result()
        // The trailer and the blank line before it.
        if m.lastOption.exists(_.trim == "longer explanation available when compiling with `-explain`") then m = m.dropRight(1)
        while m.lastOption.exists(_.trim.isEmpty) do m = m.dropRight(1)
        val head = placed match
          case Some(p) => s"${severity(p.group(1))} ${p.group(3)}:${col.getOrElse(0)}"
          case None => severity(bare.get.group(1))
        // `-Werror`'s refusal, which teq gives in its own words and its exit carries (tests/warnings/policy.py).
        if !(placed.isEmpty && m.map(_.trim) == Vector("No warnings can be incurred under -Werror")) then
          found += head
          m.foreach(l => found += "  " + l.trim)
        i = j
      else
        if line.trim.nonEmpty && !Chatter.exists(line.startsWith) && !Summary.matches(line.trim) && !line.startsWith("\tat ") && !line.startsWith("-- ")
          && line.trim != "No warnings can be incurred under -Werror" then
          found += "error"
          found += "  " + line.trim
        i += 1
    Vector(s"exit $exit") ++ sorted(found.result()) ++ output(out) ++ exception(err)

  private def output(out: String): Vector[String] =
    out.replace("\r", "").split("\n").toVector.filter(l => l.nonEmpty && !Chatter.exists(l.startsWith)).map("out " + _)

  private def exception(err: String): Vector[String] =
    err.replace("\r", "").split("\n").toVector.find(_.startsWith("Exception in thread")).map(l => "exception " + l.replaceFirst("""^Exception in thread "main" """, "")).toVector

  private val Head = """^(.*?):(\d+):(\d+): (migration warning|warning|error|info): (.*)$""".r
  private val UnplacedHead = """^(migration warning|warning|error|info): (.*)$""".r
  private val TeqCaret = """^\s*\^+\s*$""".r
  private val Wording = Vector("--source" -> "-source", "--language" -> "-language")

  // teq's diagnostics (`path:line:col: severity: message`, the message's further lines, then the source line and
  // its caret) and, for an interpreted run, the program's output and an uncaught exception.
  def teq(exit: Int, err: String, out: String, file: String): Vector[String] =
    val lines = err.replace("\r", "").split("\n", -1).toVector
    val found = Vector.newBuilder[String]
    var errors = 0
    var placedAny = false
    var exceptionLine: Option[String] = None
    var i = 0
    def words(s: String) = Wording.foldLeft(s) { case (acc, (a, b)) => acc.replace(a, b) }
    while i < lines.length do
      val line = lines(i)
      val head = Head.findFirstMatchIn(line).filter(_.group(1).endsWith(file)).map(m => (s"${m.group(4)} ${m.group(2)}:${m.group(3)}", m.group(5), m.group(4)))
        .orElse(UnplacedHead.findFirstMatchIn(line).map(m => (m.group(1), m.group(2), m.group(1))))
      head match
        case Some((h, first, sev)) =>
          if sev == "error" then errors += 1
          if h.contains(':') then placedAny = true
          found += h
          // The message's lines run to the source line its caret follows, or to the next diagnostic.
          def ends(k: Int) = k >= lines.length || Head.findFirstMatchIn(lines(k)).isDefined || UnplacedHead.findFirstMatchIn(lines(k)).isDefined || Summary.matches(lines(k).trim)
          var c = i + 1
          while !ends(c) && !(c + 1 < lines.length && TeqCaret.matches(lines(c + 1)) && lines(c).startsWith("  ")) do c += 1
          val placed = !ends(c)
          val msg = (first +: lines.slice(i + 1, c)).reverse.dropWhile(_.trim.isEmpty).reverse
          msg.foreach(l => found += "  " + words(l.trim))
          i = if placed then c + 2 else c
        case None =>
          if line.startsWith("Exception in thread") || line.startsWith("uncaught exception") then exceptionLine = Some(line)
          i += 1
    val status = if exit == 2 && errors == 1 && !placedAny then 1 else exit
    Vector(s"exit $status") ++ sorted(found.result()) ++ output(out) ++ exceptionLine.toVector.map(l => "exception " + l.replaceFirst("""^Exception in thread "main" """, "").replaceFirst("^uncaught exception: ", ""))

@main def sourceVersion(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/source-version.scala [--teq <binary>] [--capture 3.8.4|3.10.0] [--jobs n] [--only <run>...]")
  val teq = args.teq("./target/release/teq")
  val capture = args.option("capture")
  val jobs = args.option("jobs").map(_.toInt).getOrElse(4)
  val only = scala.collection.mutable.ArrayBuffer.empty[String]
  var more = args.option("only")
  while more.isDefined do
    only += more.get
    more = args.option("only")
  args.exactly(0)
  val root = Paths.get("tests/source-version")
  if !Files.isDirectory(root) then Log.die("tests/source-version.scala runs from the repository's root")
  val all = Runs.read(root.resolve("runs.txt"))
  val dup = all.groupBy(_.name).collect { case (n, rs) if rs.length > 1 => n }
  if dup.nonEmpty then Log.die(s"runs.txt: runs named twice: ${dup.mkString(" ")}")
  val runs = if only.isEmpty then all else all.filter(r => only.contains(r.name))
  def probe(r: Run): Path = root.resolve("probes").resolve(r.probe + ".scala")
  for r <- runs if !Files.exists(probe(r)) do Log.die(s"runs.txt: no probe ${probe(r)}")
  def chomp(s: String): String = s.replaceAll("\n+$", "")

  capture match
    case Some(tag) =>
      if !Runs.tags.contains(tag) then args.fail(s"no scalac $tag")
      val expected = root.resolve("expected").resolve(tag)
      val logs = Paths.get("out/source-version-capture").resolve(tag)
      Files.createDirectories(expected)
      Files.createDirectories(logs)
      val scratch = Files.createTempDirectory("teq-source-version.")
      Script.atExit(Sh.remove(scratch))
      val cache = Option(System.getenv("COURSIER_CACHE_310")).getOrElse(System.getProperty("user.home") + "/.cache/coursier-310")
      var done = 0
      Sh.pool(jobs, runs) { r =>
        val dir = scratch.resolve(r.name.replaceAll("[^A-Za-z0-9._@-]", "_"))
        Files.createDirectories(dir)
        val file = dir.resolve(r.probe + ".scala")
        Files.copy(probe(r), file)
        val opts = ("-color:never" +: r.options).flatMap(o => Vector("-O", o))
        val cmd0 = Sh((Vector("scala-cli", if r.interp then "run" else "compile", "-S", tag, "--jvm", "system", "--server=false") ++ opts :+ file.toString)*)
          .timeout(300).check(false).in(dir)
        val cmd = if tag == "3.10.0" then cmd0.withEnv("COURSIER_CACHE" -> cache) else cmd0
        Sh.Then(cmd, res => {
          // A compile's stdout is scala-cli's and scalac's; a run's, the program's.
          val (err, out) = if r.interp then (res.errAll(), res.outAll()) else (res.errAll() + "\n" + res.outAll(), "")
          Files.writeString(logs.resolve(r.name + ".log"), s"exit ${res.status}\n--- stderr\n$err\n--- stdout\n$out")
          val norm = Norm.scalac(res.status, err, out, r.probe + ".scala")
          Files.writeString(expected.resolve(r.name + ".txt"), norm.mkString("\n") + "\n")
          done += 1
          Sh.End
        })
      }
      println(s"captured $done runs under scalac $tag")
    case None =>
      val pendingFile = root.resolve("pending.txt")
      val pending = if Files.exists(pendingFile) then
        Files.readAllLines(pendingFile).toArray.toVector.map(_.toString.trim).filter(l => l.nonEmpty && !l.startsWith("#")).map(_.split("\\s+")(0)).toSet
      else Set.empty[String]
      for p <- pending if !all.exists(_.name == p) do Log.die(s"pending.txt: no run $p")
      val wordingFile = root.resolve("wording.txt")
      val wording = if Files.exists(wordingFile) then
        Files.readAllLines(wordingFile).toArray.toVector.map(_.toString.trim).filter(l => l.nonEmpty && !l.startsWith("#")).map(_.split("\\s+")(0)).toSet
      else Set.empty[String]
      for p <- wording if !all.exists(_.name == p) do Log.die(s"wording.txt: no run $p")
      def placesOnly(text: String): String = text.split("\n").filterNot(_.startsWith("  ")).mkString("\n")
      val records = Paths.get("out/source-version")
      Files.createDirectories(records)
      var matched = 0
      var stillPending = 0
      var fail = false
      Sh.pool(jobs, runs) { r =>
        val flags = Runs.teqFlags(r.options)
        val cmd = Sh((Vector(teq) ++ (if r.interp then Vector("interp") else Vector("compiler", "check")) ++ Vector(probe(r).toString) ++ flags)*)
          .timeout(60).check(false)
        Sh.Then(cmd, res => {
          val err = res.errAll()
          val out = res.outAll()
          // teq writes its diagnostics to stdout for a check and to stderr for a run; both are read as diagnostics but
          // an interpreted run's stdout, which is the program's.
          val (diag, prog) = if r.interp then (err, out) else (out + err, "")
          Files.writeString(records.resolve(r.name + ".out"), s"exit ${res.status}\n$out\n$err")
          val got = Norm.teq(res.status, diag, prog, r.probe + ".scala").mkString("\n")
          val expectedFile = root.resolve("expected").resolve(r.holds).resolve(r.name + ".txt")
          if !Files.exists(expectedFile) then
            println(s"FAIL ${r.name}: no capture of scalac ${r.holds} ($expectedFile)")
            fail = true
          else
            val want0 = chomp(Files.readString(expectedFile))
            val (want, gotHeld) = if wording(r.name) then (placesOnly(want0), placesOnly(got)) else (want0, got)
            val same = gotHeld == want
            if same && pending(r.name) then
              println(s"FAIL ${r.name}: gives scalac ${r.holds}'s diagnostics now; take it off tests/source-version/pending.txt")
              fail = true
            else if same then matched += 1
            else if pending(r.name) then stillPending += 1
            else
              println(s"FAIL ${r.name} (${r.options.mkString(" ")}): teq's diagnostics or exit differ from scalac ${r.holds}'s (${probe(r)})")
              val a = records.resolve(r.name + ".want")
              val b = records.resolve(r.name + ".got")
              Files.writeString(a, want + "\n")
              Files.writeString(b, gotHeld + "\n")
              Sh("diff", a.toString, b.toString).check(false).run().lines.take(40).foreach(l => println("  " + l))
              fail = true
          Sh.End
        })
      }
      val summary = s"source version: $matched of ${runs.length} runs as scalac, $stillPending pending"
      println(if fail then s"$summary; failed" else summary)
      Script.exit(if fail then 1 else 0)
}
