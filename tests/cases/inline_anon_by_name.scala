// A class an inline body makes, whose members take by-name parameters and keep them in lazy vals, copied
// at each expansion: the copy's parameters stay by-name, and a thunk's node has no type of the value it
// gives (dotty's `TreeTypeMap` maps the symbol with its `ExprType`), so a read applies the thunk its
// caller passed, from the trait's own code as from the expansion's.
case class Box[T](v: T)
trait Deriv[F[_]]:
  def join[T](name: String, inner: => F[T]): F[T]
  def wrap[T](x: T): F[T]
  inline def derived[T](x: T): F[T] = join("n", wrap(x))
  def viaThunk[T](x: T): F[T] = join("t", wrap(x))
inline def derive[T](x: T): Box[T] =
  val d = new Deriv[Box]:
    def join[T](name: String, inner: => Box[T]): Box[T] =
      lazy val l = inner
      Box(l.v)
    def wrap[T](x: T): Box[T] = Box(x)
  d.derived[T](x)
inline def derive2[T](x: T): Box[T] =
  class Cached extends Deriv[Box]:
    def join[T](name: String, inner: => Box[T]): Box[T] = cached(name)(inner)
    private def cached[T](key: String)(f: => Box[T]): Box[T] = f
    def wrap[T](x: T): Box[T] = Box(x)
  new Cached().viaThunk[T](x)
@main def run(): Unit =
  println(derive(3))
  println(derive2("s"))
