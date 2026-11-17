// An inline method that concatenates two arrays with a tag it summons, called for an element type
// whose tag the program gives and for an array of arrays, whose tag the compiler makes from the
// element's. scalac prints the program's tag once and runs both calls.
// teq is to run both calls: the compiler's tag of the array of arrays is no value the
// concatenation needs.
import scala.compiletime.summonInline
import scala.reflect.ClassTag

class Token(val n: Int)
object Token { implicit def tag: ClassTag[Token] = { println("token tag"); ClassTag(classOf[Token]) } }

inline def joinedSize[T](a: Array[T], b: Array[T]): Int = {
  val both = a.++(b)(using summonInline[ClassTag[T]])
  val more = both.length * 100 + a.length * 10 + b.length
  println("joined " + more)
  more + 1
}

object Main {
  def main(args: Array[String]): Unit = {
    val tokens = new Array[Token](1)
    val rows = new Array[Array[Int]](2)
    println(joinedSize(tokens, tokens))
    println(joinedSize(rows, rows))
  }
}
