// `Array(x, xs*)` has scalac's overloads for the primitive element types, which take no
// `ClassTag` and to whose element type the arguments are converted: `Array(1, 2L)` is an
// `Array[Long]`, `Array('a', 1)` an `Array[Int]`, and `Array(x)` of an `x: T <: Int` an
// `Array[Int]`. Two references or a reference beside a primitive take the generic one.
import scala.reflect.ClassTag

def described[T](a: Array[T])(using ct: ClassTag[T]): String = ct.toString + " " + a.length
def ints[T](a: Array[T]): String = a.map(x => x.toString).mkString(",")
def tenths(a: Array[Double]): String = a.map(x => (x * 10).toInt).mkString(",")
def f[T <: Int](x: T) = Array(x)
def g[T <: Boolean](x: T) = Array(x)
def h[T <: Long](x: T, y: T) = Array(x, y)

@main def run(): Unit =
  val i = 1
  val s: Short = 2
  val b: Byte = 3
  val c = 'c'
  println(described(Array(1, 2, 3)) + " " + ints(Array(1, 2, 3)))
  println(described(Array(i, 2.5)) + " " + tenths(Array(i, 2.5)))
  println(described(Array(1, 2L)) + " " + ints(Array(1, 2L)))
  println(described(Array('a', 1)) + " " + ints(Array('a', 1)))
  println(described(Array(s, 'a')) + " " + ints(Array(s, 'a')))
  println(described(Array(i, 'a')) + " " + ints(Array(i, 'a')))
  println(described(Array(1.5f, i)) + " " + Array(1.5f, i).map(x => (x * 10).toInt).mkString(","))
  println(described(Array(true, false)) + " " + ints(Array(true, false)))
  println(described(Array((), ())) + " " + ints(Array((), ())))
  println(described(Array(1, "x")) + " " + ints(Array(1, "x")))
  println(described(Array[Long](1, 2)) + " " + ints(Array[Long](1, 2)))
  println(described(Array(b, b)) + " " + ints(Array(b, b)) + " " + described(Array(s)) + " " + described(Array(c, c)) + " " + ints(Array(c, 'd')))
  println(described(f(1)) + " " + f(1)(0) + " " + described(g(true)) + " " + g(true)(0) + " " + described(h(1L, 2L)) + " " + ints(h(1L, 2L)))
  val longs: Array[Long] = Array(1, 2)
  val doubles: Array[Double] = Array(1, 2)
  println(described(longs) + " " + ints(longs) + " " + described(doubles) + " " + tenths(doubles))
  val more = List(2, 3)
  println(ints(Array(1, more*)) + " " + ints(Array(7)) + " " + described(Array[Int]()) + " " + described(Array("x", "y")) + " " + described(Array(Some(1), None)))
  val nested = Array(Array(1, 2), Array(3))
  println(nested.length.toString + " " + nested.map(ints).mkString(";") + " " + described(Array(1, 2).map(_ + 1)))
