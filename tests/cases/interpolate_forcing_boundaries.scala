// Where a variable left open (`Inferencing.interpolateTypeVars`) is instantiated before teq's
// comparison or reduction needs it: inside a type-level operation or a match type, or as a
// higher-kinded variable (dotty's comparer constrains it there: `compareS`, a match's reduction,
// `canInstantiate`); where an adaptation fails (`adaptToSubType`'s `recover`); at a selection the
// variable's member decides (`couldInstantiateTypeVar`); at the end of a lambda's body, an anonymous
// method whose result is fully defined (`inferredResultType`), so its parameter's path reaches the
// closure's type; before an inline method expands (`Inliner`).
import scala.compiletime.ops.int.{S, +}
class Foo[T <: Int] {
  def incS = new Foo[S[T]]
  def incP = new Foo[T + 1]
}
object Foo {
  def apply[T <: Int & Singleton](value: T): Foo[T] = new Foo[T]
}
class C[T]
type IsTrue[T] = T match
  case C[true] => true
  case C[false] => false
class W[T] extends C[IsTrue[T]]
def f[T <: C[?]](t: T) = W[T]()
trait M[A] { def bla: Int = 1 }
trait Fk[F[_]] { def foo[G[x] >: F[x]]: G[Unit] }
trait Sv { type N; def n: N }
def newS[X](x: X): Sv { type N = X } = new Sv { type N = X; def n = x }
def needTuple[T <: Tuple](x: T): Int = x.productArity
def rest(x: Any): Int = x match
  case p: (h *: t) => needTuple[t](p.tail)
  case _ => -1
@main def run() =
  val a: Foo[2] = Foo(1).incS
  val b: Foo[2] = Foo(1).incP
  val c: C[true] = f(C[true]())
  val fk = new Fk[M] { def foo[G[x] >: M[x]]: G[Unit] = new M[Unit] {} }
  val cl = (s: Sv) => newS(s.n)
  val cl2: (s: Sv) => Sv { type N = s.N } = cl
  val sv = newS("n")
  println((a.toString.nonEmpty, b.toString.nonEmpty, c.toString.nonEmpty, fk.foo.bla, cl2(sv).n, rest((1, 2, 3))))
