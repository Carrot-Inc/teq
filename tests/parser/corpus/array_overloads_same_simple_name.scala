// Overloads over arrays of two classes of one simple name in different objects.
object Left:
  class X:
    override def toString = "left"
object Right:
  class X:
    override def toString = "right"
object O:
  def f(x: Array[Left.X]): String = "l " + x.head
  def f(x: Array[Right.X]): String = "r " + x.head

@main def run(): Unit =
  println(O.f(Array(new Left.X)))
  println(O.f(Array(new Right.X)))
