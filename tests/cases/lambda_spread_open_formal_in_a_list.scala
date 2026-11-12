// A lambda under an open formal whose parameter is spread into a repeated parameter, put into a list.
def join(xs: Int*): String = xs.mkString("/")
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => join(xs*))
  println(List(f).size)
