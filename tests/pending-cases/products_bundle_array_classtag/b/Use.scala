// A downstream of a/ as tests/modules builds it, reduced from tests/modules/boundederasure, with a JDK class's array
// beside them. scalac 3.8.4 prints the .expected, and so does each of teq's runs. scalac's typer passes each
// `Array(...)` its ClassTag (`ClassTag.apply[TimeoutException](classOf[TimeoutException])`, `ClassTag.apply[String]
// (classOf[String])`, `ClassTag.apply[V](classOf[V])`), which its Scala.js back end keeps. teq's Scala.js bundles pass
// `Array.apply` `null`, the whole program's and b/'s over a/'s lean products alike, master's too. Until integration
// 33953b6ff the whole bundle passed the tag and the products bundle `null`; since A4 (the ClassTag module recorded at
// start-up and carried by the workers' merge) both pass `null`. A method that reads its ClassTag (`ct != null`) gets
// its tag in both builds: teq elides the tag where the callee, its std's `Array.apply`, ignores it. Under --release
// the products build also names 17 more members, by their slots those of java.lang.System that teq's std System
// lacks (reviewer #15; not traced), the mechanism of products_release_names_completion.
package btb

import bta.*
import java.util.concurrent.TimeoutException

@main def run(): Unit =
  println(Api.a1(Array("x", "y")))
  println(Api.a5(Array(new V(6))))
  println(Api.plain(Array("p")))
  println(Array(new TimeoutException()).length)
