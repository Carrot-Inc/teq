//> using file ../tools/script
//> using file reference/records.scala
// The public reference projects built by Mill, Tyrian and Indigo (github.com/PurpleKingdomGames), as lila is for sbt
// (bench/lila/lila.scala): the pinned revision checked out with the reviewed overlay, built and tested under scalac
// 3.8.4 by its own Mill build (the oracle), exported with mill-teq, then compiled and tested by teq against the
// oracle's results. Run from the repository's root:
//
//   ./teq interp bench/mill-reference.scala -- <tyrian|indigo> <verb> <checkout> <out> [options]
//
// bench/<project>/ holds pin.json (the repository, the commit, the JDK, the Mill the build names, the build's
// directory in the checkout, the oracle's steps, the overlay's digests and, for Indigo, the bootstrap of its own
// Mill plugin), tests.json (the oracle's inventory), generated.json (the generated inputs' digests) and overlay/.
//
// checkout  clones the repository into <checkout> (pin.json's `shallow`: the commit alone) or fetches the pin into an
//           existing clone, checks the commit out detached and applies the overlay: overlay/build.mill.patch with
//           `git apply` (Scala 3.8.4 and no fatal warnings, the oracle's and teq's build alike), teq.mill.json into the
//           build's directory and the overlay's other files into its teq-overlay/ (the generators teq runs in the
//           place of the build's own)
// oracle    the bootstrap first where pin.json has one (Indigo's mill-indigo published into <out>/ivy by the plugin
//           build of the pinned commit's tree written to the bootstrap's `root`, each artifact pin.json names held to
//           its entries' digest), then the build's CI steps (pin.json's `steps`) under Mill with
//           JAVA_HOME's JDK, each timed (--runs n); the inventory read from Mill's out/: each module's scalac warnings
//           (out/<module>/compile.log) and each test module's cases with their outcomes (out/<module>/test/
//           testForked.json, the suites from discoveredTestClasses.json), against tests.json (--record rewrites it)
// export    mill-teq built from integrations/mill and published under a version of the run's own into <out>/maven,
//           imported by the build's command line (`mill --import`, no edit of the build), the lock copied to <out>; a
//           lock recording a generator only Mill runs fails
// compile   `teq compile` of every module over the export (--teq <binary>), teq's outputs but the lock removed before
//           each run (--runs n): each run's wall time, the peak memory of teq's processes, its errors and warnings by
//           module, each run held to bench/<project>/errors.txt (bench/reference/records.scala: every configuration
//           compiled once, the run complete, its errors within the record's, a new one failing, one gone listed;
//           --record writes a passing run's improvement, --baseline the first record, --migrate <reason> one over new
//           inputs), then the generated inputs' digests against generated.json (--baseline and --migrate rewrite
//           them from a run that is evidence, a run that is not recording nothing): each configuration's generated
//           directory and each file the overlay's generators declare they write besides (Indigo's font sheets in its
//           assets), then each kind deleted apart and compiled again to the same digests, each of those compiles held
//           to the record too
// test      the export's modules, the projects of the lock made at the pin (its settings' digest the pin's), each
//           with a test configuration asked for its suites (`teq test --list`) and run where it has one (`teq test
//           --verbose`), whatever tests.json holds: the modules, the suites listed and run and the cases against
//           tests.json, identity for identity, each case's statuses as many times as each side reports it, every
//           departure held to deviations.json (bench/reference/records.scala: a module whose own build failed
//           blocked, its cases departures of its; --record writes a passing run's improvement, --baseline the first
//           record, --migrate <reason> one over a new pin or inventory) (--runs n)
// run       (its named assertions held to run.json, bench/reference/records.scala: an assertion the run does not hold a
//           violation by its name, a build that failed blocking every one; --record, --baseline, --migrate as compile's)
//           the module pin.json's `run` names built by `teq build` over the export, its output served as out/ beside the
//           page it names, the harness's files of bench/<project>/run/ and the assets it lists, and opened in headless
//           Chrome by bench/smoke.mjs, whose scenario drives the page (Tyrian's counter clicked, Indigo's frame drawn
//           and its scene switched) and fails on a browser error or its time running out
//
// Every verb checks that the checkout is at the pin with exactly the overlay applied, the files the overlay's generators
// declare they write left to the compile verb, and that the overlay's files are the bytes pin.json records: the pin,
// tests.json and the overlay move together. <out> holds the logs, Mill's and
// coursier's work and the lock, and stays outside the repository. JAVA_HOME names the JDK (its major version must be
// pin.json's). Every command is bounded: MILL_REFERENCE_SECONDS (default 1500) for a Mill run, 300 for git,
// MILL_REFERENCE_TEQ_SECONDS (default 1800) for teq, whose processes are killed past MILL_REFERENCE_MEMORY_GB
// (default 16), failing the verb.
import java.nio.file.{Files, Path, Paths, StandardCopyOption}
import java.security.MessageDigest

@main def millReference(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/mill-reference.scala <tyrian|indigo> <checkout|oracle|export|compile|test|run> <checkout> <out> [--runs n] [--record | --baseline | --migrate <reason>] [--teq <binary>]")
  val teq = args.teq("./target/release/teq")
  val runs = args.option("runs").map(_.toInt).getOrElse(1)
  val mode = Mode.of(args)
  val record = mode == Mode.Record
  val positionals = args.exactly(4)
  val (project, verb, checkoutArg, outArg) = (positionals(0), positionals(1), positionals(2), positionals(3))
  if project != "tyrian" && project != "indigo" then args.fail(s"no reference project $project")
  val here = Paths.get("bench", project)
  if !Files.isRegularFile(here.resolve("pin.json")) then Log.die("bench/mill-reference.scala runs from the repository's root")
  val pin = Pin.read(project, here)
  val checkout = Paths.get(checkoutArg).toAbsolutePath.normalize
  val out = Paths.get(outArg).toAbsolutePath.normalize
  if out.startsWith(Paths.get("").toAbsolutePath) then Log.die(s"$out is inside the repository; the logs and caches stay outside it")
  Files.createDirectories(out)
  pin.checkOverlay()
  verb match
    case "checkout" => Ref.checkout(pin, checkout)
    case "oracle" =>
      if mode != Mode.Judge && mode != Mode.Record then args.fail("the oracle records its inventory with --record alone")
      Ref.atPin(pin, checkout)
      Ref.oracle(pin, checkout, out, runs, record)
    case "export" =>
      Ref.atPin(pin, checkout)
      Ref.exportLock(pin, checkout, out)
    case "compile" =>
      Ref.atPin(pin, checkout)
      Ref.compile(pin, checkout, out, teq, runs, mode)
    case "test" =>
      Ref.atPin(pin, checkout)
      Ref.test(pin, checkout, out, teq, runs, mode)
    case "run" =>
      Ref.atPin(pin, checkout)
      Ref.run(pin, checkout, out, teq, mode)
    case other => args.fail(s"no verb $other")
}

