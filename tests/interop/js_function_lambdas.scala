// A lambda typed as a `js.FunctionN` is a JavaScript function (`SJSPlatform#isSam`: a trait whose superclass is
// the native `js.Function`), whatever the arity and wherever the function goes: a val, a facade's parameter, an
// object literal's field.
import scala.scalajs.js

@main def main(): Unit =
  val f: js.Function1[Int, Int] = (x: Int) => x + 1
  println(js.typeOf(f))
  val g: js.Function2[Int, Int, Int] = (a: Int, b: Int) => a * b
  println(js.typeOf(g))
  val h: js.Function0[String] = () => "zero"
  println(js.typeOf(h))
  println(js.Array(1, 2, 3).map((x: Int) => x * 10).join(","))
  val d = js.Dynamic.literal(cb = ((x: Int) => x + 2): js.Function1[Int, Int])
  println(js.typeOf(d.cb))
