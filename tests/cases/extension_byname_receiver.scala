// An extension whose receiver is by-name (`extension (n: => Int)`) is passed the receiver's thunk:
// each use evaluates it, selected on its receiver or, right-associative, on its other operand.
object S:
  extension (n: => Int) def twice: Int = n + n
  extension (n: => Int) def +:(s: String): String = s + n + n
var count = 0
def next(): Int = { count += 1; count }

@main def run(): Unit =
  import S.*
  println(next().twice)
  println(count)
  println("x".+:(next()))
  println(count)
  println(next() +: "y")
  println(count)