// bench/<project>/pin.json: the revision, its toolchain, the oracle's steps, the overlay's digests and the bootstrap.
final case class Pin(name: String, here: Path, json: Json):
  def repository: String = json("repository").str
  def commit: String = json("commit").str
  def shallow: Boolean = json.get("shallow").exists(_.bool)
  // The build's directory in the checkout: Indigo's engine build is indigo/.
  def build: String = json("build").str
  def java: Int = json("java").int
  def steps: Vector[String] = json("steps").elements.map(_.str)
  def overlay: Map[String, String] = json("overlay").members.map((f, d) => f -> d.str).toMap
  def bootstrap: Option[Json] = json.get("bootstrap")
  def overlayDir: Path = here.resolve("overlay")
  // The files the overlay's generators write besides their generated directories, by their paths in the build: the
  // build's products, as Mill's own generators write them (Indigo's font sheets into its assets).
  def generatorOutputs: Vector[String] =
    val settings = overlayDir.resolve("teq.mill.json")
    if !Files.isRegularFile(settings) then Vector.empty
    else
      val modules = Json.parse(Files.readString(settings)).get("modules").fold(Vector.empty[(String, Json)])(_.members.toVector)
      modules.flatMap((_, m) => m.get("generators").fold(Vector.empty[Json])(_.elements))
        .flatMap(g => g.get("outputs").fold(Vector.empty[String])(_.elements.map(_.str))).distinct.sorted

  // The overlay's files are the bytes pin.json records, and no other file is in its directory.
  def checkOverlay(): Unit =
    val present = Files.list(overlayDir).toArray.toList.map(_.asInstanceOf[Path].getFileName.toString).sorted
    if present != overlay.keys.toList.sorted then
      Log.die(s"bench/$name/overlay holds ${present.mkString(", ")}, where pin.json records ${overlay.keys.toList.sorted.mkString(", ")}")
    for (f, digest) <- overlay do
      val actual = Pin.sha256(Files.readAllBytes(overlayDir.resolve(f)))
      if actual != digest then Log.die(s"bench/$name/overlay/$f is $actual, where pin.json records $digest: the pin, tests.json and the overlay move together")

object Pin:
  def read(name: String, here: Path): Pin = Pin(name, here, Json.parse(Files.readString(here.resolve("pin.json"))))
  def sha256(bytes: Array[Byte]): String =
    MessageDigest.getInstance("SHA-256").digest(bytes).map(b => f"${b & 0xff}%02x").mkString

