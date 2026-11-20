// A trait's overloaded alternative implementing a generic trait's method at an instance (`T[Int]`) in the
// class that mixes both: the call through `T[Int]` reaches `f(Int)`, never `f(String)`.
trait T[A] { def f(a: A): Int }
trait O { def f(a: Int): Int = 2; def f(a: String): Int = 3 }
class C extends T[Int] with O
object D extends T[Int] with O

@main def run(): Unit =
  val c = C()
  println("C " + c.f(1) + " " + c.f("s"))
  println("T " + (c: T[Int]).f(1))
  println("O " + (c: O).f(1))
  println("object " + (D: T[Int]).f(1))
