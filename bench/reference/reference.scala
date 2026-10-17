// What the scripts of teq's public reference projects share (bench/lila/lila.scala, bench/airstream/airstream.scala,
// bench/laminar/laminar.scala): the inventory of a test run, read from an oracle's JUnit reports or from the events
// `teq test --events` writes and held to the pinned one (tests.json), and the peak memory of the processes one
// invocation starts while it runs.
import java.nio.file.{Files, Path, Paths}
import java.security.MessageDigest

object Reference:
  // The peak memory of the process trees one invocation starts, sampled each second: the processes whose environment
  // carries the invocation's token (`TEQ_MEASURE`, which `body` gives the commands it runs, the client hands to the
  // daemon it starts and the daemon to its residents and test runners, node's and the JVM's), with all their
  // descendants, their resident set sizes summed. Nothing another invocation or session started counts, a process
  // that names the binary or the test runner's class among them. A verb stops teq's daemon before each run, so that
  // the daemon a run uses is the one it started. The sampler writes 0 at its start, so that a run whose processes it
  // never saw reads as measured to nothing, and ends with the body, or after `seconds`.
  def sampled[A](out: Path, seconds: Double)(body: ((String, String)) => A): (A, Option[Long]) =
    val file = out.resolve("peak.kb")
    Files.deleteIfExists(file)
    val token = java.util.UUID.randomUUID.toString
    val roots = "grep -lz \"^TEQ_MEASURE=$1\\$\" /proc/[0-9]*/environ 2>/dev/null | cut -d/ -f3 | tr '\\n' ' '"
    val tree = "ps -eo pid=,ppid=,rss= | awk -v r=\"$r\" 'BEGIN { n = split(r, rs, \" \"); for (k = 1; k <= n; k++) mine[rs[k]] = 1 } " +
      "{ pid[NR] = $1; ppid[NR] = $2; rss[NR] = $3; root[NR] = ($1 in mine) } " +
      "END { do { more = 0; for (i = 1; i <= NR; i++) if (!root[i]) for (j = 1; j <= NR; j++) if (root[j] && ppid[i] == pid[j]) { root[i] = 1; more = 1; break } } while (more); " +
      "for (i = 1; i <= NR; i++) if (root[i]) s += rss[i]; print s + 0 }'"
    val script = s"peak=0; echo 0 > \"$$2\"; while :; do r=$$($roots); n=$$($tree); [ \"$$n\" -gt \"$$peak\" ] && peak=$$n && echo $$peak > \"$$2\"; sleep 1; done"
    val sampler = Sh("sh", "-c", script, "sampler", token, file.toString).timeout(seconds).spawn()
    val a = try body("TEQ_MEASURE" -> token) finally sampler.kill()
    (a, if Files.isRegularFile(file) then Files.readString(file).trim.toLongOption else None)

  def gigabytes(peak: Option[Long]): String = peak.fold("unmeasured")(k => f"${k / 1048576.0}%.2f GB")

// The suites and cases of a test run: each suite by its project and its name (a suite of one name may run in two
// projects), with its cases, each with the status sbt's test interface gave it (`sbt.testing.Status`: `Success`,
// `Error`, `Failure`, `Skipped`, `Ignored`, `Canceled`, `Pending`), a case as many times as the run reported it.
final case class Suite(project: String, cases: Vector[(String, String)])
final case class Tests(suites: Map[(String, String), Suite]):
  def cases: Int = suites.values.map(_.cases.size).sum
  // Each case with its status, as the deviations take them (bench/reference/records.scala), and the suites that hold
  // a case (a suite of none, a base class sbt's JUnit reports listed, is compared by neither side).
  def items: Vector[(TestCase, String)] =
    suites.toVector.sortBy(_._1).flatMap((key, s) => s.cases.map((c, st) => TestCase(key._1, key._2, c) -> st))
  def heldSuites: Vector[(String, String)] = suites.toVector.filter(_._2.cases.nonEmpty).map(_._1).sorted
  // How many cases of each status, in the interface's order.
  def counts: String =
    val all = suites.values.flatMap(_.cases.map(_._2)).groupBy(identity).view.mapValues(_.size).toMap
    Tests.statuses.filter(all.contains).map(s => s"${all(s)} ${s.toLowerCase}").mkString(", ")

object Tests:
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

