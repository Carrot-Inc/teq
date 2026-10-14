//> using file ../../tools/script
//> using file ../reference/reference.scala
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
//           TEQ_PLUGIN_VERSION, by default integrations/sbt/plugin-version.txt's; overlay/LilaTeq.scala declares the
//           generators teq runs in the place of sbt's: the routers and the translations), the lock copied to <out>;
//           a lock recording a generator only sbt runs fails
// compile   `teq compile` of every project over the export (--teq <binary>, by default ./target/release/teq), the
//           daemon stopped and teq's class files removed before each run (--runs n), the generators' outputs kept:
//           each run's wall time and the peak memory of teq's processes (sampled), then the generated inputs'
//           digests against generated.json's (--record rewrites them)
// test      `teq test` of every project: its suites and cases counted against tests.json, failing on fewer or on a
//           failed case (--runs n)
// run       MongoDB and Redis (LILA_MONGOD and LILA_REDIS, else mongod and redis-server on the PATH) on lila's ports,
//           `teq run lila` over the export, the home page within LILA_READY_SECONDS (default 600): its content
//           (lila's title and an asset of the manifest) and that asset served; the start-up time to the home page.
//           lila's assets are built by its ui/build (public/compiled/manifest.json): a checkout without them fails
//
// Every verb checks that the checkout is at the pin and that the overlay's files are the bytes pin.json
// records: the pin, the oracle's results (tests.json) and the overlay move together. <out> holds the logs,
// sbt's caches and the lock and stays outside the repository. JAVA_HOME names the JDK (its major version
// must be pin.json's); sbt is the one on the PATH. Every command is bounded: LILA_SBT_SECONDS (default 900)
// for an sbt run, 300 for git.
import java.nio.file.{Files, Path, Paths}
import java.security.MessageDigest

