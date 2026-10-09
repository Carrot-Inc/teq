@main def run(): Unit =
  val i: Int = 1
  val l: Long = 1L
  println(i == l)
  println(l == i)
  println(1 == 1.0)
  println(1L == 1.0)
  val a: Any = 1
  val b: Any = 1L
  println(a == b)
  println(Set[Any](1, 1L).size)
  println(Map[Any, Int](1 -> 1, 1L -> 2).size)
  println(1.5.toInt)
  println((1.5 + 1.5) == 3)
  println(3.0 == 3)
  val d: Double = 3
  println(d)
  println(List(d))
  println(d.toString)
