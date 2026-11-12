//> using platform jvm
// (JavaScript left out.)
// Local lazy vals in the temporaries of a superclass call's named arguments written
// out of order, whose initialisers read the enclosing instance and a constructor
// parameter, from an inner class and from a local class: their accessors and
// initialisers are the declaring class's, which the constructor calls before the
// superclass call with the enclosing instance its outer parameter.
class Base(val a: Int, val b: Int)

class Outer:
  val k = 5
  def f = 6
  class Inner(q: Int) extends Base(b = { lazy val x = q + k; x }, a = { lazy val y = f; y })
  def make(): Int =
    class Local(r: Int) extends Base(b = { lazy val x = r + k; x }, a = { lazy val y = f + r; y })
    val l = new Local(10)
    l.a + l.b

@main def run(): Unit =
  val o = new Outer
  val i = new o.Inner(3)
  println(i.a + " " + i.b)
  println(o.make())
