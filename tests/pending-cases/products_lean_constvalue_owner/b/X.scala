// A downstream of a/ as tests/modules builds it. a/'s inline methods call `constValue`, written by its path and
// imported, and `erasedValue`. scalac 3.8.4 prints the .expected, and so does each of teq's runs. scalac typing b/ over
// teq's products of a/ built with the lean std (tests/modules' scalac-lean-b) fails at each call: "undefined:
// scala.compiletime.compiletime$package.constValue" (and `.erasedValue`). The lean pickle names the member's owner by
// the lean std's file object (std/compiletime.scala's `compiletime$package`), where scala-library's is
// `scala.compiletime.package$package`. Over the JVM products (scalac-b) it passes. Found beside the SELECTtpt port of
// the workers exports helper; the same on integration b602956824.
package cvb

@main def run(): Unit =
  println(cva.A.five + cva.A.six)
  println(cva.A.kind[Int])
