// jars: scala-library
// A lexical extension with the same name must not hide Predef's implicit view.
class Fence(val position: Int)
object FenceSyntax:
  extension (left: Fence)
    def until(right: Fence): Int = right.position - left.position
object ImportedSyntax:
  export FenceSyntax.until
import ImportedSyntax.*

object Main:
  def indexes(size: Int): List[Int] = (0 until size).toList
  def main(args: Array[String]): Unit =
    assert(indexes(4) == List(0, 1, 2, 3))
    assert(indexes(0).isEmpty)
    assert(new Fence(2).until(new Fence(9)) == 7)
