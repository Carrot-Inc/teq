// expect: 16:11: error: values of types Map[String, Int] and Map[Int, Int] cannot be compared with == or !=
// expect: 18:11: error: values of types Map[String, Int] and Map[Int, Int] cannot be compared with == or !=
// expect: 22:11: error: values of types Map[String, Int] and Map[Int, Int] cannot be compared with == or !=
// expect: 23:11: error: values of types Map[String, Int] and Map[Int, Int] cannot be compared with == or !=
// expect: 24:11: error: values of types Left[String, Nothing] and Left[Int, Nothing] cannot be compared with == or !=
// expect: 25:11: error: values of types (String, Int) and (Int, Int) cannot be compared with == or !=
// expect: 26:11: error: values of types Map[String, Int] and Map[Int, Int] cannot be compared with == or !=
// expect: 27:11: error: values of types Map[String, Int] and Map[Int, Int] cannot be compared with == or !=
// expect: 29:11: error: values of types Either[String, Int] and Left[Int, Nothing] cannot be compared with == or !=
// expect: 30:11: error: values of types Left[Int, Nothing] and Either[String, Int] cannot be compared with == or !=
// expect: 10 errors found
// With an operand's type arguments fixed (a val, an ascription, explicit type arguments, a
// covariant class's application or a tuple), `==` checks CanEqual on them as scalac does.
@main def Main(): Unit =
  val a = Map("a" -> 1)
  println(a == Map(1 -> 1))
  println(List("a") == List(1))
  println(Map("a" -> 1) == (Map(1 -> 1): Map[Int, Int]))
  println(Set("a") == Set(1))
  println(Some("a") == Some(1))
  val b: Map[Int, Int] = Map(1 -> 1)
  println(a == b)
  println(Map("a" -> 1) == b)
  println(Left("a") == Left(1))
  println(("a", 1) == (1, 1))
  println(Map("a" -> 1) == Map[Int, Int](1 -> 1))
  println(Map[String, Int]("a" -> 1) == Map(1 -> 1))
  val e: Either[String, Int] = Left("a")
  println(e == Left(1))
  println(Left(1) == e)
