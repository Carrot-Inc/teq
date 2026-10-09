package fix.inlforms

import scala.compiletime.constValue
import scala.deriving.Mirror

// The shapes of a derivation's inline body: a lambda converted to a trait with a single
// abstract method, and an operator over a `constValue` of a match type, which scalac selects
// on the match type's bound (`Tuple.Size[X] <: Int`).
trait IfEqv[A]:
  def eqv(x: A, y: A): Boolean

object IfEqv:
  inline def by[A]: IfEqv[A] = (x, y) => x == y
  inline def derived[A](using m: Mirror.Of[A]): IfEqv[A] =
    inline if constValue[Tuple.Size[m.MirroredElemTypes]] > 2 then (_, _) => true
    else (x, y) => x == y
  inline def arity[T <: Tuple]: Int = constValue[Tuple.Size[T]] + 1

// An object's export of inline members: scalac's forwarders are inline, the transparent one's
// transparent.
object IfImpl:
  inline def twice(x: Int): Int = x + x
  transparent inline def pick(inline b: Boolean): Any = inline if b then 1 else "s"

object IfFacade:
  export IfImpl.{twice, pick}
