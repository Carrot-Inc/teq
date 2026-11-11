// A given with parameters is a method, so its parameters' defaults are getters as a method's are
// (`Desugar.addDefaultGetters`, Desugar.scala:382, of the given's DefDef): scalac 3.8.4 prints the .expected, each
// omitted using argument its getter's value, the inline given's inline one expanded. teq enters getters only for a
// `def` (getters.rs `enter_getters`, its `DefKind::Fun`), and reports "no given instance of type Int was found for
// parameter x" at `c.g` and "missing default getter" for `gg` and `h`. Reviewer #10's note of the workers exports
// helper.
class C(k: Int):
  given g(using x: Int = k + 1): String = s"g$x"
  inline given h(using inline x: Int = 7): Long = x.toLong
object O:
  given gg(using x: Int = 9): String = s"gg$x"
@main def run(): Unit =
  val c = C(3)
  println(c.g)
  import O.gg
  println(summon[String])
  import c.h
  println(summon[Long])
