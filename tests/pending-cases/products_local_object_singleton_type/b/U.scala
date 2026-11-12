// A downstream of a/ as tests/modules builds it. scalac 3.8.4
// prints the .expected. `local`'s result is a tuple of its local
// objects: the Scala.js and interpreter builds over teq's lean
// products are to print the .expected too (js-run, interp-run).
package lob

@main def run(): Unit =
  println(loa.local().toString.nonEmpty)
