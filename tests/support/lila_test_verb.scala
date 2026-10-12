//> using file ../../bench/lila/lila.scala
// bench/lila/lila.scala's test verb held to an inventory (tests/lila-test-verb.sh): `Lila.test` over the
// inventory in <here>, a checkout <dir> and the stand-in teq <bin>, whose transcript the verb compares.
import java.nio.file.Paths

@main def lilaTestVerb(argv: String*): Unit = Script.run {
  val Seq(here, dir, out, bin) = argv: @unchecked
  Lila.test(Pin.read(Paths.get("bench/lila")), Paths.get(here), Paths.get(dir), Paths.get(out), bin, 1)
}
