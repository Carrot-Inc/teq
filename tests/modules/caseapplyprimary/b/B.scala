import cap.*
import cap.A as Renamed

object D extends Outer
class C extends Outer:
  def make: In = In(1, 2)
object P:
  export cap.A

@main def main(): Unit =
  println(G(1, 2))
  println(A(1, 2))
  println(A.apply(1, 2))
  println(Renamed(1, 2))
  println(P.A(1, 2))
  println(O.K(1, 2))
  println(D.In(1, 2))
  val c = new C
  println(c.In(1, 2))
  println(c.make)
  locally:
    import D.In
    println(In(1, 2))
  val pair: (Int, Int) => A = A(_, _)
  println(pair(1, 2))
  val f: Int => V = V.apply
  println(f(5))
  println(List(1, 2).map(V.apply))
  println(made())
  // A secondary of the primary's arity, and a name applied in the arguments of a `new` of an array.
  println(S("abc"))
  println(new Array[Int](S("abc").x.toString.length).length)
  println(new Array[Int](G(1, 2).x._1 + V(2).x).length)
