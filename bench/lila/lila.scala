//> using file ../../tools/script
//> using file ../reference/reference.scala
//> using file ../reference/records.scala
// lila, lichess's server (github.com/lichess-org/lila), as teq's public reference project: the pinned
// revision checked out, built and tested under scalac 3.8.4 by its own sbt build (the oracle), exported with
// sbt-teq through the overlay, then compiled and tested by teq against the oracle's results. Run from the
// repository's root:
//
//   ./teq interp bench/lila/lila.scala -- <verb> <checkout> <out> [options]
//
// checkout  clones lila into <checkout> (blob-filtered) or fetches the pin into an existing clone, checks out
//           pin.json's commit detached, refuses a tracked file changed, and writes conf/version.conf from the pin
//           as lila's CI writes it from the commit (never from the clock)
// oracle    lila's sbt build, as its CI runs it with `testFull` for `test` (sbt 2's `test` is testQuick, which
//           reruns nothing and reports green): `clean;compile;testFull;stage` under the JDK pin.json names, a
//           fresh sbt disk cache under <out> each run (--runs n, default 1), each run's wall time, peak memory
//           and its suites and cases (sbt's JUnit reports) against tests.json; --record rewrites tests.json
// export    `sbt teqExportAll` with the overlay (overlay/plugins.sbt by sbt's -addPluginSbtFile, sbt-teq at
//           TEQ_PLUGIN_VERSION, by default the head's integrations/sbt published under a SNAPSHOT of its sources'
//           digest into a repository beside <out> (`SbtPlugin`); overlay/LilaTeq.scala declares the
//           generators teq runs in the place of sbt's: the routers and the translations), the lock copied to <out>;
//           a lock recording a generator only sbt runs fails
// compile   `teq compile` of every project over the export (--teq <binary>, by default ./target/release/teq), the
//           daemon stopped and teq's class files removed before each run (--runs n), the generators' outputs kept:
//           each run's wall time and the peak memory of teq's processes (sampled), each run held to errors.txt
//           (bench/reference/records.scala: every configuration compiled once, the run complete, its errors within
//           the record's, a new one failing, one gone listed; --record writes a passing run's improvement,
//           --baseline the first record, --migrate <reason> one over new inputs), then the generated inputs' digests
//           against generated.json's (--baseline and --migrate rewrite them from a run that is evidence)
// test      `teq test --events` of every project: its suites and cases, as its events report each with its framework's
//           status, against tests.json, the oracle's, each run's departures
//           held to deviations.json (bench/reference/records.scala: every project of the oracle's run or blocked by
//           its own build's failure, a case that departs, or is blocked, an item; --record writes a passing run's
//           improvement, --baseline the first record, --migrate <reason> one over a new pin or inventory) (--runs n)
// run       MongoDB and Redis (LILA_MONGOD and LILA_REDIS, else mongod and redis-server on the PATH) on lila's ports,
//           `teq run lila` over the export, the home page within LILA_READY_SECONDS (default 600): its content
//           (lila's title and an asset of the manifest) and that asset served, each a named assertion held to
//           run.json (bench/reference/records.scala: an assertion not held a violation by its name, a run whose build
//           failed blocking every one; --record, --baseline, --migrate as compile's); the start-up time to the home
//           page (--runs n). lila's assets are built by its ui/build (public/compiled/manifest.json): a checkout without
//           them fails
// measure   compile, test and run in turn (--runs n each), then the lines' admission: the three verbs' median
//           wall times summed against the 120 s the gate may add, and the session's peak against 12 GB. The lines
//           stay off in tests/gate.sh whatever this prints: admission takes successful runs on both machines
//
// Each run of compile, test and run measures the memory of the process trees it starts (`Reference.sampled`): the
// processes whose environment carries the run's token (the client, the daemon it starts after the verb stopped the
// last one, its residents, and the JVMs of teq's test runner, `java @<argfile> dev.teq.runner.Runner`), with every
// descendant of theirs, their resident set sizes summed each second; another invocation or session counts for
// nothing. A run without a measurement fails. Each verb prints the binary's version and build profile
// (the directory of target/ it stands in) and the generators' state before and after (`kept`: their records
// untouched), and the session's peak over its runs.
//
// Every verb checks that the checkout is at the pin and that the overlay's files are the bytes pin.json
// records: the pin, the oracle's results (tests.json) and the overlay move together. <out> holds the logs,
// sbt's caches and the lock and stays outside the repository. JAVA_HOME names the JDK (its major version
// must be pin.json's); sbt is the one on the PATH. Every command is bounded: LILA_SBT_SECONDS (default 900)
// for an sbt run, 300 for git.
import java.nio.file.{Files, Path, Paths}
import java.security.MessageDigest

