// A lambda under an open formal whose parameter is spread into a case class's repeated parameter,
// applied to an element: scalac 3.8.4 types the function as `Int* => Many`.
case class Many(items: Int*)
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => Many(xs*))
  println(f(1))
