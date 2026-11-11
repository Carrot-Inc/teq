// A downstream of a/ as tests/modules builds it, whole and over teq's products. `t()` leaves out the parameter of a
// top-level export of an inline method whose default is an inline getter. The JVM build over the products prints
// the .expected, scalac 3.8.4's (it threw NoSuchMethodError on `Impl$.t$default$1` until the change that expands a
// product's inline getter as an inline call). The Scala.js products build (`compiler check --products`, the lean
// std) refuses the call: "X.scala:13:12: missing argument for parameter x of method t: (x: Int @RuntimeChecked): Int",
// so the js and interpreter runs over the products fail. The same export in an object (`object Fw: export Impl.t`,
// tests/modules/exportinlinedefaults) runs there. What differs is the path a product's top-level definitions take
// outside the JVM: the loader enters them as the package's, typed as a pseudo file of their own
// (`Worker::check_product_package`, src/typer/loader/mod.rs, `product && !self.jvm && !self.writes_products`), where
// the forwarder's parameter has no default. Moves to tests/modules with the change that keeps it.
import lib.*
@main def run(): Unit =
  println(t())
