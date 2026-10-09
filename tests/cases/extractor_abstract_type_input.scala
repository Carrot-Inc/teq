// A generic `unapply` over an abstract type of a self-typed trait (chimney's `Type.Map(k, v)` over
// a `Type[M]`) takes the scrutinee as it is, its type parameter inferred by the call.
trait Types:
  type Type[A]
  object Tpe:
    def unapply[A](a: Type[A]): Option[Int] = Some(1)
trait Uses:
  this: Types =>
  def f[M](m: Type[M]): Int = m match
    case Tpe(n) => n
    case _ => 0
object Impl extends Types with Uses:
  type Type[A] = List[A]
@main def run(): Unit = println(Impl.f(List(1)))
