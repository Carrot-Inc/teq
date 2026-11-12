// An enum case with a using clause after its parameters.
enum Node:
  case Wrapped(inner: Int)(using val tag: String)
  case Plain
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  given String = "t"
  val f = on(Node.Wrapped(_))
  println(f(1))
