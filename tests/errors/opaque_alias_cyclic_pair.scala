// teq: --wtostring-interpolated
object types:
  opaque type A = B
  opaque type B = A
  def a: A = ???

import types.*

@main def run(): Unit =
  val x = a
  println(s"1 $x")
// expect: 3:15: error: Cyclic reference involving type A
