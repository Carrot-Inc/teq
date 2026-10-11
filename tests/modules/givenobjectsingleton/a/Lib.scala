package lib

// A given object a trait declares, read through the object that extends it: the summoned value's
// inferred type is its singleton `B.u.type` (dotty keeps a module's `TermRef`), which the
// products pickle as the path and erase to the given object's class, as scalac's.
trait Token:
  def name: String

trait A:
  given u: Token with
    def name = "u"

object B extends A

val chosen = summon[B.u.type]
