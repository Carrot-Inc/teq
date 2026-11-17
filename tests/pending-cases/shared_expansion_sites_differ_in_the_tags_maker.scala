// An inline method that concatenates two arrays with a tag it summons, called for an element type
// whose tag the program gives (and prints) and for types whose tag the compiler makes. scalac
// evaluates the program's tag at each of its three calls and runs the two others.
// teq is to evaluate, at each place, that place's own tag where the program made it and nothing
// where the compiler did.
import scala.compiletime.summonInline
import scala.reflect.ClassTag

class Token(val n: Int)
class TokenTag extends ClassTag[Token] { def runtimeClass: Class[?] = classOf[Token] }
object Token { implicit def tag: ClassTag[Token] = { println("token tag"); new TokenTag } }

inline def joinedSize[T](a: Array[T], b: Array[T]): Int = {
  val both = a.++(b)(using summonInline[ClassTag[T]])
  val more = both.length * 100 + a.length * 10 + b.length
  println("joined " + more)
  more + 1
}

object Main {
  def main(args: Array[String]): Unit = {
    val words = Array("x")
    val tokens = new Array[Token](1)
    val numbers = Array(1, 2)
    println(joinedSize(tokens, tokens))
    println(joinedSize(tokens, tokens))
    println(joinedSize(numbers, numbers))
    println(joinedSize(tokens, tokens))
    println(joinedSize(words, words))
  }
}
