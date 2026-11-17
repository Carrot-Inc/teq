// An inline method that appends to an array and maps it with a tag it summons, called for element
// types whose tag the compiler makes and for one whose tag the program gives (and prints). scalac
// evaluates the program's tag twice at each of its two calls: once for `appended`, once for `map`.
// teq is to evaluate the program's tag at the places that pass it, whatever the other places pass.
import scala.compiletime.summonInline
import scala.reflect.ClassTag

class Token(val n: Int)
class TokenTag extends ClassTag[Token] { def runtimeClass: Class[?] = classOf[Token] }
object Token { implicit def tag: ClassTag[Token] = { println("token tag"); new TokenTag } }

inline def grownSize[T](a: Array[T], x: T): Int = {
  val more = a.appended(x)(using summonInline[ClassTag[T]])
  val same = a.map(v => v)(using summonInline[ClassTag[T]])
  val n = more.length * 100 + same.length * 10 + a.length
  println("grown " + n)
  n + 1
}

object Main {
  def main(args: Array[String]): Unit = {
    val words = Array("x")
    val tokens = new Array[Token](1)
    val numbers = Array(1, 2)
    val word = "y"
    val token = new Token(2)
    val number = 3
    println(grownSize(words, word))
    println(grownSize(tokens, token))
    println(grownSize(numbers, number))
    println(grownSize(tokens, token))
    println(grownSize(words, word))
  }
}
