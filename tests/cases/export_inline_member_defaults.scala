// An inline member's defaults through its export forwarder: the forwarder's getters are the
// member's on the forwarder's path (dotty's `Namer.foreachDefaultGetterOf`), an inline
// parameter's getter inline, a strict one's evaluated once, a default reading the instance on
// that path.
object Lib:
  inline def f(inline x: Int = 7): Int = x + x
object Facade:
  export Lib.f

class Strict(k: Int):
  inline def g(x: Int = k): Int = x * 2
  inline def h(inline x: Int = { println("default"); 3 }): Int = x + x
  inline def byName(x: => Int): Int = x + x
val strict = new Strict(5)
export strict.{g, h, byName}

object Chain:
  export Facade.f

@main def run(): Unit =
  println(Facade.f())
  println(Facade.f(2))
  println(g())
  println(g(4))
  println(h())
  var n = 0
  println(byName { n += 1; n })
  println(n)
  println(Chain.f())