object Ref:
  val Patch = "build.mill.patch"
  val Settings = "teq.mill.json"
  val Scripts = "teq-overlay"

  private def git(dir: Path, a: String*): Sh.Result = Sh(("git" +: "-C" +: dir.toString +: a)*).timeout(300).run()

  def millSeconds: Double = Option(System.getenv("MILL_REFERENCE_SECONDS")).filter(_.nonEmpty).map(_.toDouble).getOrElse(1500.0)
  def teqSeconds: Double = Option(System.getenv("MILL_REFERENCE_TEQ_SECONDS")).filter(_.nonEmpty).map(_.toDouble).getOrElse(1800.0)

  // Where the overlay's files go in the checkout: the patch is git's, the settings at the build's root, the scripts
  // under its teq-overlay/.
  def placed(pin: Pin, dir: Path, file: String): Path =
    if file == Settings then dir.resolve(pin.build).resolve(file).normalize
    else dir.resolve(pin.build).resolve(Scripts).resolve(file).normalize

  def checkout(pin: Pin, dir: Path): Unit =
    if !Files.isDirectory(dir.resolve(".git")) then
      Files.createDirectories(dir)
      Log.stamped(s"${pin.name}: cloning ${pin.repository} into $dir")
      git(dir, "init", "--quiet")
      git(dir, "remote", "add", "origin", pin.repository)
    val has = Sh("git", "-C", dir.toString, "cat-file", "-e", pin.commit + "^{commit}").timeout(60).check(false).run().ok
    if !has then
      Log.stamped(s"${pin.name}: fetching ${pin.commit}")
      if pin.shallow then git(dir, "fetch", "--quiet", "--depth", "1", "origin", pin.commit)
      else git(dir, "fetch", "--quiet", "origin", pin.commit)
    git(dir, "checkout", "--quiet", "--force", "--detach", pin.commit)
    git(dir, "apply", pin.overlayDir.resolve(Patch).toAbsolutePath.toString)
    for f <- pin.overlay.keys if f != Patch do
      val to = placed(pin, dir, f)
      Files.createDirectories(to.getParent)
      Files.copy(pin.overlayDir.resolve(f), to, StandardCopyOption.REPLACE_EXISTING)
    atPin(pin, dir)
    Log.stamped(s"${pin.name}: $dir at ${pin.commit} with the overlay")

  // The checkout is at the pin with exactly the overlay's patch applied to its tracked files, and the overlay's other
  // files in place; the files the overlay's generators declare they write are the build's products, whose bytes the
  // compile verb holds to generated.json.
  def atPin(pin: Pin, dir: Path): Unit =
    if !Files.isDirectory(dir.resolve(".git")) then Log.die(s"no checkout at $dir: run the verb checkout first")
    val head = git(dir, "rev-parse", "HEAD").out.trim
    if head != pin.commit then Log.die(s"$dir is at $head, where pin.json names ${pin.commit}")
    val products = pin.generatorOutputs.map(o => s":(exclude)${Paths.get(pin.build).resolve(o).normalize}")
    val diff = git(dir, (Seq("diff", "--no-color", "--no-ext-diff", "--full-index", "--", ".") ++ products)*).out
    val patch = Files.readString(pin.overlayDir.resolve(Patch))
    if normalized(diff) != normalized(patch) then Log.die(s"$dir's tracked files are not the pin's with overlay/$Patch applied: run the verb checkout again")
    for f <- pin.overlay.keys if f != Patch do
      val at = placed(pin, dir, f)
      if !Files.isRegularFile(at) || Pin.sha256(Files.readAllBytes(at)) != pin.overlay(f) then Log.die(s"$at is not overlay/$f: run the verb checkout again")

  // A diff's lines without its `index` lines, which name the blobs by an abbreviation git chooses.
  private def normalized(diff: String): String =
    diff.linesIterator.filterNot(_.startsWith("index ")).mkString("\n").trim

  // JAVA_HOME's JDK, whose major version is the pin's.
  def javaHome(pin: Pin): Path =
    val home = Option(System.getenv("JAVA_HOME")).filter(_.nonEmpty).map(Paths.get(_)).getOrElse(Log.die(s"JAVA_HOME names no JDK; ${pin.name}'s pin builds under Java ${pin.java}"))
    val r = Sh(home.resolve("bin/java").toString, "-version").timeout(60).run()
    val version = "version \"([0-9]+)".r.findFirstMatchIn(r.err + r.out).map(_.group(1).toInt)
    if !version.contains(pin.java) then Log.die(s"JAVA_HOME's java is ${version.getOrElse("of no version")}, where pin.json names ${pin.java}")
    home

  // The ivy repository Indigo's bootstrap publishes its own Mill plugin into, ahead of Maven Central.
  def repositories(out: Path, extra: Seq[String] = Nil): String =
    val ivy = out.resolve("ivy")
    (Seq(s"ivy:file:$ivy/[organisation]/[module]/[revision]/[type]s/[artifact](-[classifier]).[ext]", "central") ++ extra).mkString("|")

  // The build's Mill (its ./mill launcher, which runs the Mill the build names) in the build's directory, in the
  // foreground with no server left behind, under /usr/bin/time for the peak memory.
  def mill(pin: Pin, dir: Path, out: Path, log: Path, args: Seq[String], extraRepos: Seq[String] = Nil): (Sh.Result, Double, Option[Long]) =
    val home = javaHome(pin)
    val time = Paths.get("/usr/bin/time")
    val timed = if Files.isExecutable(time) then Seq(time.toString, "-v") else Nil
    val cmd = timed ++ Seq("./mill", "--no-server", "--disable-ticker") ++ args
    val started = System.nanoTime()
    val r = Sh(cmd*).in(dir).withEnv("JAVA_HOME" -> home.toString, "PATH" -> s"$home/bin:${System.getenv("PATH")}",
      "JAVA_OPTS" -> "-Xmx4G -Xss8m -XX:+UseG1GC", "COURSIER_REPOSITORIES" -> repositories(out, extraRepos))
      .stdout(log).mergeErr.timeout(millSeconds).check(false).run()
    val wall = (System.nanoTime() - started) / 1e9
    val peak = "Maximum resident set size \\(kbytes\\): ([0-9]+)".r.findFirstMatchIn(Files.readString(log)).map(_.group(1).toLong)
    (r, wall, peak)

  // Indigo's engine build imports its own Mill plugin, which its plugin build publishes into <out>/ivy. The plugin's
  // class files and TASTy name its sources by their absolute paths, so it is built from the commit's tree written to
  // pin.json's bootstrap `root`, the one path on every machine, afresh each time: each artifact the pin names is held
  // to its digest there, that of its entries' names and bytes (`entriesDigest`; a jar's own timestamps are its build's).
  def bootstrap(pin: Pin, dir: Path, out: Path): Unit =
    for b <- pin.bootstrap do
      val root = Paths.get(b("root").str)
      Sh.remove(root)
      Files.createDirectories(root)
      val tree = out.resolve("bootstrap-tree.tar")
      git(dir, "archive", "--format=tar", "-o", tree.toString, pin.commit)
      Sh("tar", "-x", "-f", tree.toString, "-C", root.toString).timeout(300).run()
      Files.delete(tree)
      val buildDir = root.resolve(b("dir").str)
      val log = out.resolve("bootstrap.log")
      Log.stamped(s"${pin.name}: bootstrap ${b("task").str} in $buildDir, Mill's log $log")
      val (r, wall, _) = mill(pin, buildDir, out, log, Seq(b("task").str, "--localIvyRepo", out.resolve("ivy").toString, "--doc", "false", "--transitive", "true"))
      if r.status != 0 then
        println(Files.readString(log).linesIterator.toVector.takeRight(20).mkString("\n"))
        Log.die(s"${pin.name}: the bootstrap exits ${r.status}: ${Log.cause(log)} (Mill's log $log)")
      val published = b("artifacts").members.map { (artifact, digest) =>
        val jar = out.resolve("ivy").resolve(artifact)
        if !Files.isRegularFile(jar) then Log.die(s"${pin.name}: the bootstrap did not publish $artifact, which pin.json names")
        val actual = Bootstrap.entriesDigest(jar)
        println(s"  $artifact $actual")
        (artifact, digest.str, actual)
      }
      for (artifact, recorded, actual) <- published if actual != recorded do
        Log.die(s"${pin.name}: the bootstrap published $artifact with the entries' digest $actual, where pin.json records $recorded")
      println(f"${pin.name}: bootstrap in $wall%.1f s, the ${b("artifacts").members.size} artifacts pin.json names published with its digests")

  def oracle(pin: Pin, dir: Path, out: Path, runs: Int, record: Boolean): Unit =
    // Read where a run is compared with it: a recording replaces it, whatever form it has.
    lazy val expected = Inventory.read(pin.here.resolve("tests.json"))
    bootstrap(pin, dir, out)
    val buildDir = dir.resolve(pin.build)
    var failed = false
    for i <- 1 to runs do
      val walls = Vector.newBuilder[String]
      var peakMax = 0L
      var status = 0
      for (step, k) <- pin.steps.zipWithIndex if status == 0 do
        val log = out.resolve(s"oracle-$i-step-$k.log")
        Log.stamped(s"${pin.name}: oracle run $i of $runs, `mill $step`, Mill's log $log")
        val (r, wall, peak) = mill(pin, buildDir, out, log, step.split(' ').toSeq)
        walls += f"$wall%.1f"
        peakMax = math.max(peakMax, peak.getOrElse(0L))
        status = r.status
      val found = Inventory.fromMill(buildDir)
      Files.writeString(out.resolve(s"oracle-$i-tests.json"), Inventory.write(found))
      println(f"${pin.name}: oracle run $i: exit $status, steps ${walls.result().mkString(" ")} s (${pin.steps.mkString(", ")}), peak ${peakMax / 1048576.0}%.2f GB, ${found.modules.size} modules, ${found.cases} cases, ${found.warnings} warnings")
      if status != 0 then
        println(s"FAIL ${pin.name}: oracle run $i: Mill exits $status")
        failed = true
      else if record && i == runs then
        Files.writeString(pin.here.resolve("tests.json"), Inventory.write(found))
        println(s"${pin.name}: recorded ${found.modules.size} modules, ${found.suites} suites, ${found.cases} cases in bench/${pin.name}/tests.json")
      else
        val problems = Inventory.compare(expected, found, warnings = true)
        problems.foreach(p => println(s"FAIL ${pin.name}: oracle run $i: $p"))
        if problems.nonEmpty then failed = true
    Script.exit(if failed then 1 else 0)

  // The mill-teq of this checkout, published under the run's own version into <out>/maven, which the export imports:
  // built by the reference build's own Mill launcher, which runs the Mill integrations/mill/build.mill names.
  def publishPlugin(pin: Pin, dir: Path, out: Path): String =
    val version = s"0.0.0-${pin.name}-reference-SNAPSHOT"
    val maven = out.resolve("maven")
    val log = out.resolve("plugin.log")
    val home = javaHome(pin)
    Log.stamped(s"${pin.name}: mill-teq published as $version into $maven, its log $log")
    val launcher = dir.resolve(pin.build).resolve("mill").toString
    val r = Sh(launcher, "--no-server", "--disable-ticker", "publishM2Local", "--m2RepoPath", maven.toString)
      .in(Paths.get("integrations/mill").toAbsolutePath).withEnv("JAVA_HOME" -> home.toString, "PATH" -> s"$home/bin:${System.getenv("PATH")}", "TEQ_MILL_PLUGIN_VERSION" -> version)
      .stdout(log).mergeErr.timeout(millSeconds).check(false).run()
    if r.status != 0 then
      println(Files.readString(log).linesIterator.toVector.takeRight(20).mkString("\n"))
      Log.die(s"${pin.name}: mill-teq did not build (exit ${r.status}): ${Log.cause(log)} (its log $log)")
    version

  def exportLock(pin: Pin, dir: Path, out: Path): Unit =
    val version = publishPlugin(pin, dir, out)
    bootstrap(pin, dir, out)
    val buildDir = dir.resolve(pin.build)
    val log = out.resolve("export.log")
    Log.stamped(s"${pin.name}: mill-teq's teqExportAll with the overlay, Mill's log $log")
    val (r, wall, peak) = mill(pin, buildDir, out, log, Seq("--import", s"mvn:build.teq::mill-teq::$version", "dev.teq.mill.Teq/teqExportAll"), Seq(s"file://${out.resolve("maven")}"))
    if r.status != 0 then
      println(Files.readString(log).linesIterator.toVector.takeRight(20).mkString("\n"))
      Log.die(s"${pin.name}: the export exits ${r.status}: ${Log.cause(log)} (Mill's log $log)")
    val lock = buildDir.resolve("target/teq/teq.lock")
    Files.copy(lock, out.resolve("teq.lock"), StandardCopyOption.REPLACE_EXISTING)
    println(f"${pin.name}: exported ${Files.size(lock)} bytes in $wall%.1f s (peak ${peak.fold("unmeasured")(k => f"${k / 1048576.0}%.2f GB")}), copied to ${out.resolve("teq.lock")}")
    val refused = Files.readString(lock).linesIterator.count(_.trim == "kind: mill")
    if refused > 0 then Log.die(s"${pin.name}: the lock records $refused generators only Mill runs")

  // The export's lock in the build, which records no generator only Mill runs and, where the pin has the settings
  // file, the pin's settings among its inputs: the export made at the pin.
  def exported(pin: Pin, dir: Path): Path =
    val lock = dir.resolve(pin.build).resolve("target/teq/teq.lock")
    if !Files.isRegularFile(lock) then Log.die(s"no export at $lock: run the verb export first")
    if Files.readString(lock).linesIterator.exists(_.trim == "kind: mill") then Log.die(s"$lock records a generator only Mill runs: export again with the overlay")
    for digest <- pin.overlay.get(Settings) do
      val recorded = Export.input(lock, Settings)
      if !recorded.contains(digest) then
        Log.die(s"$lock records $Settings as ${recorded.getOrElse("absent")}, where pin.json has $digest: export again at the pin")
    lock

  def binary(teq: String): String =
    val bin = Paths.get(teq).toAbsolutePath.normalize
    if !Files.isExecutable(bin) then Log.die(s"no teq binary at $bin (--teq)")
    bin.toString

  // The peak memory of teq's processes while a command runs: sampled five times a second, the resident set sizes of
  // the processes whose command line names the binary summed (the client, the daemon, its residents, test runners).
  // Past MILL_REFERENCE_MEMORY_GB (default 16) they are killed, which fails the verb: a machine of 22 GB freezes
  // before its kernel's OOM killer acts (Indigo's whole compile at eight workers reached 22.2 GB).
  def sampled[A](name: String, bin: String, out: Path)(body: => A): (A, Option[Long]) =
    val file = out.resolve("peak.kb")
    val killed = out.resolve("killed.kb")
    Files.deleteIfExists(file)
    Files.deleteIfExists(killed)
    val limitKb = (memoryGb * 1048576).toLong
    // The driver runs the binary too (`teq interp`): the sampler's ancestors are left out of both. The sampler ends
    // with its parent, the driver, however that ends.
    val script = "a=$$; up=\" \"; while [ \"$a\" -gt 1 ]; do up=\"$up$a \"; a=$(ps -o ppid= -p $a | tr -d ' '); done; peak=0; while kill -0 $PPID 2> /dev/null; do n=$(ps -eo pid=,rss=,args= | awk -v b=\"$1\" -v u=\"$up\" 'index($0, b) && !/awk/ && !index(u, \" \" $1 \" \") {s += $2} END {print s+0}'); [ \"$n\" -gt \"$peak\" ] && peak=$n && echo $peak > \"$2\"; " +
      "if [ \"$n\" -gt \"$3\" ]; then echo $n > \"$4\"; ps -eo pid=,args= | awk -v b=\"$1\" -v u=\"$up\" 'index($0, b) && !/awk/ && !index(u, \" \" $1 \" \") {print $1}' | xargs -r kill -9; fi; sleep 0.2; done"
    val sampler = Sh("sh", "-c", script, "sampler", bin, file.toString, limitKb.toString, killed.toString).timeout(teqSeconds + 600).spawn()
    val a = try body finally sampler.kill()
    if Files.isRegularFile(killed) then
      println(f"FAIL $name: teq's processes reached ${Files.readString(killed).trim.toLong / 1048576.0}%.2f GB, past MILL_REFERENCE_MEMORY_GB=$memoryGb%.1f: killed")
      Script.exit(1)
    (a, if Files.isRegularFile(file) then Files.readString(file).trim.toLongOption else None)

  def memoryGb: Double = Option(System.getenv("MILL_REFERENCE_MEMORY_GB")).filter(_.nonEmpty).map(_.toDouble).getOrElse(16.0)

  // teq's outputs of the build but the lock and the generators' records: what a cold compile writes again, the
  // generated sources included (regenerated from the overlay's commands).
  def coldOutputs(buildDir: Path): Unit =
    val teqDir = buildDir.resolve("target/teq")
    if Files.isDirectory(teqDir) then
      for entry <- Files.list(teqDir).toArray.toList.map(_.asInstanceOf[Path]) if Files.isDirectory(entry) && entry.getFileName.toString != "generators" do
        Sh.remove(entry)

  // The configurations' generated directories: each src_managed under teq's outputs.
  def managedRoots(buildDir: Path): List[Path] =
    val teqDir = buildDir.resolve("target/teq")
    if !Files.isDirectory(teqDir) then Nil else
      Files.walk(teqDir).toArray.toList.map(_.asInstanceOf[Path]).filter(p => Files.isDirectory(p) && p.getFileName.toString == "src_managed")

  // The digests of the generated inputs: every file under each configuration's src_managed, by its path, sorted, and
  // each file the overlay's generators declare they write besides, by its path in the build.
  def generatedDigests(pin: Pin, buildDir: Path): Map[String, String] =
    val managed = managedRoots(buildDir).map { root =>
      val files = Files.walk(root).toArray.toList.map(_.asInstanceOf[Path]).filter(Files.isRegularFile(_)).sortBy(_.toString)
      val listing = files.map(f => s"${root.relativize(f)} ${Pin.sha256(Files.readAllBytes(f))}\n").mkString
      buildDir.relativize(root).toString -> Pin.sha256(listing.getBytes(java.nio.charset.StandardCharsets.UTF_8))
    }
    val products = pin.generatorOutputs.map { o =>
      val f = buildDir.resolve(o)
      o -> (if Files.isRegularFile(f) then Pin.sha256(Files.readAllBytes(f)) else "absent")
    }
    (managed ++ products).toMap

  // The generated inputs' differences from generated.json's, each a line.
  def digestDifferences(recorded: Map[String, String], digests: Map[String, String]): Vector[String] =
    recorded.toVector.sortBy(_._1).filter((k, v) => !digests.get(k).contains(v)).map((k, v) => s"generated $k: ${digests.getOrElse(k, "absent")}, generated.json has $v") ++
      digests.keys.toVector.sorted.filter(k => !recorded.contains(k)).map(k => s"generated $k: ${digests(k)}, generated.json has none")

  // A compile's diagnostics by project, from teq's lines `<project>/<configuration>: N errors` and the diagnostics
  // printed before them.
  private val summary = "^([^ /]+)/(compile|test): (.*)$".r

  def compile(pin: Pin, dir: Path, out: Path, teq: String, runs: Int, mode: Mode): Unit =
    val lock = exported(pin, dir)
    val bin = binary(teq)
    val version = Sh(bin, "--version").timeout(30).check(false).run().out.trim
    val buildDir = dir.resolve(pin.build)
    val roots = Roots.of(buildDir, dir, out)
    var failed = false
    val seen = Vector.newBuilder[(Observation, String, Map[String, String])]
    for i <- 1 to runs do
      Sh(bin, "stop").in(buildDir).timeout(120).check(false).run()
      coldOutputs(buildDir)
      val log = out.resolve(s"compile-$i.log")
      // The builds' diagnostics as teq records them (`--diagnostics`), never read from the text.
      val diagnostics = out.resolve(s"compile-$i.diagnostics")
      Files.deleteIfExists(diagnostics)
      Log.stamped(s"${pin.name}: teq compile, run $i of $runs, its output $log")
      val started = System.nanoTime()
      val (r, peak) = sampled(pin.name, bin, out)(Sh(bin, "compile", "--diagnostics", diagnostics.toString).in(buildDir).stdout(log).mergeErr.timeout(teqSeconds).check(false).run())
      val wall = (System.nanoTime() - started) / 1e9
      Files.writeString(log, Transcript.mark(r.status), java.nio.file.StandardOpenOption.APPEND)
      val text = Files.readString(log)
      val records = Transcript.recorded(diagnostics)
      // Every compile configuration the export holds sources for covered by the run (`Locks.hold`).
      val observed = Locks.hold(Transcript.compile(text, records, roots), lock)
      val warnings = records.fold(Vector.empty)(Transcript.shown).count(_.severity == "warning")
      // Each configuration's last line is its outcome (its generators print before it).
      val outcomes = text.linesIterator.flatMap(l => summary.findFirstMatchIn(l)).map(m => s"${m.group(1)}/${m.group(2)}" -> m.group(3)).toMap.toVector.sorted.map((k, v) => s"$k: $v")
      println(f"${pin.name}: compile run $i: exit ${r.status}, $wall%.1f s, peak ${peak.fold("unmeasured")(k => f"${k / 1048576.0}%.2f GB")}, ${observed.debt.size} errors, $warnings warnings")
      outcomes.foreach(o => println(s"  $o"))
      // The generated inputs' digests as this run left them: each run's held apart, so that no later run hides an
      // earlier one's bytes.
      seen += ((observed, roots(text), generatedDigests(pin, buildDir)))
    Sh(bin, "stop").in(buildDir).timeout(120).check(false).run()
    val recordedFile = pin.here.resolve("generated.json")
    val runsSeen = seen.result()
    // The digests the runs are held to: a baseline's or a migration's those the first run left, published with its
    // record as one recording; else generated.json's, a generated input that differs an item of the runs' debt with the digest
    // it has (other bytes another item, so that a change of the generated bytes takes a migration).
    val recording = mode match
      case Mode.Baseline | Mode.Migrate(_) => true
      case _ => false
    val recorded =
      if recording then runsSeen.head._3
      else if Files.isRegularFile(recordedFile) then Json.parse(Files.readString(recordedFile)).members.map((k, v) => k -> v.str).toMap
      else Map.empty[String, String]
    if recorded.isEmpty then
      println(s"FAIL ${pin.name}: no digests of the generated inputs in bench/${pin.name}/generated.json (compile --baseline)")
      failed = true
    val staged = if recording then Vector("generated.json" -> (Json.write(Json.Obj(recorded.toVector.sortBy(_._1).map((k, v) => k -> Json.str(v))), indent = 1) + "\n").getBytes(java.nio.charset.StandardCharsets.UTF_8)) else Vector.empty
    val judged = Vector.newBuilder[(String, Observation)]
    for ((observed, _, digests), i) <- runsSeen.zipWithIndex do judged += s"compile run ${i + 1}" -> Transcript.withDebt(observed, digestDifferences(recorded, digests))
    // Each kind of generated input deleted apart, the generators' records kept: the files the generators declare
    // they write besides (the build's assets), then the generated directories; a compile writes each again, to
    // the digests the runs are held to, and its errors are the record's: each of these compiles a run of the line.
    val apart = Vector("the generators' declared files" -> (() => pin.generatorOutputs.map(buildDir.resolve).foreach(Files.deleteIfExists)),
      "the generated directories" -> (() => managedRoots(buildDir).foreach(Sh.remove)))
    for (what, delete) <- apart if what != apart.head._1 || pin.generatorOutputs.nonEmpty do
      Sh(bin, "stop").in(buildDir).timeout(120).check(false).run()
      delete()
      val log = out.resolve(s"compile-regenerated-${if what == apart.head._1 then "files" else "directories"}.log")
      val diagnostics = out.resolve(log.getFileName.toString.stripSuffix(".log") + ".diagnostics")
      Files.deleteIfExists(diagnostics)
      val r = Sh(bin, "compile", "--diagnostics", diagnostics.toString).in(buildDir).stdout(log).mergeErr.timeout(teqSeconds).check(false).run()
      Sh(bin, "stop").in(buildDir).timeout(120).check(false).run()
      Files.writeString(log, Transcript.mark(r.status), java.nio.file.StandardOpenOption.APPEND)
      val differences = digestDifferences(recorded, generatedDigests(pin, buildDir))
      println(s"${pin.name}: $what deleted, compile exits ${r.status}, ${if differences.isEmpty then "every generated input written again with the digests the runs are held to" else s"${differences.size} differences"}")
      judged += s"compile, $what deleted" -> Locks.hold(Transcript.withDebt(Transcript.compile(Files.readString(log), Transcript.recorded(diagnostics), roots), differences), lock)
    if !Lines.hold(pin.name, "compile", pin.here, Shape.Compile, judged.result(), runsSeen.head._2, mode, version, checked = !failed, staged) then failed = true
    Script.exit(if failed then 1 else 0)

  private val listedSuite = "^Test ([^ ]+) : .*".r

  def test(pin: Pin, dir: Path, out: Path, teq: String, runs: Int, mode: Mode): Unit =
    val lock = exported(pin, dir)
    val bin = binary(teq)
    val version = Sh(bin, "--version").timeout(30).check(false).run().out.trim
    val buildDir = dir.resolve(pin.build)
    val roots = Roots.of(buildDir, dir, out)
    // The oracle's inventory, which the run is held to: without one nothing would be checked.
    val inventory = pin.here.resolve("tests.json")
    if !Files.isRegularFile(inventory) then Log.die(s"no inventory at $inventory: record it with the oracle (oracle --record)")
    val expected = Inventory.read(inventory)
    if expected.cases == 0 then Log.die(s"the inventory $inventory holds no case")
    // The modules are the export's, not the inventory's, which they are compared with.
    val scope = Export.projects(lock)
    if scope.isEmpty then Log.die(s"the export $lock holds no project")
    var failed = false
    val held = Vector.newBuilder[(String, Observation, String)]
    for i <- 1 to runs do
      val log = out.resolve(s"test-$i.log")
      Log.stamped(s"${pin.name}: teq test, run $i of $runs, its output $log")
      val started = System.nanoTime()
      // Each module whose test configuration has sources asked for its suites (sbt's definedTests), then each with a
      // suite run verbose, so that the runner names every suite and every test with its outcome: the listing and the
      // transcript are the run's inventory. A module without a test source has no suite (teq's Scala.js runner
      // refuses a listing or a run of none, which Mill's passes). Each invocation's output and exit is the module's
      // own, so that a module whose listing or run failed in its own build is blocked by it; its two channels collected
      // apart, teq's own lines (a build's diagnostics and count, the summaries) on stderr, the listing and the run's
      // stream on stdout; the outcomes are the events' (`Transcript`).
      Files.writeString(log, "")
      val ((listed, statuses, (invoked, ranModules)), peak) = sampled(pin.name, bin, out) {
        val listed = scope.map { (m, sources) =>
          if !Export.holdsSources(buildDir, sources) then (m, Vector.empty[String], 0, Option.empty[(String, String, Option[String])])
          else
            val diagnostics = out.resolve(s"test-$i-$m-list.diagnostics")
            Files.deleteIfExists(diagnostics)
            val r = Sh(bin, "test", "--list", m, "--diagnostics", diagnostics.toString).in(buildDir).timeout(teqSeconds).check(false).run()
            val (text, errText) = (r.outAll(), r.errAll())
            Files.writeString(log, Invocation.shown(text, errText), java.nio.file.StandardOpenOption.APPEND)
            (m, text.linesIterator.collect { case listedSuite(s) => s }.toVector.distinct, r.status, Some((text, errText, Transcript.recorded(diagnostics))))
        }
        // Each module's run writes its events (`--events`), every case under the status its framework reported (an
        // error apart from a failure, which the transcript's `[fail]` does not tell apart).
        // Each invocation's output collected apart, then appended to the run's log: its transcript is its own, whatever
        // the invocations before it printed.
        val invocations = listed.filter(_._2.nonEmpty).map { (m, _, _, _) =>
          val own = out.resolve(s"test-$i-$m.log")
          val ownErr = out.resolve(s"test-$i-$m.err")
          val events = out.resolve(s"test-$i-events-$m.txt")
          val diagnostics = out.resolve(s"test-$i-$m.diagnostics")
          Files.deleteIfExists(events)
          Files.deleteIfExists(diagnostics)
          val r = Sh(bin, "test", "--verbose", "--events", events.toString, "--diagnostics", diagnostics.toString, m).in(buildDir).stdout(own).stderr(ownErr).timeout(teqSeconds).check(false).run()
          val (text, errText) = (r.outAll(), r.errAll())
          Files.writeString(log, Invocation.shown(text, errText), java.nio.file.StandardOpenOption.APPEND)
          (m, r.status, (text, errText, Transcript.recorded(diagnostics)), Option.when(Files.isRegularFile(events))(Files.readString(events)))
        }
        // Every invocation as the records judge it: each listing (no events asked) and each run (its events).
        val invoked = listed.collect { case (m, _, status, Some((text, errText, records))) => Invocation(s"$m --list", status, text, errText, records, Some("")) } ++
          invocations.map((m, status, text, events) => Invocation(m, status, text._1, text._2, text._3, events))
        (listed, listed.filter(_._3 != 0).map((m, _, status, _) => s"$m --list" -> status) ++ invocations.map((m, status, _, _) => m -> status), (invoked, invocations))
      }
      val wall = (System.nanoTime() - started) / 1e9
      val text = Files.readString(log)
      // What ran is the run's events, never its text and never a count: the cases they name with their statuses (a
      // run without its events a problem) and each suite whose task ran, by name, one that ran no case too
      // (`Deviations.ran`).
      val ran = Inventory.fromEvents(ranModules.flatMap(_._4).mkString)
      val named = ranModules.flatMap(_._4.toVector.flatMap(Deviations.ran)).groupMap(_._1)(_._2).map((m, suites) => m -> suites.toSet)
      val found = Inventory(listed.map((m, suites, _, _) => m -> Module(0, suites, ran.modules.get(m).fold(Vector.empty)(_.cases))).toMap)
      println(f"${pin.name}: test run $i: $wall%.1f s, peak ${peak.fold("unmeasured")(k => f"${k / 1048576.0}%.2f GB")}, ${found.modules.size} modules, ${found.suites} suites listed, ${ran.suites} run, ${found.cases} cases run (tests.json: ${expected.modules.size} modules, ${expected.suites} suites, ${expected.cases} cases)")
      for (m, status) <- statuses if status != 0 do println(s"  $m: teq test exits $status")
      val differences = Inventory.compare(expected, found, warnings = false) ++ Inventory.ranAsListed(found, ran, named)
      if differences.nonEmpty then
        println(s"${pin.name}: test run $i: ${differences.size} differences from tests.json")
        differences.take(60).foreach(d => println(s"  $d"))
      // The run held to deviations.json by the records' one judgment of a test run (`Deviations.judge`, the same for
      // every driver): every listing and run, the oracle's modules and cases (a case `<suite>.<test>` under no suite),
      // the modules complete with nothing to run (a listing that passed with no suite), the suites listed; and Mill's
      // own departures, a suite listed against its run and a module beyond the oracle's.
      def cases(t: Inventory) = t.modules.toVector.sortBy(_._1).flatMap((m, mod) => mod.cases.map((c, st) => TestCase(m, "", c) -> st))
      def suites(t: Inventory) = t.modules.toVector.sortBy(_._1).flatMap((m, mod) => mod.suites.distinct.map(m -> _))
      val evidence = TestEvidence(invoked, expected.modules.keys.toVector.sorted, cases(expected), suites(expected), qualified = true,
        completedWithoutRun = listed.collect { case (m, suites, 0, _) if suites.isEmpty => m },
        listedSuites = Some(suites(found)),
        debt = Inventory.ranAsListed(found, ran, named) ++ scope.map(_._1).filterNot(expected.modules.contains).map(m => s"$m (a module not the oracle's)"),
        owed = listed.collect { case (m, _, _, Some(_)) => m })
      val observed = Deviations.judge(evidence, roots)
      held += ((s"test run $i", observed, roots(text)))
    Sh(bin, "stop").in(buildDir).timeout(120).check(false).run()
    val seen = held.result()
    if !Lines.hold(pin.name, "test", pin.here, Shape.Tests, seen.map((n, o, _) => n -> o), seen.head._3, mode, version) then failed = true
    Script.exit(if failed then 1 else 0)

  // The run check: pin.json's `run` module built by teq, served with its page and checked in a browser by
  // bench/smoke.mjs; the build's time and peak memory are the line's, the page's steps the smoke's log.
  def run(pin: Pin, dir: Path, out: Path, teq: String, mode: Mode): Unit =
    exported(pin, dir)
    val bin = binary(teq)
    val version = Sh(bin, "--version").timeout(30).check(false).run().out.trim
    val buildDir = dir.resolve(pin.build)
    val roots = Roots.of(buildDir, dir, out)
    val spec = pin.json.get("run").getOrElse(Log.die(s"bench/${pin.name}/pin.json names no run"))
    val module = spec("module").str
    val log = out.resolve("run-build.log")
    // The build's diagnostics as the compiler records them (`--diagnostics`, the compiler's flag `teq build` passes on).
    val diagnostics = out.toAbsolutePath.resolve("run-build.diagnostics")
    Files.deleteIfExists(diagnostics)
    // The checker's assertions, its own declaration of them held complete (`Assertions.declared`) before the run makes
    // any evidence: a listing that failed, timed out or declared nothing whole judges and records nothing.
    val listing = Sh("node", "bench/smoke.mjs", spec("smoke").str, "--assertions").timeout(60).check(false).run()
    val names = Assertions.declared(listing.status, listing.timedOut, listing.out) match
      case Right(declared) => declared
      case Left(why) =>
        println(s"FAIL ${pin.name}: run: bench/smoke.mjs ${spec("smoke").str} --assertions: $why: nothing is judged or recorded")
        Script.exit(1)
    Sh(bin, "stop").in(buildDir).timeout(120).check(false).run()
    Log.stamped(s"${pin.name}: teq build $module, its output $log")
    val started = System.nanoTime()
    val (r, peak) = sampled(pin.name, bin, out)(Sh(bin, "build", module, "--diagnostics", diagnostics.toString).in(buildDir).stdout(log).mergeErr.timeout(teqSeconds).check(false).run())
    val wall = (System.nanoTime() - started) / 1e9
    Sh(bin, "stop").in(buildDir).timeout(120).check(false).run()
    val built = buildDir.resolve("target/teq").resolve(module).resolve("out")
    // The run held to run.json by the records' one judgment of a run (`Assertions.judge`, the same for every driver):
    // the smoke scenario's assertions, and where the build failed before any could be checked, its exit and its own
    // errors (`teq build` is a one-shot build: its errors' whole blocks).
    def held(evidence: RunEvidence, shown: String): Nothing =
      Script.exit(if Lines.hold(pin.name, "run", pin.here, Shape.Run, Vector("run" -> Assertions.judge(evidence)), shown, mode, version) then 0 else 1)
    if r.status != 0 || !Files.isDirectory(built) then
      val text = Files.readString(log)
      println(text.linesIterator.filter(_.contains("error")).take(20).mkString("\n"))
      println(s"${pin.name}: teq build $module exits ${r.status}${if Files.isDirectory(built) then "" else s", no output at $built"}")
      val (errors, counted) = RunEvidence.errors(text, Transcript.recorded(diagnostics), roots)
      held(RunEvidence(names, s"teq build $module", Some((r.status, errors)), "", Transcript.crashes(text, Transcript.recorded(diagnostics), roots) ++ counted), roots(text))
    println(f"${pin.name}: built $module in $wall%.1f s, peak ${peak.fold("unmeasured")(k => f"${k / 1048576.0}%.2f GB")}")
    // The page's directory: the build as out/, the page, the harness's files and the assets.
    val serve = out.resolve("serve")
    Sh.remove(serve)
    Files.createDirectories(serve)
    Sh("cp", "-r", built.toString, serve.resolve("out").toString).timeout(120).run()
    // The project's own page, or one of the harness's files (Indigo's plugin writes its page at its build).
    for page <- spec.get("page") do Files.copy(buildDir.resolve(page.str), serve.resolve("index.html"))
    for (dest, source) <- spec("files").members do
      val target = serve.resolve(dest)
      Files.createDirectories(target.getParent)
      Files.copy(pin.here.resolve(source.str), target)
    for (dest, source) <- spec("assets").members do
      val target = serve.resolve(dest)
      Files.createDirectories(target.getParent)
      Sh("cp", "-r", buildDir.resolve(source.str).toString, target.toString).timeout(120).run()
    val smoke = out.resolve("run-smoke.log")
    Log.stamped(s"${pin.name}: bench/smoke.mjs ${spec("smoke").str} over $serve, its log $smoke")
    val s = Sh("node", "bench/smoke.mjs", spec("smoke").str, serve.toString, "--seconds", "120", "--screenshots", out.resolve("shots").toString)
      .stdout(smoke).mergeErr.timeout(300).check(false).run()
    val smokeText = Files.readString(smoke)
    val lines = smokeText.linesIterator.toVector
    lines.filter(l => l.startsWith("  ") || l.startsWith("assert ")).filterNot(_.startsWith("  page: ")).foreach(println)
    val verdict = lines.reverse.find(l => l.startsWith("smoke: ") || l.startsWith("FAIL smoke: "))
    println(s"${pin.name}: run: ${verdict.fold(s"no verdict (node exits ${s.status})")(_.stripPrefix("FAIL ").stripPrefix("smoke: "))}")
    // The smoke's assertions held to run.json, with its exit and its verdict, which the judgment holds them to.
    held(RunEvidence(names, s"teq build $module", None, smokeText, Vector.empty, checker = Some((s.status, verdict.map(_.startsWith("smoke: "))))), roots(smokeText))

