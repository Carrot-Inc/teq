class Source:
  def withFilter[A](p: A => Boolean): Source = this
  def map[A](f: A => Int): Int = f("abcd".asInstanceOf[A])

def keep(s: String): Boolean = s.nonEmpty
def consume(s: String): Int = s.length

@main def run(): Unit =
  println(for x <- new Source if keep(x) yield consume(x))
