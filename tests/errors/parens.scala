// expect: method next in class Counter must be called with () argument
// expect: method reset in class Counter must be called with () argument
// expect: 32:11: warning: method next must be called with () argument
// expect: 33:11: warning: method result must be called with () argument
// expect: 39:3: warning: method println must be called with () argument
// expect: 40:11: warning: method result must be called with () argument
// expect: 41:11: warning: method next must be called with () argument
// expect: method tick must be called with () argument
// expect: method current in class Counter does not take parameters
// expect: method size in trait SeqOps does not take parameters
// expect: this value does not take type arguments

class Counter:
  private var n = 0
  def next(): Int =
    n += 1
    n
  def current: Int = n
  def reset(): Unit = n = 0

def tick(): Int = 1

// A Scala 2 class's `def f()`, and one overriding it, called without `()`: scalac's E100 warning.
class Once extends Iterator[Int]:
  def hasNext = true
  def next() = 1

@main def main(): Unit =
  val c = Counter()
  println(c.next)
  c.reset
  println(List(1, 2).iterator.next)
  println(StringBuilder().result)
  println(tick)
  println(c.current())
  println(List(1).size())
  val f = (x: Int) => x
  println(f[Int](1))
  Console.println
  println(List.newBuilder[Int].result)
  println(new Once().next)
