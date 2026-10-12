// A value export's forwarder has the member's defaults, its getters forwarding to the member's
// on the path (dotty's `exportForwarders`, `foreachDefaultGetterOf`): a call that leaves an
// argument out applies the forwarder's getter, which applies the member's on the qualifier.
class Svc(val base: Int):
  def f(a: Int, b: Int = 10)(c: Int = a + b): Int = base + a + b + c
  def g(s: String = "x"): String = s * 2
class Host(val svc: Svc):
  export svc.{f, g}
object Main:
  def main(args: Array[String]): Unit =
    val h = Host(Svc(100))
    println(h.f(1)())
    println(h.f(1, 2)())
    println(h.f(1)(3))
    println(h.g())
