// An inline method that concatenates two arrays, called for three element types whose expansions
// the output writes as one function: each place passes a class tag the compiler makes, which the
// concatenation does not read, and the program names no `ClassTag` value anywhere.
import scala.compiletime.summonInline

class Crate(val n: Int)

inline def joinedSize[T](a: Array[T], b: Array[T]): Int = {
  val both = a.++(b)(using summonInline[scala.reflect.ClassTag[T]])
  val more = both.length * 100 + a.length * 10 + b.length
  println("joined " + more)
  more + 1
}

object Main {
  def main(args: Array[String]): Unit = {
    val words = Array("x")
    val crates = Array(new Crate(1))
    val numbers = Array(1, 2)
    println(joinedSize(words, words))
    println(joinedSize(crates, crates))
    println(joinedSize(numbers, numbers))
  }
}
