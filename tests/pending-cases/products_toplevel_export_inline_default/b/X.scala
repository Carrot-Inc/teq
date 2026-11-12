// A downstream of a/ as tests/modules builds it, whole and
// over the products. `t()` leaves out the parameter of a
// top-level export of an inline method whose default is
// an inline getter. The JVM build over the products is to
// print the .expected, scalac 3.8.4's. The Scala.js products
// build (`compiler check --products`, the lean std) is to
// accept the call, and the js and interpreter runs over the
// products are to print it too. The same export in an object
// (`object Fw: export Impl.t`) is tests/modules/exportinlinedefaults.
// Moves to tests/modules with the change that keeps the parameter's default.
import lib.*
@main def run(): Unit =
  println(t())
