def combine(f: (Int, Int) => Int): Int = f(3, 4)

def combine3(f: (Int, Int, Int) => Int): Int = f(1, 2, 3)

@main def main(): Unit =
  println(combine((_, _) => 42))
  println(combine(_ + _))
  println(combine(_ * _ + 1))
  println(combine3((_, b, _) => b))
  println(combine3((_, _, _) => 0))
  println(combine3(_ + _ * _))
  println(List(1, 2, 3).foldLeft(0)(_ + _))
  println(List(1, 2, 3).zip(List(4, 5, 6)).map((_, _) => 1))
  println(List(1, 2, 3).zip(List(4, 5, 6)).map(_ + _))
  println(List(1 -> "a", 2 -> "b").map((_, v) => v))
  val nested: (Int, Int) => (Int, Int) => Int = (_, a) => (_, b) => a + b
  println(nested(1, 2)(3, 4))
  val typed = (_: Int, _: String) => "typed"
  println(typed(1, "x"))
  val mixed: (Int, Int) => List[Int] = (_, n) => List(1, 2).map(_ + n)
  println(mixed(0, 10))
