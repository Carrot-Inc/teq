// What the scripts of teq's public reference projects share (bench/lila/lila.scala, bench/airstream/airstream.scala,
// bench/laminar/laminar.scala): the inventory of a test run, read from an oracle's JUnit reports or from a
// `teq test --verbose` transcript and held to the pinned one (tests.json), and the peak memory of teq's processes
// while a command runs.
import java.nio.file.{Files, Path}

object Reference:
  // The peak memory of a command's processes while it runs: sampled each second, the resident set sizes summed over
  // the processes the script starts (the children of its own process, the sampler's parent, the sampler apart) and
  // the processes whose command line names the binary (the client, the daemon and its residents, which outlive the
  // client), each with all its descendants (the test runners: node, the JVM), the sampler's own processes apart. The
  // sampler ends with the body, or after `seconds`.
  def sampled[A](bin: String, out: Path, seconds: Double)(body: => A): (A, Option[Long]) =
    val file = out.resolve("peak.kb")
    Files.deleteIfExists(file)
    val tree = "ps -eo pid=,ppid=,rss=,args= | awk -v b=\"$1\" -v root=\"$PPID\" -v me=\"$$\" '" +
      "{ pid = $1; parent[pid] = $2; rss[pid] = $3; $1 = $2 = $3 = \"\"; args[pid] = $0 } " +
      "END { mine[me] = 1; grown = 1; while (grown) { grown = 0; for (p in parent) if (!(p in mine) && (parent[p] in mine)) { mine[p] = 1; grown = 1 } } " +
      "for (p in parent) if (!(p in mine) && p != root && (parent[p] == root || index(args[p], b))) seen[p] = 1; " +
      "grown = 1; while (grown) { grown = 0; for (p in parent) if (!(p in seen) && !(p in mine) && (parent[p] in seen)) { seen[p] = 1; grown = 1 } } " +
      "for (p in seen) s += rss[p]; print s + 0 }'"
    val script = s"peak=0; while :; do n=$$($tree); [ \"$$n\" -gt \"$$peak\" ] && peak=$$n && echo $$peak > \"$$2\"; sleep 1; done"
    val sampler = Sh("sh", "-c", script, "sampler", bin, file.toString).timeout(seconds).spawn()
    val a = try body finally sampler.kill()
    (a, if Files.isRegularFile(file) then Files.readString(file).trim.toLongOption else None)

  def gigabytes(peak: Option[Long]): String = peak.fold("unmeasured")(k => f"${k / 1048576.0}%.2f GB")

// The suites and cases of a test run: each suite by its project and its name (a suite of one name may run in two
// projects), with its cases, each with the status sbt's test interface gave it (`sbt.testing.Status`: `Success`,
// `Error`, `Failure`, `Skipped`, `Ignored`, `Canceled`, `Pending`), a case as many times as the run reported it.
final case class Suite(project: String, cases: Vector[(String, String)])
final case class Tests(suites: Map[(String, String), Suite]):
  def cases: Int = suites.values.map(_.cases.size).sum
  // How many cases of each status, in the interface's order.
  def counts: String =
    val all = suites.values.flatMap(_.cases.map(_._2)).groupBy(identity).view.mapValues(_.size).toMap
    Tests.statuses.filter(all.contains).map(s => s"${all(s)} ${s.toLowerCase}").mkString(", ")

