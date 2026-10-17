//> using file ../../../../../../tools/script
// Spike 2's proof, replayable: master's `tests/run_stdlib.sh` (the old runner) and the spike's
// `tests/run_stdlib.scala` (the new) over one tree and one binary, on a subset of tests/cases with failures planted,
// a cache without scala-library, `--update`, a missing binary, a logging wrapper for the binary and a SIGTERM
// mid-run. Run from the repository's root:
//
//   ./teq interp bench/survey/evidence/spikes/case-runner/recipe/plant.scala -- --teq <binary> --work <dir>
//     [--spike <commit>] [--master <commit>]
//
// --teq is the compiler both runners test, which also interprets the new runner (the way tests/all.sh runs a suite
// in Scala: `TEQ=<binary> ./teq interp ...`); --work a directory that does not exist yet, which takes the spike's
// tree (`git archive` of --spike, default 227d531cd, the old runner from --master, default 0e2ef0f12, beside it as
// tests/run_stdlib_old.sh) and every output: `<phase>-<old|new>.out` (the runner's output and `exit <status>`), the
// runners' `out/stdlib-tests`, emptied before each run, as `<phase>-<old|new>-tests/`, and `summary.txt`. The subset is `subset.txt`'s names:
// every entry of tests/cases whose name, without `.scala` or `.expected`, is one of them (46 entries at 227d531cd),
// the passing list and the summaries cut to them. The phases:
//
// clean      the subset as it is
// planted    `planted/`'s two cases (an endless loop, a compile error) with their expectations added, a listed
//            case's expectation broken (capture_release_ctor_param), a listed pass unlisted (access_qualifiers), a
//            recorded summary changed (either_right's 10 deprecation warnings to 11), STDLIB_REASONS=1
// nojar      the planted tree with COURSIER_CACHE naming no directory
// update     `--update`, the list saved and put back after each runner
// missing    the binary a path that does not exist
// wrapper    the new runner with --teq a wrapper that logs each call and runs the binary
// cancel     each runner its session's leader, SIGTERM to it 8 s after it starts, then the processes its session
//            still holds a second later (killed then): `cancel-<side>.out` what it printed and its status,
//            `cancel-<side>.left` the processes left
//
// What each runner printed and wrote is compared: the same lines (the old runner's `diff` hunk lines, `1c1` and
// `---`, left out of its summaries' difference), the same `passing.txt`, `summaries.txt` and `.actual` files, the
// same lists after `--update`. summary.txt has a line per phase; the status is 1 when a comparison differs, when
// the wrapper saw another count of builds than the cases selected, or when the new runner does not end on SIGTERM
// with 143 and its session empty. The old runner's children left after SIGTERM are recorded, not failed: they are
// the difference the spike claims.
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Path, Paths}

