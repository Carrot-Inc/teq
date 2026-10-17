//> using file ../../tools/script
// The reference projects' checkouts on the machine that holds them (docs/DEVELOPING.md, "The landing gate"): lila,
// Airstream, Laminar, Tyrian and Indigo at their pins, under one directory, with what their gate lines need, each
// verified before the machine is called ready. Run from the repository's root:
//
//   ./teq interp bench/gate-projects/setup.scala -- <dir> [--projects lila,airstream,laminar,tyrian,indigo] [--teq <binary>]
//
// Every project's lines run over `<dir>/<project>` (Airstream's directory is `airstream`, its build naming no root
// project), each stage in turn, every one bounded by a deadline of its own, its log under <dir>/.setup/logs:
//
// tools     git, curl, tar, node and npm on the PATH, the JDK JAVA_HOME names at the pins' major, sbt for the sbt builds;
//           for lila node 24 or later and pnpm 12 or later (lila's package.json's engines; this program installs
//           neither), a C compiler and make where Redis is to be built
// services  lila's: MongoDB and Redis, LILA_MONGOD and LILA_REDIS's, else the PATH's, else the releases
//           services.json pins, fetched into <dir>/downloads, held to their digests and installed under
//           <dir>/services (Redis built from its source); each started on lila's port (27017, 6379) and ready, then
//           stopped, the ports 27017, 6379 and 9663 free after
// checkout  each project's verb `checkout` (its pin, its overlay, the npm fixture of the Scala.js ones)
// export    each project's verb `export` into <dir>/.setup/out/<project>: the build's dependencies resolved, Mill's
//           launcher's Mill fetched, Indigo's plugin bootstrapped at the absolute root its pin.json names (the one path
//           on every machine; its artifacts held to their digests) and mill-teq published, the lock written
// assets    lila's ui built by its own ui/build (pnpm, incremental), and every script public/compiled/manifest.json
//           names there with the scripts it imports
// browser   for Tyrian and Indigo, the headless Chrome bench/smoke.mjs drives making a WebGL2 context
//           (`node bench/smoke.mjs webgl`; CHROME names another)
//
// Every stage runs each time, its verbs idempotent (a checkout at its pin fetches nothing, an export resolves what the
// coursier cache holds, the ui's build is incremental), so that what a stage made is checked again before the machine
// is called ready: the pins and the overlays, the npm fixtures, the exports' dependencies, the assets (a product
// deleted since is made again); an interrupted run is run again, its leftovers cleaned first (the services'
// processes, a partial download, a partial extraction). <dir>/ready is removed at the start and written at the end
// only when every stage of the run passed, with <dir>/gate-projects.env, the assignments the machine's login shell sources for the gate:
// GATE_<PROJECT>_CHECKOUT for each project, LILA_MONGOD and LILA_REDIS, JAVA_HOME. The program names no machine;
// the machine that holds a project is the pool's business. Exit 1 when a stage fails, naming it.
import java.nio.file.{Files, Path, Paths, StandardCopyOption}
import java.nio.charset.StandardCharsets.UTF_8
import java.security.MessageDigest

@main def setup(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/gate-projects/setup.scala <dir> [--projects lila,airstream,laminar,tyrian,indigo] [--teq <binary>]")
  val teq = Paths.get(args.teq("./target/release/teq")).toAbsolutePath.normalize
  val projects = args.option("projects").map(_.split(',').toVector.filter(_.nonEmpty)).getOrElse(Setup.all)
  val dirArg = args.exactly(1).head
  if !Files.isRegularFile(Paths.get("bench/gate-projects/services.json")) then Log.die("bench/gate-projects/setup.scala runs from the repository's root")
  for p <- projects if !Setup.all.contains(p) do args.fail(s"no reference project $p (${Setup.all.mkString(", ")})")
  if !Files.isExecutable(teq) then Log.die(s"no teq binary at $teq (--teq)")
  val dir = Paths.get(dirArg).toAbsolutePath.normalize
  if dir.startsWith(Paths.get("").toAbsolutePath) then Log.die(s"$dir is inside the repository; the checkouts stay outside it")
  Script.exit(if new Setup(dir, projects, teq.toString).run() then 0 else 1)
}

