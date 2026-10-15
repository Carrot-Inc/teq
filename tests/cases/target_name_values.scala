import scala.annotation.targetName

// Two values of one name with distinct target names are two declarations, never a double definition
// (dotty's `Denotation.matches` in `Checking.checkNoDoubleDeclaration`), as tyrian's generated
// attributes declare them: each initialised in order and read at its type. Two methods of one
// signature with distinct target names are two declarations too.
trait Attributes:
  @targetName("size-Int") val size: Int = { println("size Int"); 3 }
  @targetName("size-String") val size: String = { println("size String"); "three" }
  @targetName("twice-Int") def twice(x: Int): Int = x * 2
  @targetName("twice-String") def twice(x: Int): String = x.toString * 2

object Html extends Attributes

@main def run(): Unit =
  val n: Int = Html.size
  val s: String = Html.size
  println(n + 1)
  println(s + "!")
  println(Html.twice(4): Int)
  println(Html.twice(4): String)
