// The callee a case class nested in an object, written through the object and imported.
object Outer:
  case class In(a: Int)
import Outer.In
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  val f = on(Outer.In(_))
  val g = on(In(_))
  println(s"${f(1)} ${g(2)}")
