// expect: 22:24: warning: match may not be exhaustive
// expect: 25:24: warning: match may not be exhaustive
// absent: 19:22: warning
// absent: 30:10: warning
// absent: 31:28: warning
// absent: 36:8: warning
// expect: 2 warnings found, errors under --werror
// teq: --werror
// The spaces of patterns through prefixes, as scalac's keep them: an outer-tested pattern
// covers the scrutinee only where the scrutinee names the same enclosing instance (the
// children of `T` inside `O` are `O.this.A` and `O.this.B`, those of `o.T` are `o.A` and `o.B`,
// `TypeOps.childPrefix`); against an `O#T` or an `O#I` it leaves the scrutinee whole, so a
// match of bare constructors there is not exhaustive and a later wildcard is reachable.
class O:
  sealed trait T
  case class A(n: Int) extends T
  case class B(n: Int) extends T
  case class I(n: Int)
  def f(x: T): Int = x match
    case A(n) => n
    case B(n) => n
  def g(x: O#T): Int = x match
    case A(n) => n
    case B(n) => n
  def k(x: O#T): Int = x match
    case _: A => 1
    case _: B => 2
  def t1(x: O#I): String = x match
    case I(n) => "a"
    case _ => "b"
def h(o: O, x: o.T): Int = x match
  case o.A(n) => n
  case o.B(n) => n
def t5(b: O, x: O#I): String = x match
  case _: b.I => "a"
  case _ => "b"
