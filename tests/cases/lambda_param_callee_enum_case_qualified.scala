// The callee an enum case written through the enum's name.
enum Node:
  case Wrapped(inner: Int)
  case Plain
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  val f = on(Node.Wrapped(_))
  println(f(1))
