// A downstream of a/ as tests/modules builds it. a/'s
// inline methods call `constValue`, written by its
// path and imported, and `erasedValue`. scalac 3.8.4
// prints the .expected. teq's products of a/ built with
// the lean std are to be ones scalac types b/ over
// (tests/modules' scalac-lean-b). The member's owner in
// scala-library is `scala.compiletime.package$package`.
package cvb

@main def run(): Unit =
  println(cva.A.five + cva.A.six)
  println(cva.A.kind[Int])
