//> using file ../../tools/script
//> using file ../../bench/reference/reference.scala
//> using file ../../bench/reference/scalajs.scala
//> using file ../../bench/reference/records.scala
// The compile verb of the Scala.js reference projects held to its record (tests/reference-test-verb.sh): Airstream's
// `ScalaJsReference.compile` with the pin and errors.txt in <here>, a checkout <dir> and the stand-in teq <bin>, in
// the mode the last argument names (judge, record, baseline, or migrate:<reason>).
import java.nio.file.Paths

@main def referenceCompileVerb(argv: String*): Unit = Script.run {
  val Seq(here, dir, out, bin, mode) = argv: @unchecked
  val m = mode match
    case "judge" => Mode.Judge
    case "record" => Mode.Record
    case "baseline" => Mode.Baseline
    case other => Mode.Migrate(other.stripPrefix("migrate:"))
  new ScalaJsReference("airstream", Paths.get(here)).compile(Paths.get(dir), Paths.get(out), bin, 1, m)
}
