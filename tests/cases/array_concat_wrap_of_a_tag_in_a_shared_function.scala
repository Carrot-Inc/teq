// An inline method that concatenates two arrays of arrays with the wrap of its inline tag, called
// at four places whose expansions the output writes as one function: the first passes the tag
// the compiler makes, two others a tag the program makes, which is evaluated at each of the two.
import scala.reflect.ClassTag

class Crate(val n: Int)

def loudTag: ClassTag[Crate] = { println("tag made"); ClassTag(classOf[Crate]) }

inline def joinedRows[T](a: Array[Array[T]], b: Array[Array[T]])(using inline tag: ClassTag[T]): Int = {
  val both = a.++(b)(using tag.wrap)
  val more = both.length * 100 + a.length * 10 + b.length
  println("joined " + more)
  more + 1
}

object Main {
  def main(args: Array[String]): Unit = {
    val words = Array(Array("x"))
    val crates = Array(Array(new Crate(1)))
    println(joinedRows(words, words))
    println(joinedRows(crates, crates)(using loudTag))
    println(joinedRows(crates, crates))
    println(joinedRows(crates, crates)(using loudTag))
  }
}
