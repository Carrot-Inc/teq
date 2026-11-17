// An inline method that concatenates two arrays, called at four places whose expansions the
// output writes as one function: a class tag the program writes itself, at two of them, is
// evaluated at each of the two, as scalac evaluates it.
import scala.reflect.ClassTag

class Crate(val n: Int)

def loudTag: ClassTag[Crate] = { println("tag made"); ClassTag(classOf[Crate]) }

inline def joinedSize[T](a: Array[T], b: Array[T])(using inline tag: ClassTag[T]): Int = {
  val both = a.++(b)(using tag)
  val more = both.length * 100 + a.length * 10 + b.length
  println("joined " + more)
  more + 1
}

object Main {
  def main(args: Array[String]): Unit = {
    val words = Array("x")
    val crates = Array(new Crate(1))
    println(joinedSize(words, words))
    println(joinedSize(crates, crates)(using loudTag))
    println(joinedSize(crates, crates))
    println(joinedSize(crates, crates)(using loudTag))
  }
}
