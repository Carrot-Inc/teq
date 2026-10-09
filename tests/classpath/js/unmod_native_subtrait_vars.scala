// jars: fixtures
//> using platform js
// A library body setting the vars a native trait of the jar inherits from `js.PropertyDescriptor`
// (`p.writable = true`, scalajs-react's `ViaReactComponent._defineProperties`), at the top level, private in an object and pickled by Scala 3.3 as a setter call; the expectation is
// scalac 3.8.4's under Scala.js.
import fix.unmod.*
import scala.scalajs.js

@main def main(): Unit =
  val target = new js.Object
  UmDefine.defineAll(target, js.Array(UmDefine.method("a", 1), UmDefine.method("b", "two")))
  val d = target.asInstanceOf[js.Dynamic]
  println(s"${d.a} ${d.b}")
  d.a = 5
  println(d.a)
  UmNested.define(target, "c", true)
  println(d.c)
  UmSetter33.define(target, "e", 2.5)
  println(d.e)
