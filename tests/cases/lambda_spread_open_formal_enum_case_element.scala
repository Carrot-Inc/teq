// A lambda under an open formal whose parameter is spread into an enum case's repeated parameter,
// applied to an element: scalac 3.8.4 types the function as `Int* => Node`.
enum Node:
  case Many(items: Int*)
  case Plain
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => Node.Many(xs*))
  println(f(1))
