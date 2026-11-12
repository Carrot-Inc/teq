// teq: --wtostring-interpolated
object types:
  opaque type A[T] = B[List[T]]
  opaque type B[T] = A[T]
  def a: A[Int] = ???

import types.*

@main def run(): Unit =
  val x = a
  println(s"1 $x")
// expect: 3:15: error: Cyclic reference involving type A
