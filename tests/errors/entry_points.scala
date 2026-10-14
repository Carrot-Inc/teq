// expect: several entry points: A.main, B.main; choose one with --main
// expect: @main method cannot have type parameters

object A:
  def main(args: Array[String]): Unit = println("A")

object B:
  def main(args: Array[String]): Unit = println("B")

object C:
  @main def typed[T](n: Int): Unit = println(n)
