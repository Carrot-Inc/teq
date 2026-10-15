//> using file ../../bench/mill-reference.scala
// bench/mill-reference.scala's test verb held to an inventory (tests/mill-test-verb.sh): `Ref.test` with the
// pin and the inventory in <here>, a checkout <dir> holding the export's lock and the stand-in teq <bin>, whose
// listing and transcript the verb compares.
import java.nio.file.Paths

@main def millTestVerb(argv: String*): Unit = Script.run {
  val Seq(here, dir, out, bin) = argv: @unchecked
  Ref.test(Pin.read("tyrian", Paths.get(here)), Paths.get(dir), Paths.get(out), bin, 1)
}