@main def lila(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/lila/lila.scala <checkout|oracle|export|compile|test|run> <checkout> <out> [--runs n] [--record] [--teq <binary>]")
  val teq = args.teq("./target/release/teq")
  val runs = args.option("runs").map(_.toInt).getOrElse(1)
  val record = args.flag("record")
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
      Lila.atPin(pin, checkout)
      Lila.oracle(pin, here, checkout, out, runs, record)
    case "export" =>
      Lila.atPin(pin, checkout)
      Lila.exportLock(pin, here, checkout, out)
    case "compile" =>
      Lila.atPin(pin, checkout)
      Lila.compile(pin, here, checkout, out, teq, runs, record)
    case "test" =>
      Lila.atPin(pin, checkout)
      Lila.test(pin, here, checkout, out, teq, runs)
    case "run" =>
      Lila.atPin(pin, checkout)
      Lila.serve(pin, checkout, out, teq)
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
    val version = Option(System.getenv("TEQ_PLUGIN_VERSION")).filter(_.nonEmpty).getOrElse(Files.readString(Paths.get("integrations/sbt/plugin-version.txt")).trim)
    val overlay = here.resolve("overlay").toAbsolutePath.normalize
    val log = out.resolve("export.log")
    Log.stamped(s"lila: teqExportAll with sbt-teq $version and the overlay, sbt's log $log")
    val (r, wall, peak) = sbt(pin, dir, log,
      Seq(s"-Dteq.lila.overlay=$overlay", s"-Dteq.lila.scripts=${here.toAbsolutePath.normalize}", s"-Dteq.plugin.version=$version", s"-addPluginSbtFile=${overlay.resolve("plugins.sbt")}"), Seq("teqExportAll"))
    if r.status != 0 then
      println(Files.readString(log).linesIterator.toVector.takeRight(20).mkString("\n"))
      Log.die(s"lila: the export exits ${r.status}")
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

  def compile(pin: Pin, here: Path, dir: Path, out: Path, teq: String, runs: Int, record: Boolean): Unit =
    exported(dir)
    val bin = binary(teq)
    var failed = false
    for i <- 1 to runs do
      Sh(bin, "stop").in(dir).timeout(120).check(false).run()
      coldOutputs(dir)
      val log = out.resolve(s"compile-$i.log")
      Log.stamped(s"lila: teq compile, run $i of $runs, its output $log")
      val started = System.nanoTime()
      val (r, peak) = Reference.sampled(bin, out, teqSeconds + 600)(Sh(bin, "compile").in(dir).stdout(log).mergeErr.timeout(teqSeconds).check(false).run())
      val wall = (System.nanoTime() - started) / 1e9
      val errors = Files.readString(log).linesIterator.count(_.contains(" error: "))
      println(f"lila: compile run $i: exit ${r.status}, $wall%.1f s, peak ${Reference.gigabytes(peak)}, $errors errors")
      if r.status != 0 then failed = true
    Sh(bin, "stop").in(dir).timeout(120).check(false).run()
    val digests = generatedDigests(dir)
    if record then
      Files.writeString(here.resolve("generated.json"), Json.write(Json.Obj(digests.toVector.sortBy(_._1).map((k, v) => k -> Json.str(v))), indent = 1) + "\n")
      println(s"lila: recorded the digests of ${digests.size} generators' outputs in bench/lila/generated.json")
    else
      val recorded = Json.parse(Files.readString(here.resolve("generated.json"))).members.map((k, v) => k -> v.str).toMap
      for (k, v) <- digests.toVector.sortBy(_._1) if !recorded.get(k).contains(v) do
        println(s"FAIL lila: the generated $k differs from generated.json's")
        failed = true
    Script.exit(if failed then 1 else 0)


  def test(pin: Pin, here: Path, dir: Path, out: Path, teq: String, runs: Int): Unit =
    exported(dir)
    val bin = binary(teq)
    // The oracle's inventory, which the run is held to: without one nothing would be checked.
    val inventory = here.resolve("tests.json")
    if !Files.isRegularFile(inventory) then Log.die(s"no inventory at $inventory: record it with the oracle (oracle --record)")
    val expected = Tests.read(inventory)
    if expected.suites.isEmpty || expected.cases == 0 then Log.die(s"the inventory $inventory holds no suite or no case")
    var failed = false
    for i <- 1 to runs do
      val log = out.resolve(s"test-$i.log")
      Log.stamped(s"lila: teq test, run $i of $runs, its output $log")
      val started = System.nanoTime()
      // Verbose, the runner names every test with its outcome: the transcript is the run's inventory.
      val (r, peak) = Reference.sampled(bin, out, teqSeconds + 600)(Sh(bin, "test", "--verbose").in(dir).stdout(log).mergeErr.timeout(teqSeconds).check(false).run())
      val wall = (System.nanoTime() - started) / 1e9
      val transcript = Files.readString(log)
      val counts = transcript.linesIterator.flatMap(l => Tests.summaryLine.findFirstMatchIn(l)).map(m => (m.group(2).toInt, m.group(3).toInt, m.group(4).toInt)).toVector
      val (suites, passed, failedCases) = (counts.map(_._1).sum, counts.map(_._2).sum, counts.map(_._3).sum)
      println(f"lila: test run $i: exit ${r.status}, $wall%.1f s, peak ${Reference.gigabytes(peak)}, $suites suites, $passed passed, $failedCases failed (tests.json: ${expected.suites.size} suites, ${expected.cases} cases)")
      val differences = Tests.compare(expected, Tests.fromTranscript(transcript, expected))
      if r.status != 0 || differences.nonEmpty then
        println(s"FAIL lila: test run $i: the run is not tests.json's: exit ${r.status}, ${differences.size} differences")
        differences.take(40).foreach(d => println(s"  $d"))
        failed = true
    Sh(bin, "stop").in(dir).timeout(120).check(false).run()
    Script.exit(if failed then 1 else 0)

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

  def serve(pin: Pin, dir: Path, out: Path, teq: String): Unit =
    exported(dir)
    val bin = binary(teq)
    val manifest = dir.resolve("public/compiled/manifest.json")
    if !Files.isRegularFile(manifest) || !Files.readString(manifest).contains("\"js\"") then
      Log.die(s"no built assets at $manifest: build lila's ui (ui/build) in the checkout first")
    val (mongod, redis) = (service("LILA_MONGOD", "mongod"), service("LILA_REDIS", "redis-server"))
    for port <- Seq(27017, 6379, 9663) if listening(port) do Log.die(s"lila: port $port is taken")
    val data = out.resolve("mongo-data")
    Sh.remove(data)
    Files.createDirectories(data)
    val mongo = Sh(mongod, "--dbpath", data.toString, "--port", "27017", "--bind_ip", "127.0.0.1", "--quiet").stdout(out.resolve("mongod.log")).mergeErr.timeout(readySeconds + 1200).spawn()
    // Redis refuses a locale the machine lacks: the C locale.
    val cache = Sh(redis, "--port", "6379", "--bind", "127.0.0.1", "--save", "", "--appendonly", "no").withEnv("LC_ALL" -> "C").stdout(out.resolve("redis.log")).mergeErr.timeout(readySeconds + 1200).spawn()
    waitPort(27017, 60, "MongoDB")
    waitPort(6379, 30, "Redis")
    val log = out.resolve("run.log")
    Log.stamped(s"lila: teq run lila, its output $log")
    val started = System.nanoTime()
    val server = Sh(bin, "run", "lila").in(dir).stdout(log).mergeErr.timeout(readySeconds + 600).spawn()
    val home = out.resolve("home.html")
    var status = 0
    while status != 200 && server.alive && (System.nanoTime() - started) / 1e9 < readySeconds do
      Thread.sleep(1000)
      // lila redirects a host other than its domain (net.domain, localhost:9663).
      status = if listening(9663) then fetch("http://localhost:9663/", home) else 0
    val ready = (System.nanoTime() - started) / 1e9
    var failed = status != 200
    if failed && !server.alive then println(s"FAIL lila: teq run ended before the home page answered (its output $log)")
    else if failed then println(s"FAIL lila: the home page answered $status within ${readySeconds.toInt} s")
    else
      val page = Files.readString(home)
      val asset = "(?:href|src)=\"(?:http://localhost:9663)?(/assets/[^\"]+)\"".r.findFirstMatchIn(page).map(_.group(1))
      if !page.contains("<title>") || !page.toLowerCase.contains("lichess") then
        println("FAIL lila: the home page lacks lila's title")
        failed = true
      asset match
        case None =>
          println("FAIL lila: the home page names no asset")
          failed = true
        case Some(a) =>
          val got = fetch(s"http://localhost:9663$a", out.resolve("asset.bin"))
          val size = if Files.isRegularFile(out.resolve("asset.bin")) then Files.size(out.resolve("asset.bin")) else 0L
          if got != 200 || size == 0 then
            println(s"FAIL lila: the asset $a answered $got with $size bytes")
            failed = true
          else println(f"lila: the home page in $ready%.1f s, with lila's title and the asset $a ($size bytes)")
    Sh(bin, "stop").in(dir).timeout(120).check(false).run()
    server.kill()
    cache.kill()
    mongo.kill()
    Script.exit(if failed then 1 else 0)
