// The callee an enum case named through an import of the enum's cases.
enum Node:
  case Wrapped(inner: Int)
  case Plain
import Node.*
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  val f = on(Wrapped(_))
  println(f(1))
