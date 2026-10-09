package isa

import scala.compiletime.constValue
import scala.deriving.Mirror

// The shapes of a derivation's inline body: a lambda converted to a trait with a single abstract
// method, written as scalac's closure, and an operator over a `constValue` of a match type,
// selected on the match type's bound (`Tuple.Size[X] <: Int`).
trait Eqv[A]:
  def eqv(x: A, y: A): Boolean

object Eqv:
  given Eqv[Int] = (x, y) => x == y
  inline def by[A]: Eqv[A] = (x, y) => x == y
  inline def derived[A](using m: Mirror.Of[A]): Eqv[A] =
    inline if constValue[Tuple.Size[m.MirroredElemTypes]] > 2 then (_, _) => true
    else (x, y) => x == y
  inline def arity[T <: Tuple]: Int = constValue[Tuple.Size[T]] + 1
