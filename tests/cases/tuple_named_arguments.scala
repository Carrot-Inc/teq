// The tuple members the compiler types on a tuple type keep their types when the argument is
// named as scala-library's parameter (`n`, `x`, `that`, `t2`), as scalac reduces their match
// types; an index the type does not give is read at run time.
@main def run(): Unit =
  val a: Tuple1[Int] = (1, "x").take(n = 1)
  val b: Tuple1[String] = (1, "x").drop(n = 1)
  val c: (Tuple1[Int], Tuple1[String]) = (1, "x").splitAt(n = 1)
  val d: (Int, String, Boolean) = (1, "x").:*(x = true)
  println((a, b, c, d))
  val e: String = (1, "x").apply(n = 1)
  val f: (Int, String, Int, Int) = (1, "x").++(that = (3, 4))
  val g: ((Int, Int), (String, Int)) = (1, "x").zip(t2 = (5, 6))
  println((e, f, g))
  val t = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)
  val h: Int = t.apply(n = 22)
  val k: (Int, Int) = t.take(n = 2)
  println(h.toString + " " + k + " " + t.drop(n = 21) + " " + t.splitAt(n = 22)._2)
  var i = 2
  try println((1, 2)(i))
  catch case e: IndexOutOfBoundsException => println("dynamic index: " + e.getClass.getName)
  i = 1
  println((1, 2)(i))
