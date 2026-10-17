// The verbs of teq's Scala.js reference projects (bench/airstream/airstream.scala, bench/laminar/laminar.scala),
// each an sbt 1 build of Scala.js with ScalaTest suites under jsdom: the pinned release checked out with its npm
// fixture, built and tested under scalac 3.8.4 by its own sbt build (the oracle), exported with sbt-teq through the
// overlay, then compiled and tested by teq against the oracle's results. The project's directory under bench/ holds:
//
//   pin.json           the upstream repository, tag and commit, the JDK, sbt and the Scala axis the gate takes, the
//                      build's root project's name, the projects teq compiles and tests, the npm fixture's digest and
//                      jsdom's version, the overlay's digests
//   package-lock.json  the npm fixture (jsdom), which the release does not commit: the checkout's, for sbt and teq
//   tests.json         the oracle's inventory: every suite with its project and its cases, by sbt's status
//   overlay/           plugins.sbt (sbt-teq by sbt's -addPluginSbtFile) and the generators teq runs in sbt's place
//
// Verbs, each `<verb> <checkout> <out>`:
//
// checkout  clones the repository into <checkout> (blob-filtered) or fetches the pin into an existing clone,
//           checks out pin.json's commit detached, refuses a tracked file changed, writes package-lock.json and
//           installs it (`npm ci`)
// oracle    the build's sbt as its CI runs it, the Scala 3 axis alone: `++<scala>! clean test` (sbt 1's `test` is the
//           full run) under the JDK pin.json names (--runs n, default 1) with the oracle's listener
//           (bench/reference/oracle, which records every event of the run), each run's wall time, peak memory and its
//           suites and cases against tests.json; --record rewrites tests.json, never from a run that executed nothing
//           or failed
// export    `sbt ++<scala>! teqExportAll` with the overlay (sbt-teq at TEQ_PLUGIN_VERSION, by default the head's
//           integrations/sbt published under a SNAPSHOT of its sources' digest into a repository beside <out>,
//           `SbtPlugin`), the lock copied to <out>; a lock recording a generator only sbt runs fails, and so does an
//           export that compiled the build
// compile   `teq compile` of pin.json's projects over the export (--teq <binary>, by default ./target/release/teq),
//           the daemon stopped, teq's outputs removed and the generators' outputs the lock names deleted before each
//           run (--runs n): each run's wall time and the peak memory of its processes (`Reference.sampled`), each run
//           held to errors.txt (bench/reference/records.scala: every configuration compiled once, the run complete, its
//           errors within the record's, a new one failing, one gone listed; --record writes a passing run's
//           improvement, --baseline the first record, --migrate <reason> one over new inputs); the generated
//           files back afterwards with the bytes the pin commits (`git status` of the checkout clean), the checkout
//           put back at the pin where they are not
// test      `teq test --verbose --events` of pin.json's projects: its suites and cases, each with its status and as
//           many times as the run reported it, against tests.json's (--runs n), and its exit
//
// Every verb checks that the checkout is at the pin with its npm fixture and that the overlay's files are the bytes
// pin.json records. <out> holds the logs and the lock and stays outside the repository. JAVA_HOME names the JDK (its
// major version must be pin.json's); sbt and node are the PATH's. Every command is bounded: REFERENCE_SBT_SECONDS
// (default 900) for an sbt run, REFERENCE_TEQ_SECONDS (default 1800) for teq's, 300 for git and npm.
import java.nio.file.{Files, Path, Paths}
import java.security.MessageDigest

// pin.json: the release, its toolchain and fixture, the overlay's digests.
final case class ScalaJsPin(json: Json):
  def repository: String = json("repository").str
  def tag: String = json("tag").str
  def commit: String = json("commit").str
  def java: Int = json("java").int
  def sbt: String = json("sbt").str
  def scala: String = json("scala").str
  def rootProject: String = json("rootProject").str
  // Whether the build names no root project, sbt's default root taking the checkout directory's name.
  def rootProjectByDirectory: Boolean = json.get("rootProjectByDirectory").exists(_ == Json.Bool(true))
  def projects: Vector[String] = json("projects").elements.map(_.str)
  def lockfile: String = json("npm")("package-lock.json").str
  def jsdom: String = json("npm")("jsdom").str
  def overlay: Map[String, String] = json("overlay").members.map((f, d) => f -> d.str).toMap

