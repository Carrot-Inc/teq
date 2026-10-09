// A refinement's method info read twice is the same type, and so is the info of the
// refinement rebuilt from it, as under scalac (`r.info =:= r.info`).
package app
import mlib.Infos

@main def run(): Unit =
  println(Infos.same[AnyRef { def f(x: Int)(y: String): Int }])
  println(Infos.same[AnyRef { def g(x: => Int, xs: Long*): Int }])
  println(Infos.same[AnyRef { def h: Int }])