@main def lila(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/lila/lila.scala <checkout|oracle|export|compile|test|run|measure> <checkout> <out> [--runs n] [--record | --baseline | --migrate <reason>] [--teq <binary>]")
  val teq = args.teq("./target/release/teq")
  val runs = args.option("runs").map(_.toInt).getOrElse(1)
  val mode = Mode.of(args)
  val record = mode == Mode.Record
  val positionals = args.exactly(3)
  val (verb, checkoutArg, outArg) = (positionals(0), positionals(1), positionals(2))
  val here = Paths.get("bench/lila")
  if !Files.isRegularFile(here.resolve("pin.json")) then Log.die("bench/lila/lila.scala runs from the repository's root")
  val pin = Pin.read(here)
  val checkout = Paths.get(checkoutArg).toAbsolutePath.normalize
  val out = Paths.get(outArg).toAbsolutePath.normalize
  if out.startsWith(Paths.get("").toAbsolutePath) then Log.die(s"$out is inside the repository; the logs and caches stay outside it")
  Files.createDirectories(out)
  pin.checkOverlay(here)
  verb match
    case "checkout" => Lila.checkout(pin, checkout)
    case "oracle" =>
      if mode != Mode.Judge && mode != Mode.Record then args.fail("the oracle records its inventory with --record alone")
      Lila.atPin(pin, checkout)
      Lila.oracle(pin, here, checkout, out, runs, record)
    case "export" =>
      Lila.atPin(pin, checkout)
      Lila.exportLock(pin, here, checkout, out)
    case "compile" =>
      Lila.atPin(pin, checkout)
      Script.exit(if Lila.compile(pin, here, checkout, out, teq, runs, mode).failed then 1 else 0)
    case "test" =>
      Lila.atPin(pin, checkout)
      Script.exit(if Lila.test(pin, here, checkout, out, teq, runs, mode).failed then 1 else 0)
    case "run" =>
      Lila.atPin(pin, checkout)
      Script.exit(if Lila.serve(pin, here, checkout, out, teq, runs, mode).failed then 1 else 0)
    case "measure" =>
      Lila.atPin(pin, checkout)
      Script.exit(if Lila.measure(pin, here, checkout, out, teq, runs) then 0 else 1)
    case other => args.fail(s"no verb $other")
}

// bench/lila/pin.json: the revision of lila, its toolchain, the oracle's inventory and the overlay's digests.
final case class Pin(json: Json):
  def repository: String = json("repository").str
  def commit: String = json("commit").str
  def date: String = json("date").str
  def java: Int = json("java").int
  def sbt: String = json("sbt").str

  // The overlay's files are the bytes pin.json records, and no other file is in its directory.
  def checkOverlay(here: Path): Unit =
    val recorded = json("overlay").members.map((f, d) => f -> d.str).toMap
    val dir = here.resolve("overlay")
    val present = Files.list(dir).toArray.toList.map(_.asInstanceOf[Path].getFileName.toString).sorted
    if present != recorded.keys.toList.sorted then
      Log.die(s"bench/lila/overlay holds ${present.mkString(", ")}, where pin.json records ${recorded.keys.toList.sorted.mkString(", ")}")
    for (f, digest) <- recorded do
      val actual = Pin.sha256(Files.readAllBytes(dir.resolve(f)))
      if actual != digest then Log.die(s"bench/lila/overlay/$f is $actual, where pin.json records $digest: the pin, tests.json and the overlay move together")

object Pin:
  def read(here: Path): Pin = Pin(Json.parse(Files.readString(here.resolve("pin.json"))))
  def sha256(bytes: Array[Byte]): String =
    MessageDigest.getInstance("SHA-256").digest(bytes).map(b => f"${b & 0xff}%02x").mkString

