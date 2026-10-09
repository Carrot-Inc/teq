package pmb

import pma.*

// Matches whose selector's type reaches a sealed class through a bound, an abstract member's
// bound and a singleton.
object Use:
  def f[T <: S](x: T): Int = x match
    case A => 1
    case _ => 0
  def g(h: Holder)(m: h.M): Int = m match
    case B => 2
    case _ => 0
  def k(s: S): Int =
    val y: s.type = s
    y match
      case A => 3
      case _ => 0
