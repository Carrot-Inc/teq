// Rule N5: a member that nothing declares on a js.Dynamic receiver is a JavaScript property,
// read, written or called verbatim, and js.Dynamic.literal takes named arguments. Expectations
// follow Scala.js, where the same forms go through selectDynamic, updateDynamic and applyDynamic.
// Compiled together with tests/interop/scalajs-stub.
package dynamic

import scala.scalajs.js

def stringify(value: js.Any): String = js.Dynamic.global.JSON.stringify(value).asInstanceOf[String]

@main def main(): Unit =
  val d = js.Dynamic.literal(a = 1, b = "two")
  println(stringify(d))
  println(d.a)
  println(d.b)
  d.c = 3
  d.`data-cy` = "cy"
  println(stringify(d))
  println(d.`data-cy`)
  println(js.isUndefined(d.missing))

  // the declared members of js.Dynamic keep their meaning
  println(d.selectDynamic("a"))
  d.updateDynamic("z")(9)
  println(d.z)
  d.f = js.Dynamic.global.eval("(x, y) => x + y")
  println(d.applyDynamic("f")(1, 2))
  println(d.f(3, 4))
  val g = js.Dynamic.global.eval("(n) => n * 2")
  println(g(21))
  println(g(g(1)))

  // members of Any keep theirs
  println(d == d)
  println(d.isInstanceOf[js.Object])
  println(js.typeOf(d.asInstanceOf[js.Object]))

  // js.Dynamic.global: bare global identifiers
  println(js.Dynamic.global.Math.max(1, 5))
  js.Dynamic.global.globalThis.TEST_DYN = 5
  println(js.Dynamic.global.TEST_DYN)

  // an intersection with js.Dynamic
  val o: js.Object & js.Dynamic = d.asInstanceOf[js.Object & js.Dynamic]
  println(o.a)

  // the literal forms
  println(stringify(js.Dynamic.literal()))
  println(stringify(js.Dynamic.literal("k" -> 1, "l" -> js.Array(1, 2))))
  println(stringify(js.Dynamic.literal(nested = js.Dynamic.literal(x = 1), flag = true)))
  val withCallback = js.Dynamic.literal(cb = ((x: Int) => x + 1): js.Function1[Int, Int])
  println(withCallback.cb(1))
  println(js.Object.keys(js.Dynamic.literal(b = 1, a = 2).asInstanceOf[js.Object]).join(","))
