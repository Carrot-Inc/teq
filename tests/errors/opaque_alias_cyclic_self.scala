// teq: --wtostring-interpolated
object types:
  opaque type Rec = Rec
  def rec: Rec = ???

import types.*

@main def run(): Unit =
  val a = rec
  println(s"1 $a")
// expect: 3:15: error: illegal cyclic type reference