// A module's scalac warnings (its main and test compiles) and its test cases, by their names as Mill and teq's
// runner give them (`<suite>.<test>`), each with the status the framework reported (`sbt.testing.Status`'s name:
// `Success`, `Error`, `Failure`, `Skipped`, `Ignored`, `Canceled`, `Pending`), a case as many times as the run reported
// it.
final case class Module(warnings: Int, suites: Vector[String], cases: Vector[(String, String)])
final case class Inventory(modules: Map[String, Module]):
  def cases: Int = modules.values.map(_.cases.size).sum
  def suites: Int = modules.values.map(_.suites.size).sum
  def warnings: Int = modules.values.map(_.warnings).sum

object Inventory:
  // sbt.testing.Status's names, in its order.
  val statuses = Vector("Success", "Error", "Failure", "Skipped", "Ignored", "Canceled", "Pending")

  // tests.json: each module's cases under their statuses, each status's names sorted, a name as many times as the
  // case ran with that status. An inventory of the outcomes alone (an object of names, before the statuses were
  // kept) does not read: the oracle records it again (oracle --record).
  def read(file: Path): Inventory =
    if !Files.isRegularFile(file) then Inventory(Map.empty)
    else
      val j = Json.parse(Files.readString(file))
      Inventory(j("modules").members.map { (name, m) =>
        val cases = m("cases").members.flatMap { (status, names) =>
          if !statuses.contains(status) then Log.die(s"$file: module $name's cases are under `$status`, no status of sbt.testing.Status's: the oracle records the inventory again (oracle --record)")
          names.elements.map(c => c.str -> status)
        }
        name -> Module(m("warnings").int, m("suites").elements.map(_.str), cases)
      }.toMap)

  // Every module with its warnings, suites and cases by status, sorted, the counts first.
  def write(t: Inventory): String =
    val modules = t.modules.toVector.sortBy(_._1).map { (name, m) =>
      val byStatus = statuses.filter(st => m.cases.exists(_._2 == st)).map(st => st -> Json.Arr(m.cases.filter(_._2 == st).map(_._1).sorted.map(Json.str)))
      name -> Json.obj("warnings" -> Json.num(m.warnings), "suites" -> Json.Arr(m.suites.sorted.map(Json.str)), "cases" -> Json.Obj(byStatus))
    }
    Json.write(Json.obj("modules_count" -> Json.num(t.modules.size), "suites_count" -> Json.num(t.suites), "cases_count" -> Json.num(t.cases),
      "warnings_count" -> Json.num(t.warnings), "modules" -> Json.Obj(modules)), indent = 1) + "\n"

  // A module's key as Mill renders it, from its directory under out/: `tyrian-tags/js` is `tyrian-tags.js`, a cross
  // value `indigo-plugin/3` is `indigo-plugin[3]`.
  private def moduleKey(segments: Vector[String]): String =
    segments.foldLeft("") { (acc, s) =>
      if acc.nonEmpty && s.headOption.exists(_.isDigit) then s"$acc[$s]" else if acc.isEmpty then s else s"$acc.$s"
    }

  private val warningLine = "^\\[warn\\] -- .*Warning".r

  // Mill's out/ after the oracle's steps: each module's compile.log (its own and its test module's) and its test
  // module's testForked.json and discoveredTestClasses.json.
  def fromMill(buildDir: Path): Inventory =
    val outDir = buildDir.resolve("out")
    val files = Files.walk(outDir).toArray.toList.map(_.asInstanceOf[Path]).filter(Files.isRegularFile(_))
    val modules = scala.collection.mutable.LinkedHashMap.empty[String, Module]
    def segmentsOf(p: Path): Vector[String] =
      val rel = outDir.relativize(p.getParent)
      (0 until rel.getNameCount).map(i => rel.getName(i).toString).toVector
    for p <- files if p.getFileName.toString == "compile.log" do
      val segs = segmentsOf(p)
      if segs.headOption != Some("mill-build") then
        val owner = if segs.lastOption.contains("test") then segs.dropRight(1) else segs
        val key = moduleKey(owner)
        val warnings = Files.readString(p).linesIterator.count(l => warningLine.findFirstIn(l).nonEmpty)
        val m = modules.getOrElse(key, Module(0, Vector.empty, Vector.empty))
        modules(key) = m.copy(warnings = m.warnings + warnings)
    for p <- files if p.getFileName.toString == "testForked.json" do
      val segs = segmentsOf(p)
      val key = moduleKey(segs.dropRight(1))
      val results = Json.parse(Files.readString(p))("value")("results").elements
      val cases = results.map { r =>
        val status = r("status").str
        if !statuses.contains(status) then Log.die(s"$p: a case of status $status, no status of sbt.testing.Status's")
        r("fullyQualifiedName").str -> status
      }
      // The suites: Mill's discovered classes on the JVM; on Scala.js, which records none, each case's
      // longest prefix that is a class the test module compiled (`<suite>.<test>`).
      val discovered = p.getParent.resolve("discoveredTestClasses.json")
      val suites = if Files.isRegularFile(discovered) then Json.parse(Files.readString(discovered))("value").elements.map(_.str) else
        val classesDir = p.getParent.resolve("compile.dest/classes")
        val classes = if !Files.isDirectory(classesDir) then Set.empty[String] else
          Files.walk(classesDir).toArray.toList.map(_.asInstanceOf[Path]).map(f => classesDir.relativize(f).toString)
            .filter(f => f.endsWith(".sjsir") || f.endsWith(".class")).map(f => f.substring(0, f.lastIndexOf('.')).replace('/', '.')).filterNot(_.contains("$")).toSet
        cases.flatMap((c, _) => classes.filter(k => c.startsWith(k + ".")).maxByOption(_.length)).distinct
      val m = modules.getOrElse(key, Module(0, Vector.empty, Vector.empty))
      modules(key) = m.copy(suites = suites, cases = cases)
    Inventory(modules.toMap)

  // The events of a run as `teq test --events` writes them (`task::events::Event::line`): a line each, the project
  // (the module), the event's name (the suite), its selector, its status and the first line of what it threw,
  // tab-separated; a case is the test a `test <name>` selector names (`<suite>.<test>`, Mill's and munit's
  // fully qualified name), another selector as it is written.
  def fromEvents(text: String): Inventory =
    val modules = scala.collection.mutable.LinkedHashMap.empty[String, Module]
    for line <- text.linesIterator if line.nonEmpty do
      line.split("\t", -1) match
        case Array(project, suite, selector, status, _*) =>
          val m = modules.getOrElse(project, Module(0, Vector.empty, Vector.empty))
          modules(project) = m.copy(suites = (m.suites :+ suite).distinct, cases = m.cases :+ ((if selector.startsWith("test ") then selector.drop(5) else selector) -> status))
        case _ => ()
    Inventory(modules.toMap)

  // Where the run and the expected inventory differ, each a line, identity for identity: a module, a suite or a case
  // either side lacks, a case's outcome, and a module's warnings where `warnings` (the oracle's own runs) asks.
  def compare(expected: Inventory, found: Inventory, warnings: Boolean): Vector[String] =
    val out = Vector.newBuilder[String]
    if expected.modules.isEmpty then out += "tests.json holds no module"
    for (name, m) <- expected.modules.toVector.sortBy(_._1) do
      found.modules.get(name) match
        case None => out += s"module $name of tests.json (${m.suites.size} suites, ${m.cases.size} cases) is not the run's"
        case Some(f) =>
          if warnings && f.warnings != m.warnings then out += s"$name: ${f.warnings} warnings, tests.json has ${m.warnings}"
          for s <- m.suites.distinct.sorted if !f.suites.contains(s) do out += s"$name: suite $s of tests.json is not the run's"
          for s <- f.suites.distinct.sorted if !m.suites.contains(s) do out += s"$name: suite $s is not in tests.json"
          // Each case with its statuses as many times as each side reports it: a case run twice, a status changed (an
          // ignored case pending), a failure after a pass.
          def times(statuses: Vector[String]): String =
            statuses.groupBy(identity).toVector.sortBy(_._1).map((st, all) => if all.size == 1 then st else s"$st x${all.size}").mkString(", ")
          val (want, got) = (m.cases.groupMap(_._1)(_._2), f.cases.groupMap(_._1)(_._2))
          for c <- (m.cases.map(_._1) ++ f.cases.map(_._1)).distinct do
            val (w, g) = (want.getOrElse(c, Vector.empty).sorted, got.getOrElse(c, Vector.empty).sorted)
            if w != g then
              if g.isEmpty then out += s"$name: case \"$c\" did not run"
              else if w.isEmpty then out += s"$name: case \"$c\" (${times(g)}) is not in tests.json"
              else out += s"$name: case \"$c\" ${times(g)}, tests.json has ${times(w)}"
    for name <- found.modules.keys.toVector.sorted if !expected.modules.contains(name) do
      out += s"module $name is not in tests.json"
    out.result()

  // A suite teq's runner listed and did not run, or ran unlisted, each a line, and a module with suites that ran none.
  // What ran is the run's events (`named`: each module's suites whose tasks ran, by name), never its text nor a count:
  // a suite ran where its own line names it, or an event names it or one of its cases (an ignored suite reports one
  // case, `<suite>.<unknown>`; Scala.js's bridge names a case `<suite>.<test>`).
  def ranAsListed(listed: Inventory, ran: Inventory, named: Map[String, Set[String]]): Vector[String] =
    val out = Vector.newBuilder[String]
    def of(s: String, n: String) = n == s || n.startsWith(s + ".")
    for (name, m) <- listed.modules.toVector.sortBy(_._1) if m.suites.nonEmpty do
      val suites = named.getOrElse(name, Set.empty)
      if !ran.modules.contains(name) && suites.isEmpty then out += s"module $name did not run its ${m.suites.size} suites"
      else
        val r = ran.modules.getOrElse(name, Module(0, Vector.empty, Vector.empty))
        def wasRun(s: String) = suites.contains(s) || r.suites.exists(of(s, _)) || r.cases.exists(c => of(s, c._1))
        for s <- m.suites.distinct.sorted if !wasRun(s) do out += s"$name: suite $s was listed and did not run"
        for s <- (r.suites ++ suites).distinct.sorted if !m.suites.exists(of(_, s)) do out += s"$name: suite $s ran and was not listed"
    for name <- (ran.modules.keySet ++ named.keySet).toVector.sorted if !listed.modules.contains(name) do out += s"module $name ran and is not the export's"
    out.result()

