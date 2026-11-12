// An enum case with a default parameter, applied to the first alone.
enum Node:
  case Wide(a: Int, b: Int = 2)
  case Plain
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  val f = on(Node.Wide(_))
  println(f(1))
