// By-name varargs, `xs: => T*`: the sequence is built from the arguments each time the parameter
// is read, a splice included; an inline method takes one too.
var count = 0
def next(): Int = { count += 1; count }

def firstPositive(xs: => Int*): Option[Int] = xs.find(_ > 0)
def lazyAll(xs: => Int*): Int = { println("before " + count); xs.sum }
def twice(xs: => Int*): Int = xs.sum + xs.sum
inline def inlined(x: => Int*): Int = x.sum

@main def run(): Unit =
  println(firstPositive(next(), next(), -1))
  println(count)
  println(lazyAll(next(), next()))
  println(count)
  println(twice(next()))
  println(count)
  val seq = List(5, 6)
  println(lazyAll(seq*))
  println(inlined(1, 2, next()))
