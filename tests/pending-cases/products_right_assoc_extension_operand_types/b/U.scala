// A downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected, and so do teq's one-file and
// whole builds. Over a/'s products, JVM and Scala.js alike, teq rejects the calls of a right-associative extension
// whose operands differ in type, generic (`+::`) or not (`+:::`): "value +:: is not a member of List[Int]"
// (tests/modules' build), and the direct call in dotty's encoded order, `G.+:::(List(2))(1)`, with "type mismatch:
// found List[Int], required Int". A generic extension that is not right-associative (`+++`) is accepted. The
// parameters' order is likely products_right_assoc_extension_params' disagreement, where operands of one type pass
// either way and run swapped (reviewer #15). The same on integration a097de53d9. Found by reviewer #15 of the
// workers exports helper (push 9).
package gb
import ga.G
import ga.G.*
@main def run(): Unit =
  println(1 +:: List(2))
  println(1 +::: List(2))
  println(1 +++ List(2))
  println(G.+:::(List(2))(1))