object Lila:
  private def git(dir: Path, a: String*): Sh.Result = Sh(("git" +: "-C" +: dir.toString +: a)*).timeout(300).run()

  // lila's conf/version.conf as its CI writes it (`git log -1 --date=iso-strict-local` under TZ=UTC), from the pin.
  def versionConf(pin: Pin): String = s"app.version.commit = \"${pin.commit}\"\napp.version.date = \"${pin.date}\""

  def checkout(pin: Pin, dir: Path): Unit =
    if !Files.isDirectory(dir.resolve(".git")) then
      Files.createDirectories(dir.getParent)
      Log.stamped(s"lila: cloning ${pin.repository} into $dir")
      Sh("git", "clone", "--filter=blob:none", "--no-checkout", pin.repository, dir.toString).timeout(300).run()
    val has = Sh("git", "-C", dir.toString, "cat-file", "-e", pin.commit + "^{commit}").timeout(60).check(false).run().ok
    if !has then
      Log.stamped(s"lila: fetching ${pin.commit}")
      git(dir, "fetch", "--filter=blob:none", "origin", pin.commit)
    git(dir, "checkout", "--quiet", "--detach", pin.commit)
    atPin(pin, dir)
    Files.writeString(dir.resolve("conf/version.conf"), versionConf(pin))
    Log.stamped(s"lila: $dir at ${pin.commit}, conf/version.conf written from the pin")

  // The checkout is at the pin, no tracked file changed, and its version.conf is the pin's.
  def atPin(pin: Pin, dir: Path): Unit =
    if !Files.isDirectory(dir.resolve(".git")) then Log.die(s"no checkout at $dir: run the verb checkout first")
    val head = git(dir, "rev-parse", "HEAD").out.trim
    if head != pin.commit then Log.die(s"$dir is at $head, where pin.json names ${pin.commit}")
    val changed = git(dir, "status", "--porcelain", "--untracked-files=no").out.trim
    if changed.nonEmpty then Log.die(s"$dir has tracked files changed:\n$changed")
    val conf = dir.resolve("conf/version.conf")
    if Files.isRegularFile(conf) && Files.readString(conf) != versionConf(pin) then
      Log.die(s"$conf is not the pin's: run the verb checkout again")
    if !Files.isRegularFile(conf) then Files.writeString(conf, versionConf(pin))

  // JAVA_HOME's JDK, whose major version is the pin's.
  def javaHome(pin: Pin): Path =
    val home = Option(System.getenv("JAVA_HOME")).filter(_.nonEmpty).map(Paths.get(_)).getOrElse(Log.die(s"JAVA_HOME names no JDK; lila's pin builds under Java ${pin.java}"))
    val r = Sh(home.resolve("bin/java").toString, "-version").timeout(60).run()
    val version = "version \"([0-9]+)".r.findFirstMatchIn(r.err + r.out).map(_.group(1).toInt)
    if !version.contains(pin.java) then Log.die(s"JAVA_HOME's java is ${version.getOrElse("of no version")}, where pin.json names ${pin.java}")
    home

  def sbtSeconds: Double = Option(System.getenv("LILA_SBT_SECONDS")).filter(_.nonEmpty).map(_.toDouble).getOrElse(900.0)

  // lila's sbt as lila.sh starts it (its .sbtopts and the JVM options lila.sh adds), in the foreground with no
  // server left behind, under /usr/bin/time where there is one for the peak memory of the JVM.
  def sbt(pin: Pin, dir: Path, log: Path, extra: Seq[String], commands: Seq[String]): (Sh.Result, Double, Option[Long]) =
    val home = javaHome(pin)
    if !Files.exists(dir.resolve(".sbtopts")) then Files.copy(dir.resolve(".sbtopts.default"), dir.resolve(".sbtopts"))
    if !Files.exists(dir.resolve("conf/application.conf")) then Files.copy(dir.resolve("conf/application.conf.default"), dir.resolve("conf/application.conf"))
    val time = Paths.get("/usr/bin/time")
    val timed = if Files.isExecutable(time) then Seq(time.toString, "-v") else Nil
    val cmd = timed ++ Seq("sbt", "-Dreactivemongo.api.bson.document.strict=false", "-Depoll=true", "--server", "--batch", "-no-colors",
      "-Dsbt.server.autostart=false") ++ extra ++ commands
    val started = System.nanoTime()
    val r = Sh(cmd*).in(dir).withEnv("JAVA_HOME" -> home.toString, "PATH" -> s"$home/bin:${System.getenv("PATH")}").without("SBT_OPTS")
      .stdout(log).mergeErr.timeout(sbtSeconds).check(false).run()
    val wall = (System.nanoTime() - started) / 1e9
    val peak = "Maximum resident set size \\(kbytes\\): ([0-9]+)".r.findFirstMatchIn(Files.readString(log)).map(_.group(1).toLong)
    (r, wall, peak)

  def oracle(pin: Pin, here: Path, dir: Path, out: Path, runs: Int, record: Boolean): Unit =
    val expected = Tests.read(here.resolve("tests.json"))
    var failed = false
    for i <- 1 to runs do
      val cache = out.resolve(s"oracle-cache-$i")
      Sh.remove(cache)
      val log = out.resolve(s"oracle-$i.log")
      Log.stamped(s"lila: oracle run $i of $runs, sbt's log $log")
      val (r, wall, peak) = sbt(pin, dir, log, Seq(s"-Dsbt.global.localcache=$cache"), Seq("clean;compile;testFull;stage"))
      val steps = "\\[success\\] elapsed time: ([0-9]+) s".r.findAllMatchIn(Files.readString(log)).map(_.group(1)).toList
      println(f"lila: oracle run $i: exit ${r.status}, $wall%.1f s, peak ${Reference.gigabytes(peak)}, steps ${steps.mkString(" ")} s (load, compile, testFull, stage)")
      val found = Tests.fromReports(dir)
      Files.writeString(out.resolve(s"oracle-$i-tests.json"), Tests.write(found))
      if r.status != 0 then
        println(s"FAIL lila: oracle run $i: sbt exits ${r.status}")
        failed = true
      if record && i == runs then
        Files.writeString(here.resolve("tests.json"), Tests.write(found))
        println(s"lila: recorded ${found.suites.size} suites, ${found.cases} cases in bench/lila/tests.json")
      else
        val problems = Tests.compare(expected, found)
        problems.foreach(p => println(s"FAIL lila: oracle run $i: $p"))
        if problems.nonEmpty then failed = true
        else println(s"lila: oracle run $i: ${found.suites.size} suites, ${found.cases} cases, as tests.json")
    Script.exit(if failed then 1 else 0)

  def exportLock(pin: Pin, here: Path, dir: Path, out: Path): Unit =
    val (version, repository) = SbtPlugin.forExport(out, javaHome(pin))
    val overlay = here.resolve("overlay").toAbsolutePath.normalize
    val log = out.resolve("export.log")
    Log.stamped(s"lila: teqExportAll with sbt-teq $version and the overlay, sbt's log $log")
    val (r, wall, peak) = sbt(pin, dir, log,
      Seq(s"-Dteq.lila.overlay=$overlay", s"-Dteq.lila.scripts=${here.toAbsolutePath.normalize}", s"-Dteq.plugin.version=$version") ++
        repository.map(r => s"-Dteq.plugin.repository=$r") :+ s"-addPluginSbtFile=${overlay.resolve("plugins.sbt")}", Seq("teqExportAll"))
    if r.status != 0 then
      println(Files.readString(log).linesIterator.toVector.takeRight(20).mkString("\n"))
      Log.die(s"lila: the export exits ${r.status}: ${SbtPlugin.missing(log).getOrElse(Log.cause(log))} (sbt's log $log)")
    val lock = dir.resolve("target/teq/teq.lock")
    Files.copy(lock, out.resolve("teq.lock"), java.nio.file.StandardCopyOption.REPLACE_EXISTING)
    println(f"lila: exported ${Files.size(lock)} bytes in $wall%.1f s (peak ${Reference.gigabytes(peak)}), copied to ${out.resolve("teq.lock")}")
    val refused = Files.readString(lock).linesIterator.count(_.trim == "kind: sbt")
    if refused > 0 then Log.die(s"lila: the lock records $refused generators only sbt runs")

  // The export's lock in the checkout, which records no generator only sbt runs.
  def exported(dir: Path): Path =
    val lock = dir.resolve("target/teq/teq.lock")
    if !Files.isRegularFile(lock) then Log.die(s"no export at $lock: run the verb export first")
    if Files.readString(lock).linesIterator.exists(_.trim == "kind: sbt") then Log.die(s"$lock records a generator only sbt runs: export again with the overlay")
    lock

  // The binary, which is given as a path; its version line for the logs.
  def binary(teq: String): String =
    val bin = Paths.get(teq).toAbsolutePath.normalize
    if !Files.isExecutable(bin) then Log.die(s"no teq binary at $bin (--teq)")
    bin.toString

  def teqSeconds: Double = Option(System.getenv("LILA_TEQ_SECONDS")).filter(_.nonEmpty).map(_.toDouble).getOrElse(1800.0)

  // A verb's runs: each one's wall time (the home page's start-up time for run) and peak, and whether one failed.
  final case class Measured(walls: Vector[Double], peaks: Vector[Option[Long]], failed: Boolean):
    def median: Double = if walls.isEmpty then 0.0 else walls.sorted.apply(walls.size / 2)
    def peak: Option[Long] = if peaks.isEmpty || peaks.exists(_.isEmpty) then None else peaks.flatten.maxOption

  // The binary's version line and build profile, the directory of target/ it stands in (release, ship, fast).
  def describe(bin: String): Unit =
    val version = Sh(bin, "--version").timeout(30).check(false).run().out.trim
    val parent = Paths.get(bin).getParent
    val profile = Option(parent).filter(p => Option(p.getParent).exists(_.getFileName.toString.startsWith("target"))).map(_.getFileName.toString).getOrElse("unknown")
    println(s"lila: binary $bin: $version, profile $profile")

  // The generators' records under target/teq/generators, by name with their modification times.
  def generatorState(dir: Path): Map[String, Long] =
    val root = dir.resolve("target/teq/generators")
    if !Files.isDirectory(root) then Map.empty
    else Files.list(root).toArray.toList.map(_.asInstanceOf[Path]).map(p => p.getFileName.toString -> Files.getLastModifiedTime(p).toMillis).toMap

  def tellGenerators(before: Map[String, Long], after: Map[String, Long]): Unit =
    val names = (before.keySet ++ after.keySet).toVector.sorted
    val state = names.map(n => s"$n ${if before.get(n) == after.get(n) && before.contains(n) then "kept" else if before.contains(n) then "ran" else "made"}")
    println(s"lila: generators: ${if state.isEmpty then "none" else state.mkString(", ")}")

  // A run's line, failing on a missing measurement: no sample, or none above zero over a run of two seconds or
  // more (the sampler samples each second, the first at the start).
  def measuredLine(what: String, peak: Option[Long], wall: Double): (String, Boolean) = peak.filter(k => k > 0 || wall < 2.0) match
    case Some(k) => (s"peak ${Reference.gigabytes(Some(k))}", false)
    case None =>
      println(s"FAIL lila: $what: no memory measurement")
      ("peak unmeasured", true)

  def sessionPeak(verb: String, m: Measured): Unit =
    println(s"lila: $verb: the session's peak ${Reference.gigabytes(m.peak)} over ${m.walls.size} runs, the median wall time ${f"${m.median}%.1f"} s")

  // teq's outputs of the checkout but the lock, the generators' records and their outputs (the routers'
  // sources, the translations): what a cold compile writes again.
  def coldOutputs(dir: Path): Unit =
    val teqDir = dir.resolve("target/teq")
    for entry <- Files.list(teqDir).toArray.toList.map(_.asInstanceOf[Path]) if Files.isDirectory(entry) && entry.getFileName.toString != "generators" do
      for conf <- Files.list(entry).toArray.toList.map(_.asInstanceOf[Path]) do
        val name = conf.getFileName.toString
        if Files.isDirectory(conf) && name != "resources" then
          for part <- Files.list(conf).toArray.toList.map(_.asInstanceOf[Path]) if part.getFileName.toString != "src_managed" do Sh.remove(part)

  // The digests of the generated inputs: every file under each generator's output, by its path, sorted.
  def generatedDigests(dir: Path): Map[String, String] =
    val outputs = Seq("target/teq/lila/compile/src_managed", "target/teq/ui/compile/src_managed", "target/teq/i18n/resources")
    outputs.map { o =>
      val root = dir.resolve(o)
      val files = if Files.isDirectory(root) then Files.walk(root).toArray.toList.map(_.asInstanceOf[Path]).filter(Files.isRegularFile(_)).sortBy(_.toString) else Nil
      val listing = files.map(f => s"${root.relativize(f)} ${Pin.sha256(Files.readAllBytes(f))}\n").mkString
      o -> Pin.sha256(listing.getBytes(java.nio.charset.StandardCharsets.UTF_8))
    }.toMap

  def compile(pin: Pin, here: Path, dir: Path, out: Path, teq: String, runs: Int, mode: Mode): Measured =
    val lock = exported(dir)
    val bin = binary(teq)
    describe(bin)
    val version = Sh(bin, "--version").timeout(30).check(false).run().out.trim
    val roots = Roots.of(dir, dir, out)
    var failed = false
    val (walls, peaks) = (Vector.newBuilder[Double], Vector.newBuilder[Option[Long]])
    val seen = Vector.newBuilder[(Observation, String, Map[String, String])]
    val generatorsBefore = generatorState(dir)
    for i <- 1 to runs do
      Sh(bin, "stop").in(dir).timeout(120).check(false).run()
      coldOutputs(dir)
      val log = out.resolve(s"compile-$i.log")
      // The builds' diagnostics as teq records them (`--diagnostics`), each its whole message, never read from the text.
      val diagnostics = out.resolve(s"compile-$i.diagnostics")
      Files.deleteIfExists(diagnostics)
      Log.stamped(s"lila: teq compile, run $i of $runs, its output $log")
      val started = System.nanoTime()
      val (r, peak) = Reference.sampled(out, teqSeconds + 600)(mark => Sh(bin, "compile", "--diagnostics", diagnostics.toString).in(dir).withEnv(mark).stdout(log).mergeErr.timeout(teqSeconds).check(false).run())
      val wall = (System.nanoTime() - started) / 1e9
      Files.writeString(log, Transcript.mark(r.status), java.nio.file.StandardOpenOption.APPEND)
      val text = Files.readString(log)
      // Every compile configuration the export holds sources for covered by the run (`Locks.hold`).
      val observed = Locks.hold(Transcript.compile(text, Transcript.recorded(diagnostics), roots), lock)
      val (shown, unmeasured) = measuredLine(s"compile run $i", peak, wall)
      println(f"lila: compile run $i: exit ${r.status}, $wall%.1f s, $shown, ${observed.debt.size} errors")
      walls += wall
      peaks += peak
      // The generated inputs' digests as this run left them: each run's held apart, so that no later run hides an
      // earlier one's bytes.
      seen += ((observed, roots(text), generatedDigests(dir)))
      if unmeasured then failed = true
    tellGenerators(generatorsBefore, generatorState(dir))
    Sh(bin, "stop").in(dir).timeout(120).check(false).run()
    val runsSeen = seen.result()
    // A baseline or a migration publishes the generated inputs' digests its first run left with its record, one
    // recording, every run held to them; a run held to the record holds them to generated.json's. A generated input
    // a run left with other bytes is an item of that run's debt with the digest it has (other bytes another item, so
    // that a change of the generated bytes takes a migration).
    val recording = mode match
      case Mode.Baseline | Mode.Migrate(_) => true
      case _ => false
    val expected = if recording then runsSeen.head._3 else Json.parse(Files.readString(here.resolve("generated.json"))).members.map((k, v) => k -> v.str).toMap
    val staged = if recording then Vector("generated.json" -> (Json.write(Json.Obj(expected.toVector.sortBy(_._1).map((k, v) => k -> Json.str(v))), indent = 1) + "\n").getBytes(java.nio.charset.StandardCharsets.UTF_8)) else Vector.empty
    def generatedDebt(digests: Map[String, String]) = (digests.keySet ++ expected.keySet).toVector.sorted.filter(k => digests.get(k) != expected.get(k))
      .map(k => s"generated $k: ${digests.getOrElse(k, "absent")}, generated.json has ${expected.getOrElse(k, "none")}")
    val judged = runsSeen.zipWithIndex.map { case ((observed, _, digests), i) => s"compile run ${i + 1}" -> Transcript.withDebt(observed, generatedDebt(digests)) }
    if !Lines.hold("lila", "compile", here, Shape.Compile, judged, runsSeen.head._2, mode, version, checked = !failed, staged) then failed = true
    val m = Measured(walls.result(), peaks.result(), failed)
    sessionPeak("compile", m)
    m


  def test(pin: Pin, here: Path, dir: Path, out: Path, teq: String, runs: Int, mode: Mode): Measured =
    val lock = exported(dir)
    val bin = binary(teq)
    describe(bin)
    val version = Sh(bin, "--version").timeout(30).check(false).run().out.trim
    val roots = Roots.of(dir, dir, out)
    val held = Vector.newBuilder[(String, Observation, String)]
    val (walls, peaks) = (Vector.newBuilder[Double], Vector.newBuilder[Option[Long]])
    val generatorsBefore = generatorState(dir)
    // The oracle's inventory, which the run is held to: without one nothing would be checked.
    val inventory = here.resolve("tests.json")
    if !Files.isRegularFile(inventory) then Log.die(s"no inventory at $inventory: record it with the oracle (oracle --record)")
    val expected = Tests.read(inventory)
    if expected.suites.isEmpty || expected.cases == 0 then Log.die(s"the inventory $inventory holds no suite or no case")
    var failed = false
    for i <- 1 to runs do
      val log = out.resolve(s"test-$i.log")
      val errLog = out.resolve(s"test-$i.err")
      // The daemon the run uses is the one it starts, which carries the run's token (`Reference.sampled`).
      Sh(bin, "stop").in(dir).timeout(120).check(false).run()
      Log.stamped(s"lila: teq test, run $i of $runs, its output $log, teq's stderr $errLog")
      val started = System.nanoTime()
      // The run's events (`--events`), each status as the framework reported it (an error apart from a failure, which
      // the transcript's `[fail]` does not tell apart), are its inventory; verbose, the transcript names every test with
      // its outcome, and holds each project's summary and each build that failed. Its two channels are collected apart,
      // teq's own lines (a build's diagnostics and count, the summaries) on stderr, the run's stream on stdout; the
      // outcomes are the events' (`Transcript`).
      val events = out.resolve(s"test-$i-events.txt")
      val diagnostics = out.resolve(s"test-$i.diagnostics")
      Files.deleteIfExists(events)
      Files.deleteIfExists(diagnostics)
      val (r, peak) = Reference.sampled(out, teqSeconds + 600)(mark => Sh(bin, "test", "--verbose", "--events", events.toString, "--diagnostics", diagnostics.toString).in(dir).withEnv(mark).stdout(log).stderr(errLog).timeout(teqSeconds).check(false).run())
      val wall = (System.nanoTime() - started) / 1e9
      Files.writeString(log, Transcript.mark(r.status), java.nio.file.StandardOpenOption.APPEND)
      val transcript = Files.readString(log)
      val errText = if Files.isRegularFile(errLog) then Files.readString(errLog) else ""
      // The summaries as the records' scan reads them, past the builds' diagnostics (`Transcript.scan`).
      val counts = Transcript.scan(errText, Transcript.recorded(diagnostics).fold(Vector.empty)(Transcript.shown), roots, test = true).summaries
        .map((_, c) => (c.getOrElse("suites", 0), c.getOrElse("passed", 0), c.getOrElse("failed", 0)))
      val (suites, passed, failedCases) = (counts.map(_._1).sum, counts.map(_._2).sum, counts.map(_._3).sum)
      val (shown, unmeasured) = measuredLine(s"test run $i", peak, wall)
      println(f"lila: test run $i: exit ${r.status}, $wall%.1f s, $shown, $suites suites, $passed passed, $failedCases failed (tests.json: ${expected.suites.size} suites, ${expected.cases} cases)")
      walls += wall
      peaks += peak
      if unmeasured then failed = true
      // The run held to deviations.json by the records' one judgment of a test run (`Deviations.judge`, the same for
      // every driver): the invocation's channels and events, the oracle's projects and cases.
      val eventText = Option.when(Files.isRegularFile(events))(Files.readString(events))
      val found = eventText.fold(Tests(Map.empty))(Tests.fromEvents)
      val differences = Tests.compare(expected, found)
      if differences.nonEmpty then
        println(s"lila: test run $i: ${differences.size} differences from tests.json")
        differences.take(40).foreach(d => println(s"  $d"))
      val evidence = TestEvidence(Vector(Invocation("teq test", r.status, transcript, errText, Transcript.recorded(diagnostics), eventText)), expected.heldSuites.map(_._1).distinct,
        expected.items, expected.heldSuites, owed = Locks.testUnits(lock), types = Locks.typesTests(lock))
      held += ((s"test run $i", Deviations.judge(evidence, roots), roots(Invocation.shown(transcript, errText))))
    Sh(bin, "stop").in(dir).timeout(120).check(false).run()
    tellGenerators(generatorsBefore, generatorState(dir))
    val seen = held.result()
    if !Lines.hold("lila", "test", here, Shape.Tests, seen.map((n, o, _) => n -> o), seen.head._3, mode, version, checked = !failed) then failed = true
    val m = Measured(walls.result(), peaks.result(), failed)
    sessionPeak("test", m)
    m

  def readySeconds: Double = Option(System.getenv("LILA_READY_SECONDS")).filter(_.nonEmpty).map(_.toDouble).getOrElse(600.0)

  // A service binary: the environment's, else the PATH's.
  private def service(variable: String, name: String): String =
    Option(System.getenv(variable)).filter(_.nonEmpty).orElse {
      Option(System.getenv("PATH")).toList.flatMap(_.split(':')).map(d => Paths.get(d, name)).find(Files.isExecutable(_)).map(_.toString)
    }.getOrElse(Log.die(s"no $name: set $variable or put it on the PATH"))

  // bash's /dev/tcp: `sh` may be dash, which has none and so would never see a port open.
  private def listening(port: Int): Boolean =
    Sh("bash", "-c", s"exec 3<>/dev/tcp/127.0.0.1/$port").timeout(5).check(false).run().ok

  private def waitPort(port: Int, seconds: Int, what: String): Unit =
    var left = seconds * 4
    while !listening(port) && left > 0 do
      Thread.sleep(250)
      left -= 1
    if !listening(port) then Log.die(s"lila: $what is not listening on $port")

  // A page fetched with curl: its status and its body's file.
  private def fetch(url: String, body: Path): Int =
    val r = Sh("curl", "-s", "-o", body.toString, "-w", "%{http_code}", "--max-time", "60", url).timeout(90).check(false).run()
    r.out.trim.toIntOption.getOrElse(0)

  def serve(pin: Pin, here: Path, dir: Path, out: Path, teq: String, runs: Int, mode: Mode): Measured =
    exported(dir)
    val bin = binary(teq)
    describe(bin)
    val version = Sh(bin, "--version").timeout(30).check(false).run().out.trim
    val roots = Roots.of(dir, dir, out)
    val held = Vector.newBuilder[(String, Observation, String)]
    val manifest = dir.resolve("public/compiled/manifest.json")
    if !Files.isRegularFile(manifest) || !Files.readString(manifest).contains("\"js\"") then
      Log.die(s"no built assets at $manifest: build lila's ui (ui/build) in the checkout first")
    val (mongod, redis) = (service("LILA_MONGOD", "mongod"), service("LILA_REDIS", "redis-server"))
    for port <- Seq(27017, 6379, 9663) if listening(port) do Log.die(s"lila: port $port is taken")
    val data = out.resolve("mongo-data")
    Sh.remove(data)
    Files.createDirectories(data)
    val mongo = Sh(mongod, "--dbpath", data.toString, "--port", "27017", "--bind_ip", "127.0.0.1", "--quiet").stdout(out.resolve("mongod.log")).mergeErr.timeout(runs * (readySeconds + 600) + 600).spawn()
    // Redis refuses a locale the machine lacks: the C locale.
    val cache = Sh(redis, "--port", "6379", "--bind", "127.0.0.1", "--save", "", "--appendonly", "no").withEnv("LC_ALL" -> "C").stdout(out.resolve("redis.log")).mergeErr.timeout(runs * (readySeconds + 600) + 600).spawn()
    waitPort(27017, 60, "MongoDB")
    waitPort(6379, 30, "Redis")
    var failed = false
    val (walls, peaks) = (Vector.newBuilder[Double], Vector.newBuilder[Option[Long]])
    val generatorsBefore = generatorState(dir)
    for i <- 1 to runs do
      val log = out.resolve(s"run-$i.log")
      Log.stamped(s"lila: teq run lila, run $i of $runs, its output $log")
      Sh(bin, "stop").in(dir).timeout(120).check(false).run()
      val ((observed, ready), peak) = Reference.sampled(out, teqSeconds + 600)(mark => serveOnce(bin, dir, out, log, mark, roots))
      val (shown, unmeasured) = measuredLine(s"run $i", peak, ready)
      println(f"lila: run $i: ${if observed.debt.isEmpty then "served" else s"${observed.debt.size} assertions not held"}, the home page in $ready%.1f s, $shown")
      walls += ready
      peaks += peak
      if unmeasured then failed = true
      held += ((s"run $i", observed, roots(Files.readString(log))))
    cache.kill()
    mongo.kill()
    tellGenerators(generatorsBefore, generatorState(dir))
    val seen = held.result()
    if !Lines.hold("lila", "run", here, Shape.Run, seen.map((n, o, _) => n -> o), seen.head._3, mode, version, checked = !failed) then failed = true
    val m = Measured(walls.result(), peaks.result(), failed)
    sessionPeak("run", m)
    m

  // The run's named assertions (run.json, bench/reference/records.scala), in the order a run checks them.
  val assertions = Vector("the home page within LILA_READY_SECONDS", "lila's title on the home page", "an asset named on the home page", "the asset served")

  // One `teq run lila`: the home page within LILA_READY_SECONDS, lila's title on it and its first asset served, each a
  // named assertion (`assert <name>: ok`, `FAIL <reason>`, `not reached`, printed); a run whose build failed in its own
  // errors blocked, every assertion a violation. The run's observation and its start-up time to the home page; the
  // server and teq's daemon stopped after (tests/support/lila_serve_verb.scala runs it over a serving program).
  def serveOnce(bin: String, dir: Path, out: Path, log: Path, mark: (String, String), roots: Roots): (Observation, Double) =
    val started = System.nanoTime()
    // The build's diagnostics as teq records them (`--diagnostics`), never read from the run's text.
    val diagnostics = out.resolve(log.getFileName.toString.stripSuffix(".log") + ".diagnostics")
    Files.deleteIfExists(diagnostics)
    val server = Sh(bin, "run", "lila", "--diagnostics", diagnostics.toString).in(dir).withEnv(mark).stdout(log).mergeErr.timeout(readySeconds + 600).spawn()
    val home = out.resolve("home.html")
    Files.deleteIfExists(home)
    var status = 0
    while status != 200 && server.alive && (System.nanoTime() - started) / 1e9 < readySeconds do
      Thread.sleep(1000)
      // lila redirects a host other than its domain (net.domain, localhost:9663).
      status = if listening(9663) then fetch("http://localhost:9663/", home) else 0
    val ready = (System.nanoTime() - started) / 1e9
    val outcomes = Vector.newBuilder[String]
    def failing(reason: String): Unit =
      val n = outcomes.result().size
      outcomes += s"FAIL $reason"
      for _ <- n + 1 until assertions.size do outcomes += "not reached"
    // How `teq run` ended, where it did before the home page answered: 1 is a build failing on its errors.
    val ended = Option.when(status != 200 && !server.alive)(server.await().status)
    if status != 200 && !server.alive then failing(s"teq run ended before the home page answered (its output $log)")
    else if status != 200 then failing(s"the home page answered $status within ${readySeconds.toInt} s")
    else
      outcomes += "ok"
      val page = Files.readString(home)
      outcomes += (if page.contains("<title>") && page.toLowerCase.contains("lichess") then "ok" else "FAIL the home page lacks lila's title")
      // The asset's path one token of the page's, so that the outcome naming it stays one line of the run's.
      "(?:href|src)=\"(?:http://localhost:9663)?(/assets/[^\"\\s\u0085\u2028\u2029]+)\"".r.findFirstMatchIn(page).map(_.group(1)) match
        case None => failing("the home page names no asset")
        case Some(a) =>
          outcomes += "ok"
          val got = fetch(s"http://localhost:9663$a", out.resolve("asset.bin"))
          val size = if Files.isRegularFile(out.resolve("asset.bin")) then Files.size(out.resolve("asset.bin")) else 0L
          outcomes += (if got != 200 || size == 0 then s"FAIL the asset $a answered $got with $size bytes" else "ok")
          if got == 200 && size > 0 then println(f"lila: the home page in $ready%.1f s, with lila's title and the asset $a ($size bytes)")
    Sh(bin, "stop").in(dir).timeout(120).check(false).run()
    server.kill()
    val shown = assertions.zip(outcomes.result()).map((a, o) => s"assert $a: $o")
    shown.foreach(println)
    (observeRun(status, ended, Files.readString(log), Transcript.recorded(diagnostics), shown, roots), ready)

  // A run's observation (`serveOnce`'s): the home page's status, how `teq run` ended where it did before the page
  // answered (None when it still ran at LILA_READY_SECONDS), its output and the assertions as the run checked them. A
  // run is blocked by its build only where `teq run` ended by exit 1 on that build's own errors, every assertion
  // unexecuted; a run still running at the deadline never became ready and fails the readiness assertion, whatever it
  // printed; another end of a run whose build printed errors (a signal, a crash) is no block a record holds. The
  // build's errors, counts and crashes come from its records and the transcript teq alone wrote: a run whose program
  // started (`Transcript.ran`) has its server's output in the transcript, which is never read for them.
  def observeRun(status: Int, ended: Option[Int], text: String, records: Option[String], shown: Vector[String], roots: Roots): Observation =
    val (errors, buildProblems) = RunEvidence.errors(text, records, roots)
    // The build failed where `teq run` ended before the home page answered with its build's own errors printed.
    val failedBuild = ended.filter(_ => status != 200 && errors.nonEmpty).map(e => (e, errors))
    Assertions.judge(RunEvidence(assertions, "teq run lila", failedBuild, shown.mkString("\n"), Transcript.crashes(text, records, roots) ++ buildProblems))

  // The three verbs in turn and the lines' admission: the median wall times summed against the 120 s the gate may
  // add, the session's peak against 12 GB. Whether every run passed, was measured and fits.
  def measure(pin: Pin, here: Path, dir: Path, out: Path, teq: String, runs: Int): Boolean =
    val c = compile(pin, here, dir, out, teq, runs, Mode.Judge)
    val t = test(pin, here, dir, out, teq, runs, Mode.Judge)
    val r = serve(pin, here, dir, out, teq, runs, Mode.Judge)
    val added = c.median + t.median + r.median
    val peak = Seq(c.peak, t.peak, r.peak)
    val session = if peak.exists(_.isEmpty) then None else peak.flatten.maxOption
    val fits = added <= 120.0 && session.exists(_ <= 12L * 1048576L)
    println(f"lila: measured: compile ${c.median}%.1f s, test ${t.median}%.1f s, run ${r.median}%.1f s: $added%.1f s added against 120 s; the session's peak ${Reference.gigabytes(session)} against 12 GB")
    val passed = !c.failed && !t.failed && !r.failed
    println(s"lila: the lines: ${if passed && fits then "within the budget on this machine" else if passed then "over the budget" else "not admissible: a run failed"}; they stay off until runs on both machines establish admission")
    passed && fits