@main def plant(argv: String*): Unit = Script.run {
  val args = new Args(argv, "bench/survey/evidence/spikes/case-runner/recipe/plant.scala --teq <binary> --work <dir> [--spike <commit>] [--master <commit>]")
  val teq = Paths.get(args.option("teq").getOrElse(args.fail("--teq is required"))).toAbsolutePath.toString
  val work = Paths.get(args.option("work").getOrElse(args.fail("--work is required"))).toAbsolutePath
  val spike = args.option("spike").getOrElse("227d531cd")
  val master = args.option("master").getOrElse("0e2ef0f12")
  args.exactly(0)
  val recipe = Paths.get("bench/survey/evidence/spikes/case-runner/recipe").toAbsolutePath
  if Files.exists(work) then Log.die(s"plant: $work exists; the recipe takes a directory of its own")
  val tree = work.resolve("tree")
  Files.createDirectories(tree)
  val tar = work.resolve("spike.tar")
  Sh("git", "archive", "--format=tar", "-o", tar.toString, spike).timeout(120).run()
  Sh("tar", "-xf", tar.toString, "-C", tree.toString).timeout(120).run()
  Files.delete(tar)
  val old = tree.resolve("tests/run_stdlib_old.sh")
  Files.writeString(old, Sh("git", "show", s"$master:tests/run_stdlib.sh").timeout(60).run().outAll())
  Sh("chmod", "+x", old.toString).run()

  // The subset.
  val names = lines(recipe.resolve("subset.txt")).filter(_.nonEmpty).toSet
  val cases = tree.resolve("tests/cases")
  for p <- list(cases) do
    val n = p.getFileName.toString.stripSuffix(".scala").stripSuffix(".expected")
    if !names.contains(n) then Sh.remove(p)
  val listFile = tree.resolve("tests/stdlib-passing.txt")
  val summariesFile = tree.resolve("tests/stdlib-summaries.txt")
  write(listFile, lines(listFile).filter(names.contains).sorted)
  write(summariesFile, lines(summariesFile).filter(l => names.contains(l.takeWhile(_ != ':'))))
  val entries = list(cases).length

  val results = Vector.newBuilder[String]
  var failed = false
  def note(line: String, ok: Boolean): Unit =
    results += (if ok then "ok    " else "FAIL  ") + line
    if !ok then failed = true
  val oldCmd = Vector("tests/run_stdlib_old.sh")
  val newCmd = Vector("./teq", "interp", "tests/run_stdlib.scala", "--")
  // A runner's run: its output and status into <phase>-<side>.out, its out/stdlib-tests copied beside.
  def run(phase: String, side: String, cmd: Vector[String], env: Map[String, String] = Map.empty): (String, Int) =
    Sh.remove(tree.resolve("out/stdlib-tests"))
    val r = Sh(cmd*).in(tree).withEnv((Map("TEQ" -> teq, "LC_COLLATE" -> "C") ++ env).toSeq*).mergeErr.timeout(600).check(false).run()
    val text = r.outAll()
    Files.writeString(work.resolve(s"$phase-$side.out"), text + s"exit ${r.status}\n")
    val results = tree.resolve("out/stdlib-tests")
    if Files.isDirectory(results) then
      val copy = work.resolve(s"$phase-$side-tests")
      Files.createDirectories(copy)
      for f <- list(results) if Files.isRegularFile(f) do Files.copy(f, copy.resolve(f.getFileName))
    (text, r.status)
  def hunkless(text: String): Vector[String] = text.linesIterator.toVector.filterNot(l => l.matches("[0-9]+(,[0-9]+)?[acd][0-9]+(,[0-9]+)?") || l == "---")
  def sameFiles(phase: String): Boolean =
    val (a, b) = (work.resolve(s"$phase-old-tests"), work.resolve(s"$phase-new-tests"))
    val wanted = (p: Path) => { val n = p.getFileName.toString; n == "passing.txt" || n == "summaries.txt" || n.endsWith(".actual") }
    val (fa, fb) = (list(a).filter(wanted).map(_.getFileName.toString), list(b).filter(wanted).map(_.getFileName.toString))
    fa == fb && fa.forall(n => java.util.Arrays.equals(Files.readAllBytes(a.resolve(n)), Files.readAllBytes(b.resolve(n))))
  def compare(phase: String, env: Map[String, String] = Map.empty, extra: Vector[String] = Vector.empty): Unit =
    val (to, so) = run(phase, "old", oldCmd ++ extra, env)
    val (tn, sn) = run(phase, "new", newCmd ++ extra, env)
    val sameLines = hunkless(to) == hunkless(tn)
    val files = sameFiles(phase)
    note(s"$phase: exit $so and $sn, lines ${if sameLines then "the same" else "different"}, files ${if files then "the same" else "different"}; " +
      s"old's last: ${to.linesIterator.toVector.lastOption.getOrElse("")}", so == sn && sameLines && files)

  results += s"the tree: $spike with $master's tests/run_stdlib.sh beside it, $entries entries of tests/cases for ${names.size} names; the binary ${Sh(teq, "--version").check(false).run().out.trim}"
  compare("clean")

  // The plants.
  for f <- list(recipe.resolve("planted")) do Files.copy(f, cases.resolve(f.getFileName))
  Files.writeString(cases.resolve("capture_release_ctor_param.expected"), "planted\n", java.nio.file.StandardOpenOption.APPEND)
  write(listFile, lines(listFile).filterNot(_ == "access_qualifiers"))
  write(summariesFile, lines(summariesFile).map(_.replace("there were 10 deprecation", "there were 11 deprecation")))
  compare("planted", Map("STDLIB_REASONS" -> "1"))
  compare("nojar", Map("COURSIER_CACHE" -> work.resolve("no-cache").toString))

  // --update, the list put back after each runner.
  val kept = Files.readAllBytes(listFile)
  val (_, uo) = run("update", "old", oldCmd :+ "--update")
  val listOld = Files.readAllBytes(listFile)
  Files.write(listFile, kept)
  val (_, un) = run("update", "new", newCmd :+ "--update")
  val listNew = Files.readAllBytes(listFile)
  Files.write(listFile, kept)
  Files.write(work.resolve("update-old.list"), listOld)
  Files.write(work.resolve("update-new.list"), listNew)
  note(s"update: exit $uo and $un, the lists ${if java.util.Arrays.equals(listOld, listNew) then "the same" else "different"} (${new String(listNew, UTF_8).linesIterator.length} names)",
    java.util.Arrays.equals(listOld, listNew) && uo == un)

  // A binary that does not exist: the old runner by TEQ, the new by --teq, interpreted by the binary still.
  val (mo, so) = run("missing", "old", oldCmd, Map("TEQ" -> "/nonexistent/teq"))
  val (mn, sn) = run("missing", "new", newCmd ++ Vector("--teq", "/nonexistent/teq"))
  note(s"missing: exit $so and $sn; old's last: ${mo.linesIterator.toVector.lastOption.getOrElse("")}; new's: ${mn.linesIterator.toVector.headOption.getOrElse("")}", so == 1 && sn == 1)

  // The harness apart from the binary: a wrapper that logs every call.
  val calls = work.resolve("wrapper-calls.log")
  val wrapper = work.resolve("wrapper")
  Files.writeString(wrapper, s"#!/bin/sh\necho \"$$*\" >> '$calls'\nexec '$teq' \"$$@\"\n")
  Sh("chmod", "+x", wrapper.toString).run()
  run("wrapper", "new", newCmd ++ Vector("--teq", wrapper.toString), Map("STDLIB_REASONS" -> "1"))
  val built = if Files.exists(calls) then lines(calls).count(_.startsWith("compiler build")) else 0
  val selected = list(work.resolve("wrapper-new-tests")).count(_.toString.endsWith(".actual"))
  note(s"wrapper: $built builds through the wrapper for $selected cases selected", built == selected && built > 0)

  // SIGTERM 8 s in, each runner its session's leader.
  for (side, cmd) <- Vector("old" -> oldCmd, "new" -> newCmd) do
    val sid = work.resolve(s"cancel-$side.sid")
    val running = Sh(("setsid" +: "-w" +: "bash" +: "-c" +: s"echo $$$$ > '$sid'; exec \"$$@\"" +: "cancel" +: cmd)*)
      .in(tree).withEnv("TEQ" -> teq, "LC_COLLATE" -> "C").mergeErr.stdout(work.resolve(s"cancel-$side.out")).timeout(120).check(false).spawn()
    var waited = 0
    while !(Files.exists(sid) && Files.size(sid) > 0) && waited < 100 do
      Thread.sleep(100)
      waited += 1
    val leader = Files.readString(sid).trim
    Thread.sleep(8000)
    Sh("kill", "-TERM", leader).check(false).run()
    val r = running.await()
    Files.writeString(work.resolve(s"cancel-$side.out"), s"exit ${r.status}\n", java.nio.file.StandardOpenOption.CREATE, java.nio.file.StandardOpenOption.APPEND)
    Thread.sleep(1000)
    val left = Sh("ps", "-o", "pid=,args=", "-s", leader).check(false).run().out.linesIterator.map(_.trim).filter(_.nonEmpty).toVector
    left.foreach(l => Sh("kill", "-9", l.takeWhile(_ != ' ')).check(false).run())
    Files.writeString(work.resolve(s"cancel-$side.left"), if left.isEmpty then "none\n" else left.mkString("", "\n", "\n"))
    val line = s"cancel, $side: exit ${r.status}, ${left.length} processes of its session left a second later${left.map(l => "\n        " + l.take(120)).mkString}"
    if side == "new" then note(line, r.status == 143 && left.isEmpty) else results += "      " + line

  val summary = results.result()
  Files.writeString(work.resolve("summary.txt"), summary.mkString("", "\n", "\n"))
  summary.foreach(println)
  Script.exit(if failed then 1 else 0)
}

def lines(p: Path): Vector[String] = if Files.exists(p) then new String(Files.readAllBytes(p), UTF_8).linesIterator.toVector else Vector.empty
def write(p: Path, ls: Vector[String]): Unit = Files.writeString(p, ls.map(_ + "\n").mkString)
// A directory's entries in byte order, none for a directory a runner did not write.
def list(dir: Path): Vector[Path] =
  if !Files.isDirectory(dir) then return Vector.empty
  val s = Files.list(dir)
  try s.toArray.toVector.map(_.asInstanceOf[Path]).sortBy(_.getFileName.toString)
  finally s.close()
