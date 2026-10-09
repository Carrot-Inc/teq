// An array's view, mapped and cut without copying the array.
@main def main(): Unit =
  val a = Array(3, 1, 2)
  println(a.view.map(_ * 2).toList)
  println(a.view.take(2).mkString(","))
