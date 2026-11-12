// teq: --wtostring-interpolated
object types:
  opaque type W[T] = T
  opaque type Fix[F[_]] = F[Fix[F]]
  def fix: Fix[W] = ???

import types.*

@main def run(): Unit =
  val x = fix
  println(s"1 $x")
// expect: 4:15: error: illegal cyclic type reference
