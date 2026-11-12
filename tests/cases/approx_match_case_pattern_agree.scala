// The match-type case carrier where every case the approximation answers stays a match case:
// a dependent function type's result holding a match type whose case pattern names the
// parameter, at variance +1 (`Cov[..]`, the case's pattern a bounds argument, `derivedAppliedType` at v > 0) and at
// the top, its body naming the parameter in an invariant argument, and a class parameter's bound over another's
// wildcard compared at no invariant end; the match alias `MT[c.T]` in an invariant and a contravariant argument
// (dotty's `range(Nothing, Any)` for the alias's head). scalac 3.8.4 and
// teq accept each.
class Cov[+X]
class Inv[X]
class Contra[-X]
class A:
  type T
type MT[X] = Int match { case X => String }
def covPattern(f: (c: A) => Cov[Int match { case c.T => String }]): A => Cov[Any] = f
def topPattern(f: (c: A) => (Int match { case c.T => String })): A => Any = f
def invBody(f: (c: A) => Inv[Int match { case Int => c.T }]): A => Inv[?] = f
class C[P, B <: Inv[Int match { case P => String }]]
def paramBound(c: C[?, ?]): C[?, ? <: Any] = c
def aliasInv(f: (c: A) => Inv[MT[c.T]]): A => Inv[?] = f
def aliasContra(f: (c: A) => Contra[MT[c.T]]): A => Contra[Nothing] = f
@main def approxMatchCasePatternAgree(): Unit =
  val a = new A
  println(covPattern(_ => new Cov)(a) != null)
  println(aliasInv(_ => new Inv)(a) != null)