object ScalaJsPin:
  def sha256(bytes: Array[Byte]): String =
    MessageDigest.getInstance("SHA-256").digest(bytes).map(b => f"${b & 0xff}%02x").mkString

// One Scala.js reference project, by its name (its directory under bench/, `here`, where a check of the verbs puts
// a directory of its own) and the overlay's system properties' prefix (`teq.<name>.overlay`, `teq.<name>.scripts`,
// which overlay/plugins.sbt and the generators read).
final class ScalaJsReference(val name: String, val here: Path):
  def this(name: String) = this(name, Paths.get("bench", name))
  if !Files.isRegularFile(here.resolve("pin.json")) then Log.die(s"bench/$name/$name.scala runs from the repository's root")
  val pin: ScalaJsPin = ScalaJsPin(Json.parse(Files.readString(here.resolve("pin.json"))))

  def fail(message: String): Unit = println(s"FAIL $name: $message")
  def say(message: String): Unit = println(s"$name: $message")

  private def git(dir: Path, a: String*): Sh.Result = Sh(("git" +: "-C" +: dir.toString +: a)*).timeout(300).run()

  // The overlay's files are the bytes pin.json records, and no other file is in its directory; and the npm fixture.
  def checkFixtures(): Unit =
    val dir = here.resolve("overlay")
    val present = Files.list(dir).toArray.toList.map(_.asInstanceOf[Path].getFileName.toString).sorted
    if present != pin.overlay.keys.toList.sorted then
      Log.die(s"bench/$name/overlay holds ${present.mkString(", ")}, where pin.json records ${pin.overlay.keys.toList.sorted.mkString(", ")}")
    for (f, digest) <- pin.overlay do
      val actual = ScalaJsPin.sha256(Files.readAllBytes(dir.resolve(f)))
      if actual != digest then Log.die(s"bench/$name/overlay/$f is $actual, where pin.json records $digest: the pin, tests.json and the overlay move together")
    val lock = ScalaJsPin.sha256(Files.readAllBytes(here.resolve("package-lock.json")))
    if lock != pin.lockfile then Log.die(s"bench/$name/package-lock.json is $lock, where pin.json records ${pin.lockfile}")

  def checkout(dir: Path): Unit =
    if !Files.isDirectory(dir.resolve(".git")) then
      Files.createDirectories(dir.toAbsolutePath.getParent)
      Log.stamped(s"$name: cloning ${pin.repository} into $dir")
      Sh("git", "clone", "--filter=blob:none", "--no-checkout", pin.repository, dir.toString).timeout(300).run()
    val has = Sh("git", "-C", dir.toString, "cat-file", "-e", pin.commit + "^{commit}").timeout(60).check(false).run().ok
    if !has then
      Log.stamped(s"$name: fetching ${pin.tag} (${pin.commit})")
      git(dir, "fetch", "--filter=blob:none", "origin", "tag", pin.tag)
    git(dir, "checkout", "--quiet", "--detach", pin.commit)
    Files.copy(here.resolve("package-lock.json"), dir.resolve("package-lock.json"), java.nio.file.StandardCopyOption.REPLACE_EXISTING)
    Log.stamped(s"$name: installing the npm fixture (npm ci)")
    val npm = Sh("npm", "ci", "--no-audit", "--no-fund").in(dir).mergeErr.timeout(300).check(false).run()
    if !npm.ok then Log.die(s"$name: npm ci exits ${npm.status}: ${npm.out.linesIterator.toVector.takeRight(10).mkString("\n")}")
    atPin(dir)
    say(s"$dir at ${pin.tag} (${pin.commit}), its npm fixture installed")

  // sbt's id of a build's default root project, the directory's name made an id (`Project.normalizeProjectID`).
  def sbtProjectId(directory: String): String =
    val base = directory.toLowerCase.replaceAll("[^a-z0-9_]+", "-")
    if base.isEmpty then "root" else if !base.head.isLetter then "root-" + base else base

  // The checkout is where sbt's root project is pin.json's (a build that names none takes the directory's name), at
  // the pin, no tracked file changed, its package-lock.json the pin's and installed (jsdom's version the pin's).
  def atPin(dir: Path): Unit =
    if pin.rootProjectByDirectory && sbtProjectId(dir.getFileName.toString) != pin.rootProject then
      Log.die(s"$dir makes sbt's root project ${sbtProjectId(dir.getFileName.toString)}: the build names none, and sbt names it after the checkout's directory, where pin.json's projects and tests.json's are ${pin.rootProject}'s; check the pin out into a directory named ${pin.rootProject}")
    if !Files.isDirectory(dir.resolve(".git")) then Log.die(s"no checkout at $dir: run the verb checkout first")
    val head = git(dir, "rev-parse", "HEAD").out.trim
    if head != pin.commit then Log.die(s"$dir is at $head, where pin.json names ${pin.commit} (${pin.tag})")
    val changed = git(dir, "status", "--porcelain", "--untracked-files=no").out.trim
    if changed.nonEmpty then Log.die(s"$dir has tracked files changed:\n$changed")
    val lock = dir.resolve("package-lock.json")
    if !Files.isRegularFile(lock) || ScalaJsPin.sha256(Files.readAllBytes(lock)) != pin.lockfile then
      Log.die(s"$lock is not bench/$name/package-lock.json: run the verb checkout again")
    val jsdom = dir.resolve("node_modules/jsdom/package.json")
    val installed = if Files.isRegularFile(jsdom) then "\"version\": \"([^\"]+)\"".r.findFirstMatchIn(Files.readString(jsdom)).map(_.group(1)) else None
    if !installed.contains(pin.jsdom) then
      Log.die(s"$dir's node_modules holds jsdom ${installed.getOrElse("not at all")}, where pin.json names ${pin.jsdom}: run the verb checkout again")

  // JAVA_HOME's JDK, whose major version is the pin's.
  def javaHome: Path =
    val home = Option(System.getenv("JAVA_HOME")).filter(_.nonEmpty).map(Paths.get(_)).getOrElse(Log.die(s"JAVA_HOME names no JDK; the pin builds under Java ${pin.java}"))
    val r = Sh(home.resolve("bin/java").toString, "-version").timeout(60).run()
    val version = "version \"([0-9]+)".r.findFirstMatchIn(r.err + r.out).map(_.group(1).toInt)
    if !version.contains(pin.java) then Log.die(s"JAVA_HOME's java is ${version.getOrElse("of no version")}, where pin.json names ${pin.java}")
    home

  def sbtSeconds: Double = Option(System.getenv("REFERENCE_SBT_SECONDS")).filter(_.nonEmpty).map(_.toDouble).getOrElse(900.0)
  def teqSeconds: Double = Option(System.getenv("REFERENCE_TEQ_SECONDS")).filter(_.nonEmpty).map(_.toDouble).getOrElse(1800.0)

  // The build's sbt in the foreground with no server left behind, under JAVA_HOME's JDK (named to the launcher and
  // first on the PATH), under /usr/bin/time where there is one for the peak memory of the JVM; `env` adds variables.
  def sbt(dir: Path, log: Path, extra: Seq[String], commands: Seq[String], env: Seq[(String, String)] = Nil): (Sh.Result, Double, Option[Long]) =
    val home = javaHome
    val time = Paths.get("/usr/bin/time")
    val timed = if Files.isExecutable(time) then Seq(time.toString, "-v") else Nil
    val cmd = timed ++ Seq("sbt", "-java-home", home.toString, "--batch", "-no-colors", "-Dsbt.supershell=false", "-Dsbt.server.autostart=false") ++ extra ++ commands
    val started = System.nanoTime()
    val r = Sh(cmd*).in(dir).withEnv((Seq("JAVA_HOME" -> home.toString, "PATH" -> s"$home/bin:${System.getenv("PATH")}") ++ env)*).without("SBT_OPTS")
      .stdout(log).mergeErr.timeout(sbtSeconds).check(false).run()
    val wall = (System.nanoTime() - started) / 1e9
    val peak = "Maximum resident set size \\(kbytes\\): ([0-9]+)".r.findFirstMatchIn(Files.readString(log)).map(_.group(1).toLong)
    (r, wall, peak)

  def node: String = Sh("node", "--version").timeout(60).check(false).run().out.trim

  // The oracle: the build's own `++<scala>! clean test` under sbt with the oracle's listener (bench/reference/oracle,
  // by -addPluginSbtFile), whose record of every event, each status as the framework reported it and a case as many
  // times, is the run's inventory.
  def oracle(dir: Path, out: Path, runs: Int, record: Boolean): Unit =
    val expected = Tests.read(here.resolve("tests.json"))
    val listener = here.getParent.resolve("reference/oracle").toAbsolutePath.normalize
    val events = dir.resolve("target/oracle-events.txt")
    var failed = false
    for i <- 1 to runs do
      val log = out.resolve(s"oracle-$i.log")
      Files.deleteIfExists(events)
      Log.stamped(s"$name: oracle run $i of $runs (node ${node}), sbt's log $log")
      val (r, wall, peak) = sbt(dir, log, Seq(s"-Dteq.oracle.events=$listener", s"-addPluginSbtFile=${listener.resolve("plugins.sbt")}"), Seq(s"++${pin.scala}!", "clean", "test"))
      val found = if Files.isRegularFile(events) then Tests.fromEvents(Files.readString(events)) else Tests(Map.empty)
      if Files.isRegularFile(events) then Files.copy(events, out.resolve(s"oracle-$i-events.txt"), java.nio.file.StandardCopyOption.REPLACE_EXISTING)
      Files.writeString(out.resolve(s"oracle-$i-tests.json"), Tests.write(found))
      println(f"$name: oracle run $i: exit ${r.status}, $wall%.1f s, peak ${Reference.gigabytes(peak)}, ${found.suites.size} suites, ${found.counts}")
      if r.status != 0 then
        fail(s"oracle run $i: sbt exits ${r.status}")
        failed = true
      if record && i == runs then
        if found.suites.isEmpty || found.cases == 0 then Log.die(s"$name: oracle run $i executed nothing: no inventory is recorded")
        if r.status != 0 then Log.die(s"$name: oracle run $i failed: no inventory is recorded from it")
        Files.writeString(here.resolve("tests.json"), Tests.write(found))
        say(s"recorded ${found.suites.size} suites, ${found.cases} cases in bench/$name/tests.json")
      else
        val problems = Tests.compare(expected, found)
        problems.foreach(p => fail(s"oracle run $i: $p"))
        if problems.nonEmpty then failed = true
        else say(s"oracle run $i: ${found.suites.size} suites, ${found.cases} cases, as tests.json")
    Script.exit(if failed then 1 else 0)

  // The lock's path in the checkout.
  def lockOf(dir: Path): Path = dir.resolve("target/teq/teq.lock")

  def exportLock(dir: Path, out: Path, teq: String): Unit =
    val (version, repository) = SbtPlugin.forExport(out, javaHome)
    val overlay = here.resolve("overlay").toAbsolutePath.normalize
    val log = out.resolve("export.log")
    Files.deleteIfExists(lockOf(dir))
    Log.stamped(s"$name: teqExportAll with sbt-teq $version and the overlay, sbt's log $log")
    val (r, wall, peak) = sbt(dir, log,
      Seq(s"-Dteq.$name.overlay=$overlay", s"-Dteq.$name.scripts=${here.toAbsolutePath.normalize}", s"-Dteq.plugin.version=$version") ++
        repository.map(r => s"-Dteq.plugin.repository=$r") :+ s"-addPluginSbtFile=${overlay.resolve("plugins.sbt")}",
      Seq(s"++${pin.scala}!", "teqExportAll"), Seq("TEQ" -> binary(teq)))
    if r.status != 0 then
      println(Files.readString(log).linesIterator.toVector.takeRight(20).mkString("\n"))
      Log.die(s"$name: the export exits ${r.status}: ${SbtPlugin.missing(log).getOrElse(Log.cause(log))} (sbt's log $log)")
    val lock = lockOf(dir)
    Files.copy(lock, out.resolve("teq.lock"), java.nio.file.StandardCopyOption.REPLACE_EXISTING)
    say(f"exported ${Files.size(lock)} bytes in $wall%.1f s (peak ${Reference.gigabytes(peak)}), copied to ${out.resolve("teq.lock")}")
    // The export compiles nothing of the build: sbt's compile of a project writes under the checkout's target, the
    // meta-build's (the overlay) under project/.
    val compiled = "compiling [0-9]+ \\S+ sources? to (\\S+)".r.findAllMatchIn(Files.readString(log)).map(_.group(1))
      .filterNot(p => p.startsWith(dir.resolve("project").toAbsolutePath.toString)).toVector
    if compiled.nonEmpty then Log.die(s"$name: the export compiled the build into ${compiled.distinct.mkString(", ")}")
    val refused = Files.readString(lock).linesIterator.count(_.trim == "kind: sbt")
    if refused > 0 then Log.die(s"$name: the lock records $refused generators only sbt runs")

  // The export's lock in the checkout, which records no generator only sbt runs and pin.json's projects.
  def exported(dir: Path): Path =
    val lock = lockOf(dir)
    if !Files.isRegularFile(lock) then Log.die(s"no export at $lock: run the verb export first")
    if Files.readString(lock).linesIterator.exists(_.trim == "kind: sbt") then Log.die(s"$lock records a generator only sbt runs: export again with the overlay")
    val projects = lockProjects(lock)
    val missing = pin.projects.filterNot(projects.contains)
    if missing.nonEmpty then Log.die(s"$lock exports the projects ${projects.mkString(", ")}, not pin.json's ${missing.mkString(", ")}")
    lock

  // The projects a lock exports: the keys of its `projects` block.
  def lockProjects(lock: Path): Vector[String] =
    Files.readString(lock).linesIterator.dropWhile(_ != "projects:").drop(1).takeWhile(l => l.isEmpty || l.startsWith(" "))
      .filter(l => l.startsWith("  ") && !l.startsWith("   ") && l.trim.endsWith(":")).map(_.trim.stripSuffix(":")).toVector

  // The binary, which is given as a path.
  def binary(teq: String): String =
    val bin = Paths.get(teq).toAbsolutePath.normalize
    if !Files.isExecutable(bin) then Log.die(s"no teq binary at $bin (--teq)")
    bin.toString

  // The files the lock's generators write besides their directories under target/teq: each `outputs` entry of a
  // `generators` block not under target/.
  def generatedOutputs(lock: Path): Vector[String] =
    val lines = Files.readString(lock).linesIterator.toVector
    val out = Vector.newBuilder[String]
    var outputsIndent = -1
    for line <- lines do
      val indent = line.takeWhile(_ == ' ').length
      val trimmed = line.trim
      if trimmed == "outputs:" then outputsIndent = indent
      else if outputsIndent >= 0 && indent > outputsIndent && trimmed.startsWith("- ") then
        val path = trimmed.drop(2).trim
        if !path.startsWith("target/") then out += path
      else outputsIndent = -1
    out.result().distinct.sorted

  // teq's outputs of the checkout but the lock and the generators' records: what a cold compile writes again.
  def coldOutputs(dir: Path): Unit =
    val teqDir = dir.resolve("target/teq")
    for entry <- Files.list(teqDir).toArray.toList.map(_.asInstanceOf[Path]) if Files.isDirectory(entry) && entry.getFileName.toString != "generators" do
      Sh.remove(entry)

  // The generated files the lock's generators write against the bytes the pin commits: a file not written, or written
  // with other bytes (with the digest of those bytes, so that other wrong bytes are another item), an item of the run's
  // debt; the checkout put back at the pin after, for the verbs after this one.
  def generatedDebt(dir: Path, lock: Path): Vector[String] =
    val generated = generatedOutputs(lock)
    val missing = generated.filterNot(g => Files.isRegularFile(dir.resolve(g)))
    val changed = git(dir, "status", "--porcelain", "--untracked-files=no").out.linesIterator.filter(_.length > 3).map(l => (l.take(2).trim, l.drop(3))).toVector
    def written(f: String): String =
      val at = dir.resolve(f)
      if Files.isRegularFile(at) then s"generated $f: ${ScalaJsPin.sha256(Files.readAllBytes(at))}, not the pin's bytes" else s"generated $f: not written"
    val debt = (missing.map(g => s"generated $g: not written") ++ changed.filterNot((_, f) => missing.contains(f)).map((_, f) => written(f))).distinct
    if changed.nonEmpty then git(dir, "checkout", "--", ".")
    debt

  def compile(dir: Path, out: Path, teq: String, runs: Int, mode: Mode): Unit =
    val lock = exported(dir)
    val bin = binary(teq)
    val version = Sh(bin, "--version").timeout(30).check(false).run().out.trim
    val roots = Roots.of(dir, dir, out)
    val generated = generatedOutputs(lock)
    var failed = false
    val held = Vector.newBuilder[(String, Observation, String)]
    for i <- 1 to runs do
      Sh(bin, "stop").in(dir).timeout(120).check(false).run()
      coldOutputs(dir)
      generated.foreach(g => Files.deleteIfExists(dir.resolve(g)))
      val log = out.resolve(s"compile-$i.log")
      // The builds' diagnostics as teq records them (`--diagnostics`), never read from the text.
      val diagnostics = out.resolve(s"compile-$i.diagnostics")
      Files.deleteIfExists(diagnostics)
      Log.stamped(s"$name: teq compile ${pin.projects.mkString(" ")}, run $i of $runs, its output $log")
      val started = System.nanoTime()
      val (r, peak) = Reference.sampled(out, teqSeconds + 600)(mark => Sh((Seq(bin, "compile", "--diagnostics", diagnostics.toString) ++ pin.projects)*).in(dir).withEnv(mark).stdout(log).mergeErr.timeout(teqSeconds).check(false).run())
      val wall = (System.nanoTime() - started) / 1e9
      Files.writeString(log, Transcript.mark(r.status), java.nio.file.StandardOpenOption.APPEND)
      val text = Files.readString(log)
      // The generators ran again and wrote the bytes the pin commits: a file not written, or written with other bytes
      // (with the digest of those bytes, so that other wrong bytes are another item), is a departure of the run's, an
      // item of its debt beside teq's errors.
      val generatedDebt = this.generatedDebt(dir, lock)
      // Every compile configuration of the pin's projects the export holds sources for covered by the run.
      val observed = Locks.hold(Transcript.withDebt(Transcript.compile(text, Transcript.recorded(diagnostics), roots), generatedDebt), lock, pin.projects)
      println(f"$name: compile run $i: exit ${r.status}, $wall%.1f s, peak ${Reference.gigabytes(peak)}, ${observed.debt.size - generatedDebt.size} errors, ${generatedDebt.size} generated files not the pin's")
      held += ((s"compile run $i", observed, roots(text)))
    val seen = held.result()
    if !Lines.hold(name, "compile", here, Shape.Compile, seen.map((n, o, _) => n -> o), seen.head._3, mode, version) then failed = true
    if !failed then say(s"the ${generated.size} generated files held to the pin's bytes")
    Sh(bin, "stop").in(dir).timeout(120).check(false).run()
    Script.exit(if failed then 1 else 0)

  def test(dir: Path, out: Path, teq: String, runs: Int, mode: Mode, inventory: Path = here.resolve("tests.json")): Unit =
    val lock = exported(dir)
    val bin = binary(teq)
    val version = Sh(bin, "--version").timeout(30).check(false).run().out.trim
    val roots = Roots.of(dir, dir, out)
    val held = Vector.newBuilder[(String, Observation, String)]
    // The oracle's inventory, which the run is held to: without one nothing would be checked.
    if !Files.isRegularFile(inventory) then Log.die(s"no inventory at $inventory: record it with the oracle (oracle --record)")
    val expected = Tests.read(inventory)
    if expected.suites.isEmpty || expected.cases == 0 then Log.die(s"the inventory $inventory holds no suite or no case")
    var failed = false
    for i <- 1 to runs do
      val log = out.resolve(s"test-$i.log")
      val errLog = out.resolve(s"test-$i.err")
      val events = out.resolve(s"test-$i-events.txt")
      val diagnostics = out.resolve(s"test-$i.diagnostics")
      Files.deleteIfExists(events)
      Files.deleteIfExists(diagnostics)
      // The daemon the run uses is the one it starts, which carries the run's token (`Reference.sampled`).
      Sh(bin, "stop").in(dir).timeout(120).check(false).run()
      Log.stamped(s"$name: teq test ${pin.projects.mkString(" ")}, run $i of $runs, its output $log, teq's stderr $errLog")
      val started = System.nanoTime()
      // The run's events (`--events`), each status as the framework reported it, are its inventory; verbose, the
      // transcript names every test with its outcome. Its two channels are collected apart, teq's own lines (a build's
      // diagnostics and count, the summaries) on stderr, the run's stream on stdout; the outcomes are the events'
      // (`Transcript`).
      val (r, peak) = Reference.sampled(out, teqSeconds + 600)(mark => Sh((Seq(bin, "test", "--verbose", "--events", events.toString, "--diagnostics", diagnostics.toString) ++ pin.projects)*).in(dir).withEnv(mark).stdout(log).stderr(errLog).timeout(teqSeconds).check(false).run())
      val wall = (System.nanoTime() - started) / 1e9
      Files.writeString(log, Transcript.mark(r.status), java.nio.file.StandardOpenOption.APPEND)
      val transcript = Files.readString(log)
      val errText = if Files.isRegularFile(errLog) then Files.readString(errLog) else ""
      val eventText = Option.when(Files.isRegularFile(events))(Files.readString(events))
      val found = eventText.fold(Tests(Map.empty))(Tests.fromEvents)
      println(f"$name: test run $i: exit ${r.status}, $wall%.1f s, peak ${Reference.gigabytes(peak)}, ${found.suites.size} suites, ${found.cases} cases (${found.counts}); tests.json: ${expected.suites.size} suites, ${expected.cases} cases (${expected.counts})")
      val differences = Tests.compare(expected, found)
      if differences.nonEmpty then
        say(s"test run $i: ${differences.size} differences from tests.json")
        differences.take(40).foreach(d => println(s"  $d"))
      // The run held to deviations.json by the records' one judgment of a test run (`Deviations.judge`, the same for
      // every driver): the invocation's channels and events, the oracle's projects and cases; and the generators the
      // run's compile ran wrote the bytes the pin commits, a file that departs an item as the compile's (the checkout
      // goes back to the pin, for the verbs after this one).
      val evidence = TestEvidence(Vector(Invocation(s"teq test ${pin.projects.mkString(" ")}", r.status, transcript, errText, Transcript.recorded(diagnostics), eventText)),
        expected.heldSuites.map(_._1).distinct, expected.items, expected.heldSuites, debt = generatedDebt(dir, lock), owed = Locks.testUnits(lock, pin.projects), types = Locks.typesTests(lock))
      val observed = Deviations.judge(evidence, roots)
      if observed.problems.nonEmpty then Invocation.shown(transcript, errText).linesIterator.toVector.takeRight(20).foreach(l => println(s"  $l"))
      held += ((s"test run $i", observed, roots(Invocation.shown(transcript, errText))))
    Sh(bin, "stop").in(dir).timeout(120).check(false).run()
    val seen = held.result()
    if !Lines.hold(name, "test", here, Shape.Tests, seen.map((n, o, _) => n -> o), seen.head._3, mode, version) then failed = true
    Script.exit(if failed then 1 else 0)

