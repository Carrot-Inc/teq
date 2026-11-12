// The callee a case of an enum nested in an object.
object Outer:
  enum Node:
    case Wrapped(inner: Int)
    case Plain
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  val f = on(Outer.Node.Wrapped(_))
  println(f(1))
