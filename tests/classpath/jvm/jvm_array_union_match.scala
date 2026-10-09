// jars: scala-library
// std: lean scala-library
// teq: --werror
// A test against an array type covers the arrays of that type alone: after `case _:
// Array[Int]` on an `Array[Int] | Array[String]` the wildcard is reachable (scalac warns of
// nothing), and the two array tests together are exhaustive.
def f(v: Array[Int] | Array[String]): Int = v match
  case _: Array[Int] => 1
  case _ => 2
def g(v: Array[Int] | Array[String]): Int = v match
  case _: Array[Int] => 1
  case _: Array[String] => 2

@main def run(): Unit =
  val xs = Array[String]("s")
  println(f(xs).toString + g(xs) + f(Array(1)) + g(Array(2)))
