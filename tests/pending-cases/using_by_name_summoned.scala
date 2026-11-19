// A using parameter of a by-name type is a given of its type in the method's body: `summon[Int]` there evaluates
// it, anonymous or named, and so does a given's condition of a by-name type (`given (=> Int) => Long`). scalac
// 3.8.4 prints `3`, `3` and `3`.
given Int = 3
def f(using => Int): Long = summon[Int].toLong
def g(using n: => Int): Long = summon[Int].toLong
object N:
  given (=> Int) => Long = summon[Int].toLong
@main def run(): Unit =
  println(f)
  println(g)
  import N.given
  println(summon[Long])
