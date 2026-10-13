// A member of an anonymous class whose type is a class of its own body is typed by that class's
// class bound applied to the occurrence's arguments, as `TypeOps.avoid` approximates it
// (`range(Nothing, classBound(info))`, the bound `HKTypeLambda.fromParams(cls.typeParams, raw)`):
// `Hidden[String]` is `P[String]`, so `x.item.get` is a `String`.
import scala.reflect.Selectable.reflectiveSelectable

trait P[A] { def get: A }
trait Q[-A, +B] { def run(a: A): B }

def boxed[T](t: T) = new Selectable {
  class H[A](a: A) extends P[A] { def get: A = a }
  val item = new H[T](t)
}

@main def main(): Unit =
  val x = new Selectable { class Hidden[A](a: A) extends P[A] { def get: A = a }; val item = new Hidden[String]("ok") }
  val s: String = x.item.get
  println(s)
  val y = new Selectable { class Pair[A, B](a: A, b: B) extends P[(A, B)] { def get: (A, B) = (a, b) }; val both = new Pair[Int, String](1, "one") }
  val p: (Int, String) = y.both.get
  println(p)
  val z = new Selectable { class F[A, B](f: A => B) extends Q[A, B] { def run(a: A): B = f(a) }; val len = new F[String, Int](_.length) }
  val n: Int = z.len.run("four")
  println(n)
  val i: Int = boxed(3).item.get
  println(i + 1)
