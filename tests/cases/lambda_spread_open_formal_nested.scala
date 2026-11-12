// A lambda under an open formal whose parameter is spread into a repeated parameter, made inside
// another lambda under an open formal.
def join(xs: Int*): String = xs.mkString("/")
def on[A, B](f: A => B): A => B = f

@main def run(): Unit =
  val f = on(xs => on(ys => join(ys*)))
  println("made")
