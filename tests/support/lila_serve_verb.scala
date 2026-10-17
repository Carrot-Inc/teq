//> using file ../../bench/lila/lila.scala
// bench/lila/lila.scala's run of a serving program held to its record (tests/lila-test-verb.sh): `Lila.serveOnce`
// over the checkout <dir> (an export whose project lila serves its home page on port 9663) with the binary <bin>, its
// output and records under <out>, held by `Lines.hold` to the run.json in <here> in the mode the last argument names
// (judge, record, baseline), as `Lila.serve` holds each run.
import java.nio.file.{Files, Paths}

@main def lilaServeVerb(argv: String*): Unit = Script.run {
  val Seq(here, dir, out, bin, mode) = argv: @unchecked
  val (checkout, logs) = (Paths.get(dir), Paths.get(out))
  Files.createDirectories(logs)
  val roots = Roots.of(checkout, checkout, logs)
  val log = logs.resolve("run-1.log")
  val (observed, _) = Lila.serveOnce(bin, checkout, logs, log, "TEQ_SERVE_VERB" -> "1", roots)
  val m = mode match
    case "record" => Mode.Record
    case "baseline" => Mode.Baseline
    case _ => Mode.Judge
  val version = Sh(bin, "--version").timeout(30).check(false).run().out.trim
  Script.exit(if Lines.hold("lila", "run", Paths.get(here), Shape.Run, Vector("run 1" -> observed), roots(Files.readString(log)), m, version) then 0 else 1)
}
