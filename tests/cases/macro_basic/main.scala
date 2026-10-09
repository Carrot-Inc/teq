import Macros.*

@main def run(): Unit =
  println(answer)
  println(twice(21))
  val y = 5
  println(twice(y + 1))
  println(isConst(3))
  println(isConst(y))
  println(showType[List[Int]])
  println(showType[String])
  println(sum(1, 2, 3))
  println(summonShow[Int])
  println(summonShow[String])
