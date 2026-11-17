// Controls beside a class named through a path: an opaque type named from outside its scope is written with a
// prefix too and is no class there. The equality of one over Int and an array of one over a class stay what
// they are.
object A:
  class Box(val v: Int)
  opaque type T = Int
  def mk(i: Int): T = i
  opaque type B = Box
  def mkB(n: Int): B = new Box(n)
  def sum(a: Array[B]): Int = a.map(_.v).sum

@main def run(): Unit =
  println(s"${A.mk(1) == A.mk(1)} ${A.mk(1) != A.mk(2)}")
  val a = new Array[A.B](2)
  a(0) = A.mkB(3)
  a(1) = A.mkB(4)
  println(s"${a.length} ${A.sum(a)}")
