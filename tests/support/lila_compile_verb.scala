//> using file ../../bench/lila/lila.scala
// bench/lila/lila.scala's compile verb held to its records (tests/lila-test-verb.sh): `Lila.compile` with the pin,
// errors.txt and generated.json in <here>, a checkout <dir> holding the export's lock and the stand-in teq <bin>, in
// the mode the fifth argument names (judge, record, baseline, or migrate:<reason>), over the runs a sixth names
// (default 1).
import java.nio.file.Paths

@main def lilaCompileVerb(argv: String*): Unit = Script.run {
  val Seq(here, dir, out, bin, mode) = argv.take(5): @unchecked
  val runs = argv.lift(5).map(_.toInt).getOrElse(1)
  val m = mode match
    case "judge" => Mode.Judge
    case "record" => Mode.Record
    case "baseline" => Mode.Baseline
    case other => Mode.Migrate(other.stripPrefix("migrate:"))
  val c = Lila.compile(Pin.read(Paths.get("bench/lila")), Paths.get(here), Paths.get(dir), Paths.get(out), bin, runs, m)
  Script.exit(if c.failed then 1 else 0)
}
