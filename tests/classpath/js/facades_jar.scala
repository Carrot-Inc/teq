// jars: fixtures
// Facades from the fixtures jar (tests/tasty/src/facades.scala), their Scala.js annotations read
// from TASTy: native classes bound to globals (`Map`, `Date` by its Scala name) and to imports
// of node's modules (a named export, the default export, the namespace), a `@JSGlobalScope`
// object, `@JSName` and `@JSBracketAccess` members, an overload under its JS name, a property
// `def`, a binding of an object's def, a JS trait and class that are not native, and the
// operations of `js.UndefOr`, which Scala.js puts in the implicit scope of `Unit`; members of a
// top-level native val imported by name (an overload, a val renamed).
package facadesjar

import fix.facades.*
import fix.facades.math.{abs, min, E as e}
import scala.scalajs.js

@main def main(): Unit =
  val m = new JsMap[String, Int]()
  m.set("a", 1).set("b", 2)
  println(m.size)
  println(m.get("a"))
  println(m.has("c"))
  println(m.remove("a"))
  println(m.size)
  println(js.typeOf(m))
  println(new Date(0).toISOString())
  println(JsMath.max(1, 2))
  println(JsMath.max(1, 5, 3))
  println(JsMath.PI > 3)
  println(JsGlobals.parseInt("ff", 16))
  println(JsGlobals.NaN.isNaN)
  val a = js.Array("x", "y").asInstanceOf[JsArr]
  a(1) = "z"
  println(a(1))
  println(a.length)
  println(a.joinWith("-"))
  println(JsPath.basename("/a/b.txt"))
  println(JsPath.sep)
  println(JsPathDefault.extname("x.scala"))
  val em = new JsEmitter()
  em.on("hi", (s: String) => println(s"got $s"))
  println(em.emit("hi", "there"))
  println(em.listenerCount("hi"))
  println(JsBindings.join("a", "b"))
  println(JsBindings.encode("a b"))
  val o = new JsOptions { val name = "n" }
  println(js.JSON.stringify(o))
  o.verbose = true
  println(o.verbose)
  val p = new JsPoint(1, 2)
  println(p.sum)
  println(js.JSON.stringify(p))
  val u: js.UndefOr[JsPoint] = p
  println(u.map(_.x).toOption)
  println((js.undefined: js.UndefOr[Int]).toOption)
  println(abs(-2))
  println(min(3, 1))
  println(min(3, 2, 4))
  println(e > 2.7)
