// teq: --werror
// expect: 23:30: warning: match may not be exhaustive.
// expect: It would fail on pattern case: Some(_)
// absent: 18:30: warning
// absent: 19:22: warning
// A sealed parent's child in an object of the parent's class is covered through the scrutinee's
// prefix and that object (`TypeOps.childPrefix`'s walk over the owners, `a.Nest.I`): scalac is
// silent on `f` and `g`. A child of a class nested in the parent's class matched through one
// instance of it (`n.I`) leaves the others uncovered, as scalac warns of `h`.
class O:
  sealed trait T
  object Nest:
    case class I(n: Int) extends T
class P:
  sealed trait T
  class Nest:
    case class I(n: Int) extends T
val a = new O
def f(x: Option[a.T]): Int = x match { case Some(a.Nest.I(n)) => n; case None => 0 }
def g(x: a.T): Int = x match { case a.Nest.I(k) => k }
val b = new P
val n = new b.Nest
def h(x: Option[b.T]): Int = x match { case Some(n.I(k)) => k; case None => 0 }
