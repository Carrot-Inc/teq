// A downstream of a/ as tests/modules
// builds it. a/'s inline body ascribes to
// `Predef.String`, a type selection on the
// std's Predef. scalac 3.8.4 prints the
// .expected. teq's products of a/ built with
// the lean std are to be ones scalac types
// b/ over (tests/modules' scalac-lean-b).
package pdb

@main def run(): Unit =
  println(pda.A.str("abcd"))
