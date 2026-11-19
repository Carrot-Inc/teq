// expect: 7:11: error: value b is not a member of Int => C
// expect: 8:31: error: Found:    "x"⏎Required: Int
// expect: 2 errors found

case class C(a: Int)(val b: Int)
@main def run(): Unit =
  println(C(0)(9).copy(a = 1).b)
  println(C(0)(9).copy(a = 1)("x").b)
