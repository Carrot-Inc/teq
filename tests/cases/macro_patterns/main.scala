import Macros.*

@main def run(): Unit =
  val k = 7
  println(describe(1 + 2))
  println(describe(k))
  println(describe("s"))
  println(describe(List(1, 2).head))
  println(describe(List(2.5)))
  println(describe(2.5))
  println(typeName[Int])
  println(typeName[List[Option[Int]]])
  println(typeName[(Int, Double)])
  println(lift(List(1, 2, 3)))
  println(lift(List(k)))
  println(pairOf(1, "a"))
  println(pairOf(k, "b"))
  println(optionOf(Some(3)))
  println(optionOf(None))
  println(optionOf(Option(k)))
