// expect: several entry points: A.main, B.main; choose one with --main
// expect: a @main method takes no parameters or a single String*

object A:
  def main(args: Array[String]): Unit = println("A")

object B:
  def main(args: Array[String]): Unit = println("B")

object C:
  @main def typed(n: Int): Unit = println(n)
