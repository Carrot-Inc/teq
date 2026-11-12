// A downstream of a/ as tests/modules
// builds it. Two macro implementations of
// a/ each take their own `using Quotes`
// (`x$1`) and read `TypeRepr.of[T]`.
// scalac 3.8.4 prints the .expected. teq's
// pickle of a/ is to be one scalac reads
// (tests/modules' readtasty-a, `-Ycheck`).
package qrb

@main def run(): Unit =
  println(qra.Mac.nameOf[Int])
