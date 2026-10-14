// A constructor variable against an alias applied to as many arguments as it has parameters is bounded by
// the alias, not fixed to it: `U[Int]` as `M[A0]` makes `U` a lower bound of `M`, and a later operand
// `RI[R, Int]` as `M[A1]` widens `M` to `[X] =>> Z[R, Throwable, X]`, which `U[X]` conforms to, as dotty's
// `compareAppliedTypeParamRef` constrains one bound of the variable (cats' `parMapN` on a tuple of a
// `UIO` and an `RIO`). Two aliases that do not nest join their arguments: `M` is
// `[X] =>> Z[R, String | Throwable, X]`.
import scala.language.implicitConversions

class Z[-R, +E, +A](val a: A)
type U[A] = Z[Any, Nothing, A]
type U2[A] = Z[Any, String, A]
type RI[R, A] = Z[R, Throwable, A]

final class T2Ops[M[_], A0, A1](t: (M[A0], M[A1])):
  def parMapN[B](f: (A0, A1) => B): B = f(t(0).asInstanceOf[Z[Any, Any, A0]].a, t(1).asInstanceOf[Z[Any, Any, A1]].a)
final class T3Ops[M[_], A0, A1, A2](t: (M[A0], M[A1], M[A2])):
  def parMapN[B](f: (A0, A1, A2) => B): B =
    f(t(0).asInstanceOf[Z[Any, Any, A0]].a, t(1).asInstanceOf[Z[Any, Any, A1]].a, t(2).asInstanceOf[Z[Any, Any, A2]].a)
implicit def syntax2[M[_], A0, A1](t: (M[A0], M[A1])): T2Ops[M, A0, A1] = new T2Ops(t)
implicit def syntax3[M[_], A0, A1, A2](t: (M[A0], M[A1], M[A2])): T3Ops[M, A0, A1, A2] = new T3Ops(t)

def g[M[_], A0, A1](a: M[A0], b: M[A1]): M[A0] = a

def two[R](u: U[Int], r: RI[R, Int]): Int = (u, r).parMapN(_ + _)
def three[R](u: U[Int], r: RI[R, List[Int]], z: Z[R, Throwable, Int]): Int = (u, r, z).parMapN((a, b, c) => a + b.sum + c)
def direct[R](u: U[Int], r: RI[R, Int]): RI[R, Int] = g(u, r)
def joined[R](u: U2[Int], r: RI[R, Int]): Z[R, String | Throwable, Int] = g(u, r)

@main def main(): Unit =
  println(two[String](new Z(1), new Z(2)))
  println(three[String](new Z(1), new Z(List(2, 3)), new Z(4)))
  println(direct[String](new Z(5), new Z(6)).a)
  println(joined[String](new Z(7), new Z(8)).a)
