//> using file ../../tools/script
//> using file ../../bench/reference/reference.scala
//> using file ../../bench/reference/scalajs.scala
// The preflight every verb of a Scala.js reference project runs (tests/reference-test-verb.sh): Airstream's
// `ScalaJsReference.atPin` over the checkout <dir>, which refuses a directory whose name sbt makes another root
// project of Airstream's build, which names none.
import java.nio.file.Paths

@main def referencePreflight(argv: String*): Unit = Script.run {
  val Seq(dir) = argv: @unchecked
  new ScalaJsReference("airstream").atPin(Paths.get(dir))
}
