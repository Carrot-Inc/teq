// A reflected method type keeps its clause's kind: `f(x: Int)` and `f(using x: Int)` differ, as
// under scalac, and each rebuilds to itself through `Refinement(parent, name, info)`.
package app
import mlib.Infos

@main def run(): Unit =
  println(Infos.compare[AnyRef { def f(x: Int): Int }, AnyRef { def f(using x: Int): Int }])
  println(Infos.compare[AnyRef { def f(using x: Int): Int }, AnyRef { def f(using x: Int): Int }])
  println(Infos.compare[AnyRef { def f(x: Int)(using y: String): Int }, AnyRef { def f(x: Int)(y: String): Int }])
