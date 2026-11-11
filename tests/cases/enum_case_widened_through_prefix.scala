// An enum case nested in a class, created through a path: the case's type widens to its parents read through its
// prefix (`Applications.widenEnumCase`, `tree.tpe.parents`), so `a.E.K(1)` is an `a.E`, as is `new a.E.K(1)`.
class O:
  enum E:
    case A
    case K(n: Int)
  class C(val n: Int)
  object C:
    def mk(n: Int): C = new C(n)
val a = new O
val c: a.C = a.C.mk(1)
val n: a.E = new a.E.K(1)
val s: a.E = a.E.A
val k: a.E = a.E.K(1)
@main def run(): Unit = println((c.n, n, s, k))
