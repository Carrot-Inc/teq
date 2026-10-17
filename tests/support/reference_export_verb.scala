//> using file ../../tools/script
//> using file ../../bench/reference/reference.scala
//> using file ../../bench/reference/scalajs.scala
//> using file ../../bench/reference/records.scala
// The export verb of the Scala.js reference projects (tests/reference-test-verb.sh): Airstream's
// `ScalaJsReference.exportLock` over a checkout <dir> into <out> with the binary <bin>, sbt and JAVA_HOME the caller's.
import java.nio.file.{Files, Paths}

@main def referenceExportVerb(argv: String*): Unit = Script.run {
  val Seq(dir, out, bin) = argv: @unchecked
  Files.createDirectories(Paths.get(out))
  new ScalaJsReference("airstream").exportLock(Paths.get(dir), Paths.get(out), bin)
}
