// A downstream of a/ as tests/modules builds
// it, reduced from tests/modules/boundederasure,
// with a JDK class's array beside them.
// scalac 3.8.4 prints the .expected.
// scalac's typer passes each `Array(...)`
// its ClassTag (`ClassTag.apply[TimeoutException](classOf[TimeoutException])`,
// `ClassTag.apply[String](classOf[String])`, `ClassTag.apply[V](classOf[V])`), which
// its Scala.js back end keeps. teq's Scala.js
// bundles, the whole program's and b/'s over a/'s lean
// products, are to be the same, under --release too.
package btb

import bta.*
import java.util.concurrent.TimeoutException

@main def run(): Unit =
  println(Api.a1(Array("x", "y")))
  println(Api.a5(Array(new V(6))))
  println(Api.plain(Array("p")))
  println(Array(new TimeoutException()).length)
