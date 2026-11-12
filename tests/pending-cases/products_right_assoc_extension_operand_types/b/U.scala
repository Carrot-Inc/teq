// A downstream of a/ as tests/modules builds
// it. scalac 3.8.4 prints the .expected. Over
// a/'s products, JVM and Scala.js alike, teq
// is to accept the calls of a right-associative
// extension whose operands differ in type,
// generic (`+::`) or not (`+:::`) (tests/modules'
// build), and the direct call in dotty's
// encoded order, `G.+:::(List(2))(1)`.
package gb
import ga.G
import ga.G.*
@main def run(): Unit =
  println(1 +:: List(2))
  println(1 +::: List(2))
  println(1 +++ List(2))
  println(G.+:::(List(2))(1))
