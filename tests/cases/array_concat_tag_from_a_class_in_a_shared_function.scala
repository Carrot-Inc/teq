// An inline method that concatenates two arrays with a tag it makes from a class the place
// passes, called at four places whose expansions the output writes as one function: the first
// passes a class literal, two others a class whose evaluation prints, at each of the two.
import scala.reflect.ClassTag

class Crate(val n: Int)

inline def joinedBy[T](a: Array[T], b: Array[T], inline c: Class[?]): Int = {
  val both = a.++(b)(using ClassTag[T](c))
  val more = both.length * 100 + a.length * 10 + b.length
  println("joined " + more)
  more + 1
}

object Main {
  def main(args: Array[String]): Unit = {
    val words = Array("x")
    val crates = Array(new Crate(1))
    println(joinedBy(words, words, classOf[String]))
    println(joinedBy(crates, crates, { println("class made"); classOf[Crate] }))
    println(joinedBy(crates, crates, classOf[Crate]))
    println(joinedBy(crates, crates, { println("class made"); classOf[Crate] }))
  }
}
