// The callee an enum case under the name an import gives it.
enum Node:
  case Wrapped(inner: Int)
  case Plain
import Node.{Wrapped as W}
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  val f = on(W(_))
  println(f(1))
