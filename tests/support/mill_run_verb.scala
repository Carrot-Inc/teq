//> using file ../../bench/mill-reference.scala
// bench/mill-reference.scala's run verb held to its record (tests/mill-test-verb.sh): `Ref.run` with Tyrian's pin,
// generated.json, run.json and run/ files copied into <here>, a checkout <dir> holding the export's lock and the page,
// and the stand-in teq <bin>; bench/smoke.mjs runs under the PATH's node, the check's stand-in. Judged against the
// record <here> holds, or with a fifth argument `record` or `baseline` its --record or its --baseline.
import java.nio.file.Paths

@main def millRunVerb(argv: String*): Unit = Script.run {
  val Seq(here, dir, out, bin) = argv.take(4): @unchecked
  val mode = argv.lift(4) match
    case Some("record") => Mode.Record
    case Some("baseline") => Mode.Baseline
    case _ => Mode.Judge
  Ref.run(Pin.read("tyrian", Paths.get(here)), Paths.get(dir), Paths.get(out), bin, mode)
}
