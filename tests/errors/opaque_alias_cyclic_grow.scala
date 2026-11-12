// teq: --wtostring-interpolated
object types:
  opaque type Grow[T] = Grow[List[T]]
  def grow: Grow[Int] = ???

import types.*

@main def run(): Unit =
  val g = grow
  println(s"1 $g")
// expect: 3:15: error: illegal cyclic type reference
