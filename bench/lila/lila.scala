//> using file ../../tools/script
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
      println(f"lila: oracle run $i: exit ${r.status}, $wall%.1f s, peak ${peak.fold("unmeasured")(k => f"${k / 1048576.0}%.2f GB")}, steps ${steps.mkString(" ")} s (load, compile, testFull, stage)")
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
    println(f"lila: exported ${Files.size(lock)} bytes in $wall%.1f s (peak ${peak.fold("unmeasured")(k => f"${k / 1048576.0}%.2f GB")}), copied to ${out.resolve("teq.lock")}")
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

  // The peak memory of teq's processes over the checkout while a command runs: sampled each second, the
  // resident set sizes of the processes whose command line names the binary summed (the client, the daemon,
  // its residents and test runners started from it).
  def sampled[A](bin: String, out: Path)(body: => A): (A, Option[Long]) =
    val file = out.resolve("peak.kb")
    Files.deleteIfExists(file)
    val script = "peak=0; while :; do n=$(ps -eo rss=,args= | awk -v b=\"$1\" 'index($0, b) && !/awk/ {s += $1} END {print s+0}'); [ \"$n\" -gt \"$peak\" ] && peak=$n && echo $peak > \"$2\"; sleep 1; done"
    val sampler = Sh("sh", "-c", script, "sampler", bin, file.toString).timeout(teqSeconds + 600).spawn()
    val a = try body finally sampler.kill()
    (a, if Files.isRegularFile(file) then Files.readString(file).trim.toLongOption else None)

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
      val (r, peak) = sampled(bin, out)(Sh(bin, "compile").in(dir).stdout(log).mergeErr.timeout(teqSeconds).check(false).run())
      val wall = (System.nanoTime() - started) / 1e9
      val errors = Files.readString(log).linesIterator.count(_.contains(" error: "))
      println(f"lila: compile run $i: exit ${r.status}, $wall%.1f s, peak ${peak.fold("unmeasured")(k => f"${k / 1048576.0}%.2f GB")}, $errors errors")
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
      val (r, peak) = sampled(bin, out)(Sh(bin, "test", "--verbose").in(dir).stdout(log).mergeErr.timeout(teqSeconds).check(false).run())
      val wall = (System.nanoTime() - started) / 1e9
      val transcript = Files.readString(log)
      val counts = transcript.linesIterator.flatMap(l => Tests.summaryLine.findFirstMatchIn(l)).map(m => (m.group(2).toInt, m.group(3).toInt, m.group(4).toInt)).toVector
      val (suites, passed, failedCases) = (counts.map(_._1).sum, counts.map(_._2).sum, counts.map(_._3).sum)
      println(f"lila: test run $i: exit ${r.status}, $wall%.1f s, peak ${peak.fold("unmeasured")(k => f"${k / 1048576.0}%.2f GB")}, $suites suites, $passed passed, $failedCases failed (tests.json: ${expected.suites.size} suites, ${expected.cases} cases)")
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

// The suites and cases of a test run: by suite, its project and its cases' names and outcomes.
final case class Suite(project: String, cases: Vector[(String, String)])
final case class Tests(suites: Map[String, Suite]):
  def cases: Int = suites.values.map(_.cases.size).sum

