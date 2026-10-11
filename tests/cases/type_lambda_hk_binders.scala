// A higher-kinded binder of a type lambda or a polymorphic function type or literal (`[F[X]] =>> F[A]`)
// takes its own clause, recursively, as dotty's `typeParamClause` parses it: its kind is kept as its
// bound, a type lambda over the clause (`lambdaAbstract`), a class's or a method's parameter too. A
// kind bounds nothing when its variable is solved (`hasUpperBound`, `isTopOfSomeKind`).
type Apply = [F[G[H[_]]]] =>> F[[K[_]] =>> K[Int]]
class Wrap[G[H[_]]](val x: G[List])
class Hold[G[_]](val g: G[Int])
type ApplyTo[A] = [F[X]] =>> F[A]
type Twice[A] = [F[_], G[Y] <: Iterable[Y]] =>> (F[A], G[A])
type Bounded = [F[X <: AnyVal]] =>> F[Int]
trait Functor[F[_]]:
  def map[A, B](fa: F[A])(f: A => B): F[B]
given Functor[List] with
  def map[A, B](fa: List[A])(f: A => B): List[B] = fa.map(f)
def lift[A]: [F[_]] => F[A] => Functor[F] ?=> F[String] =
  [F[_]] => (fa: F[A]) => (f: Functor[F]) ?=> f.map(fa)(_.toString)
type Compose[F[_], G[_]] = [X] =>> F[G[X]]
trait Kinds:
  type T[A, B, C <: B, D <: C]
  def indexK[F[X, Y <: X]]: Option[F[Any, Any]] = None
  def res: Option[T[Any, Any, Any, Any]] = indexK
@main def run(): Unit =
  val x: ApplyTo[Int][List] = List(1)
  println(x.head)
  val p: Twice[Int][Option, List] = (Some(1), List(2))
  println(p)
  val b: Bounded[Option] = Some(3)
  println(b)
  val c: Compose[List, Option][Int] = List(Some(4))
  println(c)
  val poly: [F[_]] => F[Int] => F[Int] = [F[_]] => (fa: F[Int]) => fa
  println(poly(List(5)))
  val bare = [F[_]] => (fa: F[Int]) => fa
  println(bare(Option(6)))
  val bounded = [A <: Int] => (a: A) => a + 1
  println(bounded(7))
  println(lift[Int](List(8, 9)))
  val w: Apply[Wrap] = new Wrap[[K[_]] =>> K[Int]](List(9))
  println(w.x.head)
  val deep = [F[G[_]]] => (x: F[List]) => x
  println(deep[Hold](new Hold[List](List(10))).g)
  println((new Kinds {}).res)