object Setup:
  val all = Vector("lila", "airstream", "laminar", "tyrian", "indigo")
  val ports = Vector(27017, 6379, 9663)
  def sha256(bytes: Array[Byte]): String = MessageDigest.getInstance("SHA-256").digest(bytes).map(b => f"${b & 0xff}%02x").mkString

final class Setup(dir: Path, projects: Vector[String], teq: String):
  private val logs = dir.resolve(".setup/logs")
  private val outs = dir.resolve(".setup/out")
  private val pids = dir.resolve(".setup/pids")
  private var env = Vector.empty[(String, String)]
  private val lila = projects.contains("lila")
  private val sbtBuilt = projects.exists(Set("lila", "airstream", "laminar"))
  private val browsers = projects.exists(Set("tyrian", "indigo"))

  private def say(line: String): Unit = Log.stamped(s"setup: $line")

  // A stage, run (a failure, an exception or a stop of its body fails it): whether it passed.
  private def stage(name: String)(body: => Boolean): Boolean =
    val started = System.nanoTime()
    val ok = try body catch
      case e: Script.Exit => false
      case e: Exception =>
        println(s"FAIL setup: $name: ${e.getMessage}")
        false
    val secs = (System.nanoTime() - started) / 1e9
    if ok then say(f"$name: passed in $secs%.0f s")
    else println(f"FAIL setup: $name: failed after $secs%.0f s (its log under $logs)")
    ok

  private def failed(name: String, why: String): Boolean =
    println(s"FAIL setup: $name: $why")
    false

  // A tool's version line, None when it does not run.
  private def version(cmd: String*): Option[String] =
    try
      val r = Sh(cmd*).timeout(60).mergeErr.check(false).run()
      Option.when(r.ok)(r.out.trim.linesIterator.toVector.headOption.getOrElse(""))
    catch case _: java.io.IOException => None

  private def onPath(name: String): Option[Path] =
    Option(System.getenv("PATH")).toVector.flatMap(_.split(':')).map(d => Paths.get(d, name)).find(Files.isExecutable(_))

  // The leading number of a version line (`v24.1.0`, `12.9.0`).
  private def major(line: String): Int = "([0-9]+)".r.findFirstMatchIn(line).map(_.group(1).toInt).getOrElse(0)

  // The pins' JDK major: every project's pin names one.
  private def pinnedJava: Int =
    val majors = projects.map(p => Json.parse(Files.readString(Paths.get("bench", p, "pin.json")))("java").int).distinct
    if majors.size != 1 then Log.die(s"setup: the pins name the JDKs ${majors.mkString(", ")}; one JAVA_HOME serves one")
    majors.head

  private def tools(): Boolean =
    val missing = Vector.newBuilder[String]
    for t <- Vector("git", "curl", "tar", "node", "npm") if onPath(t).isEmpty do missing += s"$t is not on the PATH"
    if sbtBuilt && onPath("sbt").isEmpty then missing += "sbt is not on the PATH (the sbt builds' launcher)"
    val java = pinnedJava
    Option(System.getenv("JAVA_HOME")).filter(_.nonEmpty).map(Paths.get(_)) match
      case None => missing += s"JAVA_HOME names no JDK; the pins build under Java $java"
      case Some(home) =>
        version(home.resolve("bin/java").toString, "-version").map(l => "version \"([0-9]+)".r.findFirstMatchIn(l).map(_.group(1).toInt)) match
          case Some(Some(m)) if m == java => env = env :+ ("JAVA_HOME" -> home.toString)
          case other => missing += s"JAVA_HOME's java is ${other.flatten.getOrElse("of no version")}, where the pins name $java"
    if lila then
      version("node", "--version") match
        case Some(v) if major(v) >= 24 => ()
        case v => missing += s"lila's ui needs node 24 or later (its package.json's engines), the PATH's is ${v.getOrElse("none")}"
      version("pnpm", "--version") match
        case Some(v) if major(v) >= 12 => ()
        case v => missing += s"lila's ui needs pnpm 12 or later (its package.json's engines), the PATH's is ${v.getOrElse("none")}"
    val m = missing.result()
    for v <- Option(System.getenv("TEQ_PLUGIN_VERSION")).filter(_.nonEmpty) do
      say(s"tools: the environment names TEQ_PLUGIN_VERSION=$v; the setup's exports and the gate's lines take the head's sbt-teq instead")
    m.foreach(w => println(s"FAIL setup: tools: $w"))
    if m.isEmpty then say(s"tools: git, curl, tar, node, npm${if sbtBuilt then ", sbt" else ""}${if lila then ", pnpm" else ""}, Java $java at ${System.getenv("JAVA_HOME")}")
    m.isEmpty

  private val pinned = Json.parse(Files.readString(Paths.get("bench/gate-projects/services.json")))

  // A release services.json pins, fetched into <dir>/downloads once and held to its digest (a partial or another
  // file fetched again).
  private def fetched(name: String): Option[Path] =
    val s = pinned(name)
    val file = dir.resolve("downloads").resolve(s("url").str.split('/').last)
    Files.createDirectories(file.getParent)
    def good = Files.isRegularFile(file) && Setup.sha256(Files.readAllBytes(file)) == s("sha256").str
    if !good then
      val part = file.resolveSibling(file.getFileName.toString + ".part")
      Files.deleteIfExists(part)
      say(s"services: fetching ${s("url").str}")
      val r = Sh("curl", "-sSfL", "--max-time", "600", "-o", part.toString, s("url").str).timeout(660).mergeErr.check(false).run()
      if !r.ok then { println(s"FAIL setup: services: ${s("url").str}: curl exits ${r.status}: ${r.out.trim}"); return None }
      Files.move(part, file, StandardCopyOption.REPLACE_EXISTING)
    if good then Some(file) else { println(s"FAIL setup: services: $file is not services.json's ${s("sha256").str}"); None }

  // An installed release's binary under <dir>/services/<name>, extracted (and for Redis built) afresh when absent.
  private def installed(name: String): Option[Path] =
    val s = pinned(name)
    val home = dir.resolve("services").resolve(name)
    val binary = home.resolve(s("binary").str)
    if Files.isExecutable(binary) then Some(binary)
    else fetched(name).flatMap { archive =>
      Sh.remove(home)
      Files.createDirectories(home)
      val log = logs.resolve(s"services-$name.log")
      val x = Sh("tar", "-x", "-z", "-f", archive.toString, "-C", home.toString, "--strip-components=1").stdout(log).mergeErr.timeout(600).check(false).run()
      if !x.ok then { println(s"FAIL setup: services: $archive does not extract (its log $log)"); None }
      else
        val built = s.get("build").forall { b =>
          if onPath("make").isEmpty || (onPath("cc").isEmpty && onPath("gcc").isEmpty) then
            println(s"FAIL setup: services: $name is built from its source, and make or a C compiler is missing")
            false
          else
            say(s"services: building $name (its log $log)")
            Sh((Seq("make", "-j4") ++ b.elements.map(_.str))*).in(home).stdout(log, append = true).mergeErr.timeout(1200).check(false).run().ok
        }
        if built && Files.isExecutable(binary) then Some(binary)
        else { println(s"FAIL setup: services: $name has no binary at $binary (its log $log)"); Sh.remove(home); None }
    }

  private def listening(port: Int): Boolean = Sh("bash", "-c", s"exec 3<>/dev/tcp/127.0.0.1/$port").timeout(5).check(false).run().ok

  // The processes an earlier run of this program left (an interruption), killed: the services' cleanup path.
  private def killLeftovers(): Unit =
    if Files.isDirectory(pids) then
      for f <- Files.list(pids).toArray.toVector.map(_.asInstanceOf[Path]) do
        for p <- Files.readString(f).trim.toLongOption do
          val h = ProcessHandle.of(p)
          if h.isPresent then Sh.killTree(h.get)
        Files.delete(f)

  private def services(): Boolean =
    killLeftovers()
    val taken = Setup.ports.filter(listening)
    if taken.nonEmpty then return failed("services", s"the ports ${taken.mkString(", ")} are taken: lila's lines take 27017, 6379 and 9663")
    val chosen = Vector("mongod" -> "LILA_MONGOD", "redis-server" -> "LILA_REDIS").map { (name, variable) =>
      val named = Option(System.getenv(variable)).filter(_.nonEmpty).map(Paths.get(_)).filter(Files.isExecutable(_)).orElse(onPath(name))
      name -> named.orElse(installed(if name == "mongod" then "mongodb" else "redis")).map(p => (variable, p))
    }
    if chosen.exists(_._2.isEmpty) then return false
    val data = outs.resolve("services")
    Sh.remove(data)
    Files.createDirectories(data.resolve("mongo"))
    Files.createDirectories(pids)
    val ok = chosen.forall { case (name, Some((variable, binary))) =>
      val (port, cmd) =
        if name == "mongod" then (27017, Seq(binary.toString, "--dbpath", data.resolve("mongo").toString, "--port", "27017", "--bind_ip", "127.0.0.1", "--quiet"))
        else (6379, Seq(binary.toString, "--port", "6379", "--bind", "127.0.0.1", "--save", "", "--appendonly", "no"))
      val v = version(binary.toString, "--version").getOrElse("")
      val proc = Sh(cmd*).withEnv("LC_ALL" -> "C").stdout(logs.resolve(s"services-$name-run.log")).mergeErr.timeout(300).spawn()
      Files.writeString(pids.resolve(name), proc.process.pid().toString)
      var left = 240
      while !listening(port) && proc.alive && left > 0 do
        Thread.sleep(250)
        left -= 1
      val ready = listening(port)
      proc.kill()
      Files.deleteIfExists(pids.resolve(name))
      if ready then
        env = env :+ (variable -> binary.toString)
        say(s"services: $name ready on $port ($binary, ${v.linesIterator.toVector.headOption.getOrElse("")}), stopped")
      else println(s"FAIL setup: services: $name did not listen on $port within 60 s (its log ${logs.resolve(s"services-$name-run.log")})")
      ready
    case _ => false }
    var left = 120
    while Setup.ports.exists(listening) && left > 0 do
      Thread.sleep(250)
      left -= 1
    val still = Setup.ports.filter(listening)
    if still.nonEmpty then failed("services", s"the ports ${still.mkString(", ")} are still taken after the services stopped") else ok

  // A project's driver and the arguments before its verb.
  private def script(p: String): Seq[String] = p match
    case "tyrian" | "indigo" => Seq("bench/mill-reference.scala", "--", p)
    case _ => Seq(s"bench/$p/$p.scala", "--")

  private def verb(p: String, v: String, seconds: Double): Boolean =
    val out = outs.resolve(p)
    Files.createDirectories(out)
    val log = logs.resolve(s"$p-$v.log")
    say(s"$p: $v (its log $log)")
    // Without TEQ_PLUGIN_VERSION, as the gate's lines run: the export publishes the head's sbt-teq for both sbt lines
    // (bench/reference/reference.scala `SbtPlugin`), so that the machine needs no plugin published by hand.
    val r = Sh((Seq(teq, "interp") ++ script(p) ++ Seq(v, dir.resolve(p).toString, out.toString, "--teq", teq))*)
      .withEnv(env.filter(_._1 == "JAVA_HOME")*).without("TEQ_PLUGIN_VERSION").stdout(log).mergeErr.timeout(seconds).check(false).run()
    if !r.ok then
      println(Files.readString(log).linesIterator.toVector.takeRight(15).map("  " + _).mkString("\n"))
      println(s"FAIL setup: $p: $v exits ${r.status}${if r.timedOut then s" (its deadline of ${seconds.toInt} s)" else ""}")
    r.ok

  private def lock(p: String): Path = p match
    case "indigo" => dir.resolve("indigo/indigo/target/teq/teq.lock")
    case _ => dir.resolve(p).resolve("target/teq/teq.lock")

  private def assets(): Boolean =
    val checkout = dir.resolve("lila")
    val manifest = checkout.resolve("public/compiled/manifest.json")
    val log = logs.resolve("lila-assets.log")
    say(s"assets: lila's ui/build (its log $log)")
    val r = Sh("bash", "ui/build").in(checkout).stdout(log).mergeErr.timeout(1800).check(false).run()
    if !r.ok then failed("assets", s"ui/build exits ${r.status} (its log $log)")
    else if !Files.isRegularFile(manifest) || !Files.readString(manifest).contains("\"js\"") then failed("assets", s"ui/build wrote no $manifest naming the scripts")
    else
      // Each script the manifest names, `<name>.<hash>.js`, and each it imports, is there.
      val compiled = manifest.getParent
      val scripts = Json.parse(Files.readString(manifest))("js").members.flatMap((name, entry) => entry.get("hash").toVector.flatMap(h =>
        s"$name.${h.str}.js" +: entry.get("imports").toVector.flatMap(_.elements.map(_.str))))
      val missing = scripts.distinct.filterNot(f => Files.isRegularFile(compiled.resolve(f)))
      if missing.nonEmpty then failed("assets", s"${missing.size} scripts the manifest names are not in $compiled: ${missing.take(3).mkString(", ")}")
      else if scripts.isEmpty then failed("assets", s"$manifest names no script")
      else { say(s"assets: the ${scripts.distinct.size} scripts the manifest names are there"); true }

  private def browser(): Boolean =
    val log = logs.resolve("browser.log")
    val r = Sh("node", "bench/smoke.mjs", "webgl").stdout(log).mergeErr.timeout(120).check(false).run()
    if r.ok then { say(s"browser: ${Files.readString(log).trim.linesIterator.toVector.lastOption.getOrElse("")}"); true }
    else failed("browser", s"headless Chrome makes no WebGL2 context: ${Files.readString(log).trim.linesIterator.toVector.takeRight(3).mkString(" / ")}")

  def run(): Boolean =
    Files.createDirectories(logs)
    Files.createDirectories(outs)
    Files.deleteIfExists(dir.resolve("ready"))
    Script.atExit(killLeftovers())
    say(s"${projects.mkString(", ")} under $dir")
    val toolsPassed = tools()
    val servicesPassed = !lila || toolsPassed && stage("services")(services())
    // Each project's stages in turn, one failing leaving the project's later stages undone and the other projects'
    // to run.
    val passed = projects.map { p =>
      val checkout = toolsPassed && stage(s"$p-checkout")(verb(p, "checkout", 1800))
      val exported = checkout && stage(s"$p-export")(verb(p, "export", 3600) && (Files.isRegularFile(lock(p)) || failed(s"$p-export", s"no lock at ${lock(p)}")))
      val built = p != "lila" || checkout && stage("lila-assets")(assets())
      p -> (exported && built && (p != "lila" || servicesPassed))
    }
    val browserPassed = !browsers || toolsPassed && browser()
    val busy = if lila then Setup.ports.filter(listening) else Vector.empty
    if busy.nonEmpty then println(s"FAIL setup: ports: ${busy.mkString(", ")} taken at the end")
    val notReady = passed.filterNot(_._2).map(_._1)
    val ok = toolsPassed && notReady.isEmpty && browserPassed && busy.isEmpty
    if ok then
      val vars = projects.map(p => s"GATE_${p.toUpperCase}_CHECKOUT" -> dir.resolve(p).toString) ++ env.distinctBy(_._1)
      Files.writeString(dir.resolve("gate-projects.env"), vars.map((k, v) => s"export $k=${quote(v)}\n").mkString)
      val pins = projects.map(p => s"$p ${Json.parse(Files.readString(Paths.get("bench", p, "pin.json")))("commit").str}")
      Files.writeString(dir.resolve("ready"), (pins :+ version(teq, "--version").getOrElse("teq of no version") :+ s"at ${Log.utc("yyyy-MM-dd'T'HH:mm:ss'Z'")}").mkString("", "\n", "\n"))
      say(s"ready: ${dir.resolve("ready")}; the machine's login shell sources ${dir.resolve("gate-projects.env")}:")
      vars.foreach((k, v) => println(s"  $k=$v"))
    else
      val what = (if toolsPassed then Vector.empty else Vector("the tools")) ++ notReady ++ (if browserPassed then Vector.empty else Vector("the browser"))
      println(s"FAIL setup: not ready: ${(what ++ busy.map(p => s"port $p")).mkString(", ")} (run again once fixed: every stage runs again)")
    ok

  private def quote(v: String): String = if v.matches("[A-Za-z0-9_./:@+-]+") then v else "'" + v.replace("'", "'\\''") + "'"
