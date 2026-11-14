// std: scala-library
// teq: --std=scala-library
// scala-library's own value classes below `Proxy`, which overrides `equals` and `hashCode`, have the rule's two all
// the same: the library is compiled as Scala 2's, where scalac 3.8.4 synthesizes both for a value class whatever its
// traits define. So two `RichInt`s of one number are equal and a `RichInt` is not its number.
import scala.runtime.RichInt

object Main {
  def main(args: Array[String]): Unit = {
    val a: Any = new RichInt(5)
    val b: Any = new RichInt(5)
    val c: Any = new RichInt(6)
    println(a == b)
    println(a == c)
    println(a.hashCode)
    println(a == 5)
    println(Set[Any](new RichInt(5), new RichInt(5), new RichInt(6)).size)
    println(5.max(7))
    println((1 to 3).sum)
  }
}
