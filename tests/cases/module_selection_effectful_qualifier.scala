// The selection of a member object keeps its qualifier's effects (SelectStatic's `Block(List(qualifier),
// ref(module))`), where the selection is a lambda's retained body as where it is not. scalac prints the lines of the
// .expected file.
object Outer:
  object Nested:
    def apply(n: Int): Int = n

object Main:
  def receiver(): Outer.type =
    println("receiver")
    Outer

  def main(args: Array[String]): Unit =
    val f = n => receiver().Nested(n)
    println(f(1))
    val g = receiver().Nested
    println(g(2))
