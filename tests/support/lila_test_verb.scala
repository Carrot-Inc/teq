//> using file ../../bench/lila/lila.scala
// bench/lila/lila.scala's test verb held to an inventory (tests/lila-test-verb.sh): `Lila.test` over the
// inventory and deviations.json in <here>, a checkout <dir> and the stand-in teq <bin>, whose transcript the verb
// compares, in the mode a fifth argument names (baseline, record; judge without one).
import java.nio.file.Paths

@main def lilaTestVerb(argv: String*): Unit = Script.run {
  val Seq(here, dir, out, bin) = argv.take(4): @unchecked
  val mode = argv.lift(4) match
    case Some("baseline") => Mode.Baseline
    case Some("record") => Mode.Record
    case _ => Mode.Judge
  val m = Lila.test(Pin.read(Paths.get("bench/lila")), Paths.get(here), Paths.get(dir), Paths.get(out), bin, 1, mode)
  Script.exit(if m.failed then 1 else 0)
}
