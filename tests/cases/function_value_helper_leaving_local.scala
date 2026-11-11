// A function that may be an object and leaves for JavaScript's own code (the predicate of an array's `filter`) is
// written through a helper of the JavaScript runtime; a local of the body named as that helper keeps its value
// and the call its meaning.
object Main:
  def evens(xs: Array[Int], p: Int => Boolean): Int =
    val $jsFn = 1
    xs.filter(p).length + $jsFn

  def main(args: Array[String]): Unit =
    println(evens(Array(1, 2, 3, 4), x => x % 2 == 0))