object Bootstrap:
  // A jar's identity apart from its timestamps: the digest of its entries' names, each with its bytes' digest, in name
  // order.
  def entriesDigest(jar: Path): String =
    val zip = new java.util.zip.ZipFile(jar.toFile)
    try
      val entries = scala.jdk.CollectionConverters.EnumerationHasAsScala(zip.entries()).asScala.toVector.filterNot(_.isDirectory).sortBy(_.getName)
      val listing = entries.map { e =>
        val in = zip.getInputStream(e)
        val bytes = try in.readAllBytes() finally in.close()
        s"${e.getName} ${Pin.sha256(bytes)}\n"
      }.mkString
      Pin.sha256(listing.getBytes(java.nio.charset.StandardCharsets.UTF_8))
    finally zip.close()

// The export's lock, read by its indentation (two spaces a level): its projects, each with its test configuration's
// source directories (none without one), and the digest it records of an input file.
object Export:
  private def level(l: String): Int = l.takeWhile(_ == ' ').length / 2

  private def key(l: String): String = l.trim.stripSuffix(":").stripPrefix("\"").stripSuffix("\"")

  def projects(lock: Path): Vector[(String, Vector[String])] =
    val lines = Files.readString(lock).linesIterator.toVector
    val body = lines.dropWhile(_ != "projects:").drop(1).takeWhile(l => l.isEmpty || l.startsWith(" ")).filter(_.trim.nonEmpty)
    val out = Vector.newBuilder[(String, Vector[String])]
    var project: Option[String] = None
    var sources = Vector.empty[String]
    var section = Vector.empty[String]
    for l <- body do
      val at = level(l)
      val within = section.take(at - 1)
      section = within :+ key(l)
      at match
        case 1 =>
          project.foreach(p => out += p -> sources)
          project = Some(key(l))
          sources = Vector.empty
        case 5 if within.drop(1) == Vector("configurations", "test", "sources") && l.trim.startsWith("- ") =>
          sources = sources :+ key(l.trim.stripPrefix("- "))
        case _ => ()
    project.foreach(p => out += p -> sources)
    out.result().sortBy(_._1)

  // Whether a directory under the build holds a Scala or Java source.
  def holdsSources(buildDir: Path, dirs: Vector[String]): Boolean =
    dirs.map(buildDir.resolve).filter(Files.isDirectory(_)).exists { d =>
      val walk = Files.walk(d)
      try walk.anyMatch(f => Files.isRegularFile(f) && (f.toString.endsWith(".scala") || f.toString.endsWith(".java")))
      finally walk.close()
    }

  def input(lock: Path, file: String): Option[String] =
    val lines = Files.readString(lock).linesIterator.toVector
    val inputs = lines.dropWhile(_ != "inputs:").drop(1).takeWhile(l => l.isEmpty || l.startsWith(" "))
    val files = inputs.dropWhile(_.trim != "files:").drop(1).takeWhile(l => level(l) >= 2)
    files.map(_.trim).collectFirst { case l if l.startsWith(file + ": ") => l.stripPrefix(file + ": ").trim }
