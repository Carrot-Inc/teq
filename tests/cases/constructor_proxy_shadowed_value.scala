// Outside an application a constructor proxy's name is the binding it shadows (dotty's `checkNotShadowed`
// returning the shadowed reference), and a plain `val` lets the proxy's application through; a selection
// through an instance names the class's proxy whatever an outer scope defines.
object Outer:
  val C = 9
  def D(x: String): String = x
  class Inner:
    class C(val x: Int)
    class D(val x: Int)
    val c = C
    val made = C(1)
@main def run(): Unit =
  val i = new Outer.Inner
  println(i.c)
  println(i.made.x)
  println(i.D(3).x)
