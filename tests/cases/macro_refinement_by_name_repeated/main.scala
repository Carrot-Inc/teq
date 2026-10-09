// A by-name repeated parameter keeps both wrappers in a refinement's method info,
// `ByNameType(<repeated>[Int])` as under scalac, so it differs from a plain repeated one, and
// rebuilds to itself.
package app
import mlib.Infos

@main def run(): Unit =
  println(Infos.compare[AnyRef { def f(xs: => Int*): Int }, AnyRef { def f(xs: Int*): Int }])
  println(Infos.compare[AnyRef { def f(xs: => Int*): Int }, AnyRef { def f(xs: => Int*): Int }])
  println(Infos.compare[AnyRef { def f(a: => String, xs: Long*): Int }, AnyRef { def f(a: String, xs: => Long*): Int }])
