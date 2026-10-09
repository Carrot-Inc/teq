// Overloads whose parameters are unions with an opaque type erase to what the members erase
// to: `Show[Int] | D[Show[Int]]` is a `Show` after erasure, `Read[Int] | D[Read[Int]]` a `Read`.
object Wrap:
  opaque type D[A] = A
  object D:
    def apply[A](a: A): D[A] = a
import Wrap.D
class Show[A](val s: String)
class Read[A](val s: String)
object Ops:
  def name(x: Show[Int] | D[Show[Int]]): String = "show " + (x match { case s: Show[?] => s.s; case _ => "?" })
  def name(x: Read[Int] | D[Read[Int]]): String = "read " + (x match { case r: Read[?] => r.s; case _ => "?" })
object Main:
  def main(args: Array[String]): Unit =
    println(Ops.name(new Show[Int]("a")) + ", " + Ops.name(new Read[Int]("b")) + ", " + Ops.name(D(new Show[Int]("c"))))
