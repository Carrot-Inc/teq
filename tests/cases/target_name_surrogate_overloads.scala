// Overloads told apart by @targetName literals that differ only in a lone surrogate keep their code units in every
// target's names (the adversary's probe TN4.scala on the M2e refold: the JavaScript names collapsed to $u65533).
package target_name_surrogate_overloads
import scala.annotation.targetName
// Overloads that erase alike, told apart by @targetName literals that differ only in a lone surrogate.
class O {
  @targetName("\uD83D") def f(x: Int): Int = x + 1
  @targetName("\uD83E") def f(x: Int, y: Int = 0): Int = x + 2
  @targetName("�") def g(x: String): Int = 3
  @targetName("😀") def g(x: String, y: Int = 0): Int = 4
}
object Use4 {
  def main(args: Array[String]): Unit = {
    val o = new O
    println(List(o.f(10), o.f(10, 1), o.g("a"), o.g("a", 1)).mkString(","))
  }
}
