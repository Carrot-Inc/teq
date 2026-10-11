//> using file ../tools/script
// The oracle of `teq test` over a Scala.js project: sbt-scalajs's run of tests/scalajs-test's suites under each
// pinned tuple, recorded in tests/scalajs-test/oracle-<Scala.js version>.txt, which tests/task.sh compares teq's
// run with. Each tuple runs over a copy of the fixture (its jsdom installed by `npm ci` from package-lock.json),
// `oracleDefined` writing the suites sbt's discovery defines and the build's listener every event, one line each:
// the project, the suite, the selector (or `defined`), the status and the first line of the throwable, sorted.
// Needs sbt, node and npm and takes minutes; not a suite of tests/all.sh.
// `./teq interp tests/scalajs-oracle.scala [-- --tuple 1.22.0|1.20.1]`
import java.nio.file.{Files, Path, Paths, StandardCopyOption}
import scala.jdk.CollectionConverters.*

// The tuples: Scala.js, munit, and whether zio-test 2.1.26 (built on Scala.js 1.21) is in.
val tuples = Vector(("1.22.0", "1.3.4", true), ("1.20.1", "1.2.1", false))

def copyTree(from: Path, to: Path): Unit =
  val s = Files.walk(from)
  try
    for p <- s.iterator.asScala do
      val rel = from.relativize(p)
      val skip = rel.iterator.asScala.exists(n => n.toString == "node_modules" || n.toString == "target")
      if !skip then
        if Files.isDirectory(p) then Files.createDirectories(to.resolve(rel))
        else if !p.getFileName.toString.startsWith("oracle-") then Files.copy(p, to.resolve(rel), StandardCopyOption.REPLACE_EXISTING)
  finally s.close()

@main def scalajsOracle(argv: String*): Unit = Script.run {
  val args = new Args(argv, "tests/scalajs-oracle.scala [--tuple <Scala.js version>]")
  val only = args.option("tuple")
  args.exactly(0)
  val fixture = Paths.get("tests/scalajs-test")
  for (scalajs, munit, zio) <- tuples if only.forall(_ == scalajs) do
    val work = Files.createTempDirectory("teq-scalajs-oracle-")
    Script.atExit(Sh.remove(work))
    copyTree(fixture, work)
    Log.stamped(s"Scala.js $scalajs, munit $munit${if zio then ", zio-test" else ""}: npm ci in $work")
    Sh("npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund").in(work).timeout(300).run()
    val props = Vector(s"-Dfixture.scalajs=$scalajs", s"-Dfixture.munit=$munit", s"-Dfixture.zio=$zio")
    val sbt = Sh(Vector("sbt", "-batch") ++ props ++ Vector("oracleDefined", "test")*).in(work).timeout(1200).check(false).stdout(work.resolve("sbt.log")).run()
    Files.copy(work.resolve("sbt.log"), Paths.get(s"out/scalajs-oracle-$scalajs.log"), StandardCopyOption.REPLACE_EXISTING)
    val events = work.resolve("target/oracle-events.txt")
    if !Files.exists(events) then Log.die(s"sbt recorded nothing (status ${sbt.status}):\n${sbt.out.linesIterator.toVector.takeRight(40).mkString("\n")}\n${sbt.errTail}")
    val lines = Files.readAllLines(events).asScala.toVector.sorted
    val out = fixture.resolve(s"oracle-$scalajs.txt")
    Files.write(out, (lines.mkString("\n") + "\n").getBytes("UTF-8"))
    Log.stamped(s"$out: ${lines.size} lines (sbt exited ${sbt.status})")
}
