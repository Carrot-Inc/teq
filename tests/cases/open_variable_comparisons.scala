// The variables the trees around a selection constrain stay open through it: `S[?N]` against a
// positive constant constrains `?N` with its predecessor (dotty's `compareS`), so `prev(2).x`
// against a `1` makes `?N` a `1`; a match alias's application that reduces where the method is
// declared holds no variable (`M[T]` of a `T <: Int` is `Int` or `T`), so `mk(1)._1.x` keeps
// `?T` open for the `1`; an applied variable bounded below is instantiated for a selection on it
// (`couldInstantiateTypeVar` of an `AppliedType`), and against a longer application of an
// abstract constructor it takes the leftmost arguments fixed (`canInstantiate`).
import scala.compiletime.ops.int.S
class Box[A](val x: A)
def prev[N <: Int](n: S[N]): Box[N] = new Box((n - 1).asInstanceOf[N])
def pred[N <: Int](n: S[N]): N = (n - 1).asInstanceOf[N]
def succ[T <: Int](x: T): (Box[T], S[T]) = (new Box(x), (x + 1).asInstanceOf[S[T]])
type IntCase[T] = T match
  case Int => Int
  case _ => String
type Echo[T] = T match
  case Int => T
  case _ => String
def int[T <: Int](x: T): (Box[T], IntCase[T]) = (new Box(x), 0.asInstanceOf[IntCase[T]])
def echo[T <: Int](x: T): (Box[T], Echo[T]) = (new Box(x), x.asInstanceOf[Echo[T]])
trait M[A] { def bla: Int = 7 }
trait Fk[F[_]] { def foo[G[x] >: F[x]]: G[Unit] }
trait Kinds:
  type T[A, B, C <: B, D <: C]
  def indexK[F[X, Y <: X]]: Option[F[Any, Any]] = None
  def res: Option[T[Any, Any, Any, Any]] = indexK
@main def run() =
  val a: 1 = prev(2).x
  val b: 1 = pred(2)
  val c: 1 = succ(1)._1.x
  val d: 1 = int(1)._1.x
  val e: 1 = echo(1)._1.x
  val fk = new Fk[M] { def foo[G[x] >: M[x]]: G[Unit] = new M[Unit] {} }
  val k = new Kinds { type T[A, B, C <: B, D <: C] = Int }
  println((a, b, c, d, e, fk.foo.bla, k.res))
