// A refinement method's by-name parameter is a `ByNameType` in its info and a repeated one
// scalac's `<repeated>[A]`, kept through `Refinement(parent, name, info)`.
package app
import mlib.Params

@main def run(): Unit =
  println(Params.of[AnyRef { def f(x: => Int): Int }])
  println(Params.of[AnyRef { def g(xs: Int*): Int }])
  println(Params.of[AnyRef { def h(a: String, b: => Long)(c: Boolean*): Int }])
