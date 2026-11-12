// A downstream of a/ as tests/modules builds it. scalac 3.8.4
// prints the .expected. `Lib`'s bodies call a/'s top-level
// overloads: a run of them over teq's products of a/ is to
// print the .expected too (the interpreter over the check
// build's products; the JavaScript over the Scala.js products).
package tou

@main def run(): Unit =
  println(tov.Lib.topLiteral)
  println(tov.Lib.topPlain)