object Tests:
  // A test configuration's summary line: `<project>/test: N suites, P passed, F failed in ...`.
  val summaryLine = "^([^ ]+)/test: ([0-9]+) suites?, ([0-9]+) passed, ([0-9]+) failed".r

  // sbt's statuses, `sbt.testing.Status` in its order; tests.json keeps a case that passed under `cases` and one of
  // another status under the status's name in lower case.
  val statuses = Vector("Success", "Error", "Failure", "Skipped", "Ignored", "Canceled", "Pending")
  def key(status: String): String = if status == "Success" then "cases" else status.toLowerCase
  def failed(status: String): Boolean = status == "Failure" || status == "Error"

  // tests.json: each suite under its name, an object with its project and its cases by status, or an array of them
  // where the name runs in more than one project.
  def read(file: Path): Tests =
    if !Files.isRegularFile(file) then Tests(Map.empty)
    else
      val j = Json.parse(Files.readString(file))
      def suite(s: Json) = Suite(s("project").str, statuses.flatMap(st => s.get(key(st)).toVector.flatMap(_.elements).map(c => c.str -> st)))
      Tests(j("suites").members.flatMap { (name, v) =>
        val all = v match
          case Json.Arr(items) => items
          case one => Vector(one)
        all.map(s => (s("project").str, name) -> suite(s))
      }.toMap)

  // Every suite with its project and its cases' names, sorted (a case reported twice twice), the counts first: what
  // passed under `cases`, a case of another status under the status's key; a name of several projects an array.
  def write(t: Tests): String =
    def one(s: Suite) =
      def named(status: String) = Json.Arr(s.cases.filter(_._2 == status).map(_._1).sorted.map(Json.str))
      val others = statuses.drop(1).filter(st => s.cases.exists(_._2 == st)).map(st => key(st) -> named(st))
      Json.Obj(Vector("project" -> Json.str(s.project), "cases" -> named("Success")) ++ others)
    val suites = t.suites.toVector.groupBy(_._1._2).toVector.sortBy(_._1).map { (name, all) =>
      val sorted = all.sortBy(_._1._1).map(_._2)
      name -> (if sorted.size == 1 then one(sorted.head) else Json.Arr(sorted.map(one)))
    }
    Json.write(Json.obj("suites_count" -> Json.num(t.suites.size), "cases_count" -> Json.num(t.cases), "suites" -> Json.Obj(suites)), indent = 1) + "\n"

  private val suiteLine = "<testsuite [^>]*\\sname=\"([^\"]*)\"".r
  private val caseTag = "(?s)<testcase\\s([^>]*?)(/>|>(.*?)</testcase>)".r
  private val nameAttr = "\\sname=\"([^\"]*)\"".r

  def unescape(s: String): String =
    "&(#x[0-9a-fA-F]+|#[0-9]+|amp|lt|gt|quot|apos);".r.replaceAllIn(s, m =>
      java.util.regex.Matcher.quoteReplacement(m.group(1) match
        case "amp" => "&"
        case "lt" => "<"
        case "gt" => ">"
        case "quot" => "\""
        case "apos" => "'"
        case n if n.startsWith("#x") => codePoint(Integer.parseInt(n.drop(2), 16))
        case n => codePoint(n.drop(1).toInt)))

  private def codePoint(cp: Int): String =
    if cp < 0x10000 then cp.toChar.toString
    else s"${(0xd800 + ((cp - 0x10000) >> 10)).toChar}${(0xdc00 + ((cp - 0x10000) & 0x3ff)).toChar}"

  // sbt's JUnit reports under the checkout's target directories, `<project>/test-reports/TEST-<suite>.xml`, the
  // project the directory's name (sbt 2's layout); `projectOf` names it from the report's path otherwise.
  def fromReports(dir: Path, projectOf: Path => String = _.getParent.getParent.getFileName.toString): Tests =
    val reports = Files.walk(dir.resolve("target")).toArray.toList.map(_.asInstanceOf[Path])
      .filter(p => p.getFileName.toString.startsWith("TEST-") && p.getFileName.toString.endsWith(".xml") && p.getParent.getFileName.toString == "test-reports")
    Tests(reports.map { p =>
      val text = Files.readString(p)
      val name = suiteLine.findFirstMatchIn(text).map(m => unescape(m.group(1))).getOrElse(p.getFileName.toString.stripPrefix("TEST-").stripSuffix(".xml"))
      val cases = caseTag.findAllMatchIn(text).map { m =>
        val body = Option(m.group(3)).getOrElse("")
        // sbt's JUnit reports keep a failure and an error apart and write any of skipped, ignored and pending as
        // `<skipped/>` (a canceled test as a pass): lila's oracle, whose inventory is of passes.
        val outcome = if body.contains("<failure") then "Failure" else if body.contains("<error") then "Error" else if body.contains("<skipped") then "Skipped" else "Success"
        unescape(nameAttr.findFirstMatchIn(" " + m.group(1)).map(_.group(1)).getOrElse("?")) -> outcome
      }.toVector
      (projectOf(p), name) -> Suite(projectOf(p), cases)
    }.toMap)

  // A `teq test --verbose` transcript as the run's inventory (lila's): each project's section ends with its summary
  // line (`<project>/test: N suites, ..`), and holds a line per test the run reports, its status (`[success]`,
  // `[skipped]`, `[ignored]`, `[canceled]`, `[pending]`) or `[fail]` (a failure or an error, which the transcript
  // does not tell apart) before the suite and the test (`task::events::report`). A test is matched to the
  // inventory's suite whose name prefixes it, another suite is the line's first word.
  def fromTranscript(transcript: String, expected: Tests): Tests =
    val known = expected.suites.keys.map(_._2).toVector.distinct.sortBy(-_.length)
    val casesOf = expected.suites.toVector.groupMap(_._1._2)(_._2.cases.map(_._1)).view.mapValues(_.flatten).toMap
    val outcome = "^\\[(success|skipped|ignored|canceled|pending|fail)\\] (.*)$".r
    val suites = scala.collection.mutable.LinkedHashMap.empty[(String, String), Vector[(String, String)]]
    var section = Vector.empty[(String, String, String)]
    for line <- transcript.linesIterator do
      summaryLine.findFirstMatchIn(line) match
        case Some(m) =>
          val project = m.group(1).stripSuffix("/test")
          for (suite, test, result) <- section do
            suites((project, suite)) = suites.getOrElse((project, suite), Vector.empty) :+ (test -> result)
          section = Vector.empty
        case None =>
          line match
            case outcome(kind, named) if !named.startsWith("  ") && !named.startsWith(" at ") =>
              val suite = known.find(n => named == n || named.startsWith(n + " ")).getOrElse(named.takeWhile(_ != ' '))
              val rest = named.drop(suite.length).stripPrefix(" ")
              // A failure's line goes on with its throwable's first line after `: `.
              val test = if kind == "fail" then casesOf.get(suite).flatMap(_.filter(c => rest == c || rest.startsWith(c + ": ")).maxByOption(_.length)).getOrElse(rest.takeWhile(_ != ':')) else rest
              val result = if kind == "fail" then "Failure" else kind.capitalize
              section = section :+ (suite, test, result)
            case _ => ()
    Tests(suites.toMap.map((key, cases) => key -> Suite(key._1, cases)))

  // The events of a run as `teq test --events` writes them (`task::events::Event::line`) and the oracle's listener
  // (bench/reference/oracle/OracleEvents.scala): a line each, the project, the event's name (the suite), its selector,
  // its status and the first line of what it threw, tab-separated. A case is the test a `test <name>` selector names,
  // another selector as it is written (`nested <suite> <test>`, `suite`), in the suite of its own event's project.
  def fromEvents(text: String): Tests =
    val suites = scala.collection.mutable.LinkedHashMap.empty[(String, String), Vector[(String, String)]]
    for line <- text.linesIterator if line.nonEmpty do
      line.split("\t", -1) match
        case Array(project, suite, selector, status, _*) =>
          val test = if selector.startsWith("test ") then selector.drop(5) else selector
          suites((project, suite)) = suites.getOrElse((project, suite), Vector.empty) :+ (test -> status)
        case _ => ()
    Tests(suites.toMap.map((key, cases) => key -> Suite(key._1, cases)))

  // What the expected inventory and the run's differ in: each suite of either by its project and its name, and each
  // case with its statuses as many times as each side reports it (a case run twice, a case ignored on one side and
  // pending on the other, a case of another project's suite of the name). A suite without a case (a base class
  // sbt's JUnit reports listed) is compared by neither side.
  def compare(expected: Tests, found: Tests): Vector[String] =
    val out = Vector.newBuilder[String]
    if expected.suites.isEmpty then out += "tests.json holds no suite"
    if found.suites.isEmpty then out += "the run discovered no suite"
    def times(statuses: Vector[String]): String =
      statuses.groupBy(identity).toVector.sortBy(_._1).map((st, all) => if all.size == 1 then st else s"$st x${all.size}").mkString(", ")
    for ((project, name), s) <- expected.suites.toVector.sortBy(_._1) if s.cases.nonEmpty do
      found.suites.get((project, name)) match
        case None =>
          val elsewhere = found.suites.keys.filter(_._2 == name).map(_._1).toVector.sorted
          if elsewhere.isEmpty then out += s"suite $name ($project) did not run"
          else out += s"suite $name ran in project ${elsewhere.mkString(", ")}, tests.json's is $project"
        case Some(f) =>
          val want = s.cases.groupMap(_._1)(_._2)
          val got = f.cases.groupMap(_._1)(_._2)
          for c <- (s.cases.map(_._1) ++ f.cases.map(_._1)).distinct do
            val (w, g) = (want.getOrElse(c, Vector.empty).sorted, got.getOrElse(c, Vector.empty).sorted)
            if w != g then
              if g.isEmpty then out += s"$name: case \"$c\" did not run"
              else if w.isEmpty then out += s"$name: case \"$c\" (${times(g)}) is not in tests.json"
              else out += s"$name: case \"$c\" ${times(g)}, tests.json's ${times(w)}"
    for (project, name) <- found.suites.keys.toVector.sorted if !expected.suites.contains((project, name)) && found.suites((project, name)).cases.nonEmpty do
      out += s"suite $name ($project) is not in tests.json"
    out.result()
