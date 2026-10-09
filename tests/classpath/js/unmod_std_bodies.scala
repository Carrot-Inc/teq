// jars: fixtures
//> using platform js
// Library bodies that name scala-library's `sys` package, `List.reverse_:::`, `NameTransformer`,
// and Scala.js's `js.Symbol`, `js.PropertyDescriptor` and `js.Object`'s companion operations
// (tests/tasty/src/unmod.scala, the shapes of scalajs-react's and cats-effect's bodies). The
// expectation is scalac 3.8.4's under Scala.js.
import fix.unmod.*
import scala.scalajs.js

@main def main(): Unit =
  println(UmBodies.attempt("boom"))
  println(UmBodies.revConcat(List(1, 2), List(3)))
  val o = js.Dynamic.literal(a = 1, b = 2).asInstanceOf[js.Object]
  println(s"${UmBodies.hasKey(o, "a")} ${UmBodies.hasKey(o, "z")} ${UmBodies.keysOf(o)}")
  println(UmBodies.describe(js.Symbol.forKey("app")))
  val target = new js.Object
  UmBodies.define(target, "x", 7)
  println(target.asInstanceOf[js.Dynamic].x)
  println(UmBodies.decoded("$plus$plus"))
