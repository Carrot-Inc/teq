// A downstream of a/ as tests/modules builds it. a/'s inline body ascribes to `Predef.String`, a type selection on the
// std's Predef, which teq pickles as SELECTtpt over the qualifier's tree (TreePickler.pickleTree's Select case).
// scalac 3.8.4 prints the .expected, and so does each of teq's runs. scalac typing b/ over teq's products of a/ built
// with the lean std (tests/modules' scalac-lean-b) fails: "Found: scala.type, Required: ?{ String: ? }". In the lean
// pickle the qualifier `Predef` is the package `scala`, where the lean std keeps Predef's members, and scala-library
// has no `scala.String`. Over the JVM products (scalac-b) it passes. Found beside the SELECTtpt port of the workers
// exports helper; before it the products build panicked on the type selection.
package pdb

@main def run(): Unit =
  println(pda.A.str("abcd"))