object Tests:
  // A test configuration's summary line: `<project>/test: N suites, P passed, F failed in ...`.
  val summaryLine = "^([^ ]+)/test: ([0-9]+) suites?, ([0-9]+) passed, ([0-9]+) failed".r

  def read(file: Path): Tests =
    if !Files.isRegularFile(file) then Tests(Map.empty)
    else
      val j = Json.parse(Files.readString(file))
      Tests(j("suites").members.map { (name, s) =>
        name -> Suite(s("project").str, s("cases").elements.map(c => c.str -> "passed"))
      }.toMap)

  // Every suite with its project and its cases' names, sorted, the counts first: what passed.
  def write(t: Tests): String =
    val suites = t.suites.toVector.sortBy(_._1).map { (name, s) =>
      name -> Json.obj("project" -> Json.str(s.project), "cases" -> Json.Arr(s.cases.map(_._1).sorted.map(Json.str)))
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

  // sbt's JUnit reports under the checkout's target directories, `<project>/test-reports/TEST-<suite>.xml`.
  def fromReports(dir: Path): Tests =
    val reports = Files.walk(dir.resolve("target")).toArray.toList.map(_.asInstanceOf[Path])
      .filter(p => p.getFileName.toString.startsWith("TEST-") && p.getFileName.toString.endsWith(".xml") && p.getParent.getFileName.toString == "test-reports")
    Tests(reports.map { p =>
      val text = Files.readString(p)
      val name = suiteLine.findFirstMatchIn(text).map(m => unescape(m.group(1))).getOrElse(p.getFileName.toString.stripPrefix("TEST-").stripSuffix(".xml"))
      val cases = caseTag.findAllMatchIn(text).map { m =>
        val body = Option(m.group(3)).getOrElse("")
        val outcome = if body.contains("<failure") then "failed" else if body.contains("<error") then "error" else if body.contains("<skipped") then "skipped" else "passed"
        unescape(nameAttr.findFirstMatchIn(" " + m.group(1)).map(_.group(1)).getOrElse("?")) -> outcome
      }.toVector
      name -> Suite(p.getParent.getParent.getFileName.toString, cases)
    }.toMap)

  // A `teq test --verbose` transcript as the run's inventory: each project's section ends with its summary
  // line (`<project>/test: N suites, ..`), and holds a line per test the run reports, its status
  // (`[success]`, `[skipped]`, `[ignored]`, `[canceled]`, `[pending]`) or `[fail]` before the suite and the test
  // (`task::events::report`). A test is matched to the inventory's suite
  // whose name prefixes it, another suite is the line's first word.
  def fromTranscript(transcript: String, expected: Tests): Tests =
    val known = expected.suites.keys.toVector.sortBy(-_.length)
    val outcome = "^\\[(success|skipped|ignored|canceled|pending|fail)\\] (.*)$".r
    val suites = scala.collection.mutable.LinkedHashMap.empty[String, (String, Vector[(String, String)])]
    var section = Vector.empty[(String, String, String)]
    for line <- transcript.linesIterator do
      summaryLine.findFirstMatchIn(line) match
        case Some(m) =>
          val project = m.group(1).stripSuffix("/test")
          for (suite, test, result) <- section do
            val (_, cases) = suites.getOrElse(suite, (project, Vector.empty))
            suites(suite) = (project, cases :+ (test -> result))
          section = Vector.empty
        case None =>
          line match
            case outcome(kind, named) if !named.startsWith("  ") && !named.startsWith(" at ") =>
              val suite = known.find(n => named == n || named.startsWith(n + " ")).getOrElse(named.takeWhile(_ != ' '))
              val rest = named.drop(suite.length).stripPrefix(" ")
              // A failure's line goes on with its throwable's first line after `: `.
              val test = if kind == "fail" then known.find(_ == suite).flatMap(s => expected.suites(s).cases.map(_._1).filter(c => rest == c || rest.startsWith(c + ": ")).maxByOption(_.length)).getOrElse(rest.takeWhile(_ != ':')) else rest
              val result = kind match
                case "success" => "passed"
                case "fail" => "failed"
                case _ => "skipped"
              section = section :+ (suite, test, result)
            case _ => ()
    Tests(suites.toMap.map((name, v) => name -> Suite(v._1, v._2)))

  // What the expected inventory has that the run lacks or did not pass: each a line; a run's suite or case
  // tests.json lacks is named too, since the pin moves with its inventory, and a suite run in another project.
  def compare(expected: Tests, found: Tests): Vector[String] =
    val out = Vector.newBuilder[String]
    if expected.suites.isEmpty then out += "tests.json holds no suite"
    if found.suites.isEmpty then out += "the run discovered no suite"
    for (name, s) <- expected.suites.toVector.sortBy(_._1) do
      found.suites.get(name) match
        case None => out += s"suite $name (${s.project}) did not run"
        case Some(f) =>
          if f.project != s.project then out += s"suite $name ran in project ${f.project}, tests.json's is ${s.project}"
          val outcomes = f.cases.toMap
          for (c, _) <- s.cases do
            outcomes.get(c) match
              case None => out += s"$name: case \"$c\" did not run"
              case Some("passed") => ()
              case Some(o) => out += s"$name: case \"$c\" $o"
          for (c, _) <- f.cases if !s.cases.exists(_._1 == c) do out += s"$name: case \"$c\" is not in tests.json"
    for name <- found.suites.keys.toVector.sorted if !expected.suites.contains(name) do out += s"suite $name is not in tests.json"
    out.result()
