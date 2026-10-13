// teq: --werror
// expect: 14:22: warning: match may not be exhaustive
// expect: 17:21: warning: pattern's type
// expect: 18:26: warning: pattern's type a.Nest.I is more specialized than the right hand side expression's type a.T
// A child through the prefix its owners rebuild keeps its fields (dotty's `Space.signature`, the
// extractor's result a member of the scrutinee's type): `a.Nest.I(0)` covers one `I`, not every
// one, as scalac warns of `f`; and a pattern definition reads them too, refutable as scalac warns.
class O:
  sealed trait T
  object Nest:
    case class I(n: Int) extends T
  case class W(t: T) extends T
val a = new O
def f(x: a.T): Int = x match
  case a.Nest.I(0) => 0
def g(x: a.Nest.I, w: a.W): Int =
  val a.Nest.I(1) = x
  val a.W(a.Nest.I(q)) = w
  q
