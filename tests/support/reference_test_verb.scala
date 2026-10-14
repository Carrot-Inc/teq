//> using file ../../tools/script
//> using file ../../bench/reference/reference.scala
//> using file ../../bench/reference/scalajs.scala
// The test verb of the Scala.js reference projects held to an inventory (tests/reference-test-verb.sh): Airstream's
// `ScalaJsReference.test` over the inventory <inventory>, a checkout <dir> and the stand-in teq <bin>, whose
// transcript the verb compares.
import java.nio.file.Paths

@main def referenceTestVerb(argv: String*): Unit = Script.run {
  val Seq(inventory, dir, out, bin) = argv: @unchecked
  new ScalaJsReference("airstream").test(Paths.get(dir), Paths.get(out), bin, 1, Paths.get(inventory))
}
