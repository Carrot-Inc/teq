// A constant argument of a dependent method stands for its parameter as its literal type, a
// stable singleton dotty's `safeSubstParam` (TypeAssigner.scala) substitutes as it is: `h(1)` of
// `def h(a: Int): a.type` is a `1`, and `IsX[a.type]` over `1` and `2` are two match types that
// do not reduce (`1` neither matches `x.type` nor is disjoint from it), not `IsX[Int]` twice.
val x = 42
type IsX[T] = T match
  case x.type => true
  case _ => false
def f(a: Int, b: Int)(using IsX[a.type] =:= IsX[b.type]): Unit = ()
def g(a: Int)(using IsX[a.type] =:= IsX[a.type]): Unit = ()
def h(a: Int): a.type = a
def k(a: Int, b: Int): (IsX[a.type], IsX[b.type]) = ???
def distinct = f(1, 2)
def same = g(1)
def paths = f(x, x)
def local = { val y = 3; f(y, y) }
def literal: 1 = h(1)
def folded: 2 = h(1 + 1)
def pair = { val p = k(1, 2); val q: (IsX[1], IsX[2]) = p }
def widened = { val p = k(1, 2); val q: (IsX[Int], IsX[Int]) = p }
// expect: 13:23: error:
// expect: 20:64: error: type mismatch: found (IsX[1], IsX[2]), required (IsX[Int], IsX[Int])
// absent: 14:
// absent: 15:
// absent: 16:
// absent: 17:
// absent: 18:
// absent: 19:
