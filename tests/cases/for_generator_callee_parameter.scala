class Source:
  def map[A](f: A => Int): Int = f("abc".asInstanceOf[A])
  def foreach[A](f: A => Unit): Unit = f("hello".asInstanceOf[A])

def consume(s: String): Int = s.length
def shout(s: String): Unit = println(s.toUpperCase)

@main def run(): Unit =
  println(for x <- new Source yield consume(x))
  for x <- new Source do shout(x)