// sbt-teq for a reference project's export: TEQ_PLUGIN_VERSION's where it names one (published by the caller), else
// the head's own, from integrations/sbt, published on both sbt lines (`^publish`: sbt-teq_sbt2_3 and
// sbt-teq_2.12_1.0) under a SNAPSHOT named by the digest of what the plugin is built from, into a Maven repository of
// its own beside the line's directory of logs and caches, once for each digest: an export takes the plugin of the
// sources under test, nothing reaches ~/.ivy2/local and no release's version is published (docs/DEVELOPING.md, "Building
// and testing"). The overlay's plugins.sbt adds the repository the property `teq.plugin.repository` names. The
// version and the repository, None for TEQ_PLUGIN_VERSION's.
object SbtPlugin:
  private def sha256(bytes: Array[Byte]): String = MessageDigest.getInstance("SHA-256").digest(bytes).map(b => f"${b & 0xff}%02x").mkString

  // What the plugin is built from: its build and sources (not its fixtures, checks and products), the launchers it
  // bakes in and the compiler's version Cargo.toml names.
  def sources: Vector[Path] =
    val sbt = Paths.get("integrations/sbt")
    val kept = Vector("build.sbt", "plugin-version.txt", "project", "src").map(sbt.resolve) ++ Vector(Paths.get("tools/launcher"), Paths.get("Cargo.toml"))
    kept.filter(Files.exists(_)).flatMap(p => Files.walk(p).toArray.toVector.map(_.asInstanceOf[Path]))
      .filter(Files.isRegularFile(_)).filterNot(p => p.toString.split('/').exists(s => s == "target" || s == ".bsp")).sortBy(_.toString)

  def digest: String = sha256(sources.map(f => s"$f ${sha256(Files.readAllBytes(f))}\n").mkString.getBytes(java.nio.charset.StandardCharsets.UTF_8))

  // What an export that failed for want of its plugin lacked, in its own words, from sbt's log: the coordinates it could
  // not download (sbt 1's `build.teq:sbt-teq;sbtVersion=1.0;scalaVersion=2.12:<version>`, sbt 2's
  // `build.teq:sbt-teq_sbt2_3:<version>`), and where TEQ_PLUGIN_VERSION named it, that it names a plugin the machine has
  // not published for that sbt.
  def missing(log: Path): Option[String] =
    val text = try Files.readString(log) catch case _: java.io.IOException => ""
    "Error downloading build\\.teq:(sbt-teq[^ ;:]*)(?:;sbtVersion=([^;]+);scalaVersion=([^:]+))?:(\\S+)".r.findFirstMatchIn(text).map { m =>
      val line = Option(m.group(2)).fold(if m.group(1).contains("sbt2") then "sbt 2" else m.group(1))(v => s"sbt $v (Scala ${m.group(3)})")
      val named = Option(System.getenv("TEQ_PLUGIN_VERSION")).filter(_ == m.group(4))
      s"sbt-teq ${m.group(4)} for $line is in no repository sbt reads" +
        named.fold("")(_ => s": TEQ_PLUGIN_VERSION names it, a plugin this machine has not published for $line; unset TEQ_PLUGIN_VERSION, and the export publishes the head's")
    }

  def forExport(out: Path, jdk: Path): (String, Option[String]) =
    Option(System.getenv("TEQ_PLUGIN_VERSION")).filter(_.nonEmpty) match
      case Some(v) => (v, None)
      case None =>
        val d = digest.take(12)
        val version = s"${Files.readString(Paths.get("integrations/sbt/plugin-version.txt")).trim}-reference-$d-SNAPSHOT"
        val repo = out.toAbsolutePath.normalize.getParent.resolve(s"sbt-teq-$d")
        if !Files.isRegularFile(repo.resolve("published")) then
          val staged = Files.createTempDirectory(out.toAbsolutePath.normalize.getParent, s"sbt-teq-$d.")
          val log = out.resolve("plugin.log")
          Log.stamped(s"sbt-teq: publishing integrations/sbt as $version on both sbt lines into $repo, its log $log")
          val r = Sh("sbt", "-java-home", jdk.toString, "--server", "--batch", "-no-colors", "-Dsbt.supershell=false", "-Dsbt.server.autostart=false",
            s"set version := \"$version\"", s"set publishTo := Some(\"teq-reference\" at \"file://$staged\")", "^publish")
            .in(Paths.get("integrations/sbt").toAbsolutePath).withEnv("JAVA_HOME" -> jdk.toString, "PATH" -> s"$jdk/bin:${System.getenv("PATH")}").without("SBT_OPTS")
            .stdout(log).mergeErr.timeout(900).check(false).run()
          val jars = Vector("sbt-teq_sbt2_3", "sbt-teq_2.12_1.0").map(a => staged.resolve(s"build/teq/$a/$version/$a-$version.jar"))
          if r.status != 0 || !jars.forall(Files.isRegularFile(_)) then
            println(Files.readString(log).linesIterator.toVector.takeRight(20).mkString("\n"))
            Sh.remove(staged)
            Log.die(s"sbt-teq: the publication of integrations/sbt exits ${r.status}${if r.status == 0 then ", without both lines' jars" else s": ${Log.cause(log)}"} (its log $log)")
          Files.writeString(staged.resolve("published"), s"$version\n")
          try Files.move(staged, repo, java.nio.file.StandardCopyOption.ATOMIC_MOVE)
          catch case _: java.io.IOException => Sh.remove(staged) // another export published the same digest meanwhile
        (version, Some(s"file://$repo"))
