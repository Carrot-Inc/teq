// jars: scala-library
// std: scala-library
// targets: jvm
// The JVM target's linking: scala-library's bytecode as the JVM std.
// Implicit conversions to scala-library's value classes: scalac's descriptors return the
// underlying type (augmentString: String => String, intWrapper: I => I, ArrowAssoc: Object =>
// Object) and calls go to the companion's `m$extension`; teq's return the box class.
import scala.concurrent.duration.*
@main def run(): Unit =
  println("a,b".split(",").length)
  println((1 -> "one")._2)
  println((1 to 3).sum)
  println(5.seconds)
  println("abc".reverse)
