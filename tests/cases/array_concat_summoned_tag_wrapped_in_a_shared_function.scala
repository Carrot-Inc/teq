// An inline method that concatenates two arrays of arrays with the wrap of a tag it summons,
// called at four places whose expansions the output writes as one function: for one element
// class the tag is the compiler's, for another an implicit of the class's companion, which is
// evaluated at each place that asks for it.
import scala.reflect.ClassTag
import scala.compiletime.summonInline

class Crate(val n: Int)
class Token(val n: Int)
object Token { implicit def tag: ClassTag[Token] = { println("token tag"); ClassTag(classOf[Token]) } }

inline def joinedRows[T](a: Array[Array[T]], b: Array[Array[T]]): Int = {
  val both = a.++(b)(using summonInline[ClassTag[T]].wrap)
  val more = both.length * 100 + a.length * 10 + b.length
  println("joined " + more)
  more + 1
}

object Main {
  def main(args: Array[String]): Unit = {
    val words = Array(Array("x"))
    val crates = Array(Array(new Crate(1)))
    val tokens = Array(new Array[Token](1))
    println(joinedRows(words, words))
    println(joinedRows(tokens, tokens))
    println(joinedRows(crates, crates))
    println(joinedRows(tokens, tokens))
  }
}
