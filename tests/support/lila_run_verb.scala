//> using file ../../bench/lila/lila.scala
// bench/lila/lila.scala's run held to its record (tests/lila-test-verb.sh): `Lila.observeRun` over a run's output
// <log> (its builds' diagnostics as teq records them in <log>.diagnostics), the home page's status, how `teq run` ended (a status, or `running` for a run still running at the readiness
// deadline) and the assertions' outcomes <shown> (a line each, `assert <name>: <outcome>`), held by `Lines.hold` to the
// run.json in <here> in the mode the last argument names (judge, record, baseline).
import java.nio.file.{Files, Paths}

@main def lilaRunVerb(argv: String*): Unit = Script.run {
  val Seq(here, log, status, ended, shown, mode) = argv: @unchecked
  val roots = Roots(Vector.empty)
  val observed = Lila.observeRun(status.toInt, ended.toIntOption, Files.readString(Paths.get(log)), Transcript.recorded(Paths.get(log + ".diagnostics")), Files.readString(Paths.get(shown)).linesIterator.toVector, roots)
  val m = mode match
    case "record" => Mode.Record
    case "baseline" => Mode.Baseline
    case _ => Mode.Judge
  Script.exit(if Lines.hold("lila", "run", Paths.get(here), Shape.Run, Vector("run 1" -> observed), Files.readString(Paths.get(log)), m, "teq stand-in") then 0 else 1)
}
