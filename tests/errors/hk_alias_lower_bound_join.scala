// A constructor variable an alias bounds from below keeps the bound when a later operand's class instantiates
// it: `U2[Int]` then `RI[R, Int]` as `M[A0]`, `M[A1]` make `M` `[X] =>> Z[R, String | Throwable, X]`, the
// join dotty's constraint gives the two lower bounds, not the second operand's constructor alone.
class Z[-R, +E, +A](val a: A)
type U2[A] = Z[Any, String, A]
type RI[R, A] = Z[R, Throwable, A]
def g[M[_], A0, A1](a: M[A0], b: M[A1]): M[A0] = a
def t2[R](u: U2[Int], r: RI[R, Int]) = g(u, r)
val z: Z[Any, Throwable, Int] = t2[Any](new Z(1), new Z(2))
// expect: 9:33: error: type mismatch: found Z[Any, String | Throwable, Int], required Z[Any, Throwable, Int]
