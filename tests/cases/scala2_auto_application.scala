// A bare `f` of a Scala 2 class's `def f()`, the standard library's `Iterator.next()`,
// `Console.println()` and `Builder.result()`, or of a program's method overriding one, calls it
// with scalac's E100 warning, which the output here shows; a program's own `def f()` named so is
// an error (tests/errors/parens.scala).
class Counting extends Iterator[Int]:
  var n = 0
  def hasNext = true
  def next() =
    n += 1
    n

@main def run(): Unit =
  val it = Iterator(1, 2)
  println(it.next)
  println(it.next)
  Console.println
  val b = List.newBuilder[Int]
  b += 3
  println(b.result)
  val c = new Counting
  println(c.next + c.next)
  val i: Iterator[Int] = c
  println(i.next)
