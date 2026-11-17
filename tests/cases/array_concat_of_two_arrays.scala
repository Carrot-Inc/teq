// `a ++ b` of two arrays, of any element type: the class tag scalac passes it is no part of what
// the lean std's concatenation does, and a JavaScript build over that std does not evaluate a
// tag the compiler makes there. Nothing else of the program names a `ClassTag`.
class Crate(val n: Int) { override def toString = "C" + n }

def text[T](xs: Array[T]): String = {
  var s = ""
  var k = 0
  while (k < xs.length) {
    s = s + (if (k > 0) "," else "") + xs(k)
    k += 1
  }
  s
}

// In a helper that takes no tag.
def joined(a: Array[Crate], b: Array[Crate]): Array[Crate] = a ++ b

object Main {
  def main(args: Array[String]): Unit = {
    println("start")
    val a = Array(new Crate(1), new Crate(5))
    val b = Array(new Crate(2))
    println(text(a ++ b))
    println(text(Array(1, 5) ++ Array(3, 2)))
    println(text(Array("x") ++ Array("y")))
    val nested = Array(Array(1), Array(2, 3)) ++ Array(Array(4))
    println(nested.length.toString + " " + text(nested(2)))
    println(text(joined(a, b)))
    val typed: Array[Crate] = b ++ a
    println(text(typed))
    var grown = a
    grown = grown ++ b
    grown = grown ++ grown
    println(text(grown))
  }
}
