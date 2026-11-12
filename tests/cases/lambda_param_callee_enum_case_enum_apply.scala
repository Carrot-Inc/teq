// The callee an enum case whose enum has a companion written with an apply of its own.
enum Node:
  case Wrapped(inner: Int)
  case Plain
object Node:
  def apply(s: String): Node = Wrapped(s.length)
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  val f = on(Node.Wrapped(_))
  println(f(1))
