package uaa

// A union side erases to an array of the classes `erasedLub` reads the base classes of
// (TypeErasure.scala:815-819, 439-445): a match type's bound, and an applied parameter's
// bound over its argument. `f` and `g` are to take `Exception[]` in this module alone too.
import java.util.concurrent.{CancellationException, TimeoutException}

object P:
  type M[X] <: Array[TimeoutException] = X match
    case String => Array[TimeoutException]
  def f[X](x: M[X] | Array[CancellationException]): Int = 1
  def g[F[X] <: Array[X]](x: F[TimeoutException] | Array[CancellationException]): Int = 2
