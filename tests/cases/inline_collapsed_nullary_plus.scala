// `+(k.h.toInt)` over a parameterless `h: 42` in an inline body: both operations collapse into the call's node, and
// its typed records keep dotty's finite spine Select(Select(k.h, toInt), unary_+), which the inline copy carries
// whole. scalac 3.8.4's output.
class Holder:
  def h: 42 = { println("h"); 42 }
object Holder:
  inline def f(k: Holder): Int = +(k.h.toInt)
@main def main(): Unit = println(Holder.f(new Holder))