object ScalaJsReference:
  // The verbs every Scala.js reference project has; `extra` names the project's own, which `other` runs.
  def main(name: String, argv: Seq[String], extra: Seq[String] = Nil)(own: (ScalaJsReference, String, Path, Path, String, Args) => Unit = (_, v, _, _, _, a) => a.fail(s"no verb $v")): Unit =
    val verbs = Seq("checkout", "oracle", "export", "compile", "test") ++ extra
    val args = new Args(argv, s"bench/$name/$name.scala <${verbs.mkString("|")}> <checkout> <out> [--runs n] [--record | --baseline | --migrate <reason>] [--teq <binary>]")
    val teq = args.teq("./target/release/teq")
    val runs = args.option("runs").map(_.toInt).getOrElse(1)
    val mode = Mode.of(args)
    val record = mode == Mode.Record
    val positionals = args.exactly(3)
    val (verb, checkoutArg, outArg) = (positionals(0), positionals(1), positionals(2))
    val project = new ScalaJsReference(name)
    val checkout = Paths.get(checkoutArg).toAbsolutePath.normalize
    val out = Paths.get(outArg).toAbsolutePath.normalize
    if out.startsWith(Paths.get("").toAbsolutePath) then Log.die(s"$out is inside the repository; the logs and caches stay outside it")
    Files.createDirectories(out)
    project.checkFixtures()
    verb match
      case "checkout" => project.checkout(checkout)
      case "oracle" =>
        if mode != Mode.Judge && mode != Mode.Record then args.fail("the oracle records its inventory with --record alone")
        project.atPin(checkout)
        project.oracle(checkout, out, runs, record)
      case "export" =>
        project.atPin(checkout)
        project.exportLock(checkout, out, teq)
      case "compile" =>
        project.atPin(checkout)
        project.compile(checkout, out, teq, runs, mode)
      case "test" =>
        project.atPin(checkout)
        project.test(checkout, out, teq, runs, mode)
      case other if extra.contains(other) =>
        project.atPin(checkout)
        own(project, other, checkout, out, teq, args)
      case other => args.fail(s"no verb $other")
