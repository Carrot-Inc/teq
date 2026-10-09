// expect: PartialFunction cannot be extended

object Doubler extends PartialFunction[Int, Int]:
  def isDefinedAt(x: Int): Boolean = x > 0
  def apply(x: Int): Int = x * 2

@main def main(): Unit = println(Doubler(2))
