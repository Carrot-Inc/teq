// A parameterless polymorphic method whose result is a function applied at once
// (`toRoutes(builder)` of tapir's `def toRoutes[R2]: Builder[RIO[R & R2, *]] => Routes[..]`):
// its type argument stays open for the argument, also where only the arguments of a type
// alias (`RIO[R & R2, a]`) mention it.
final case class Eff[-R, +E, +A](a: A)
type RIO[-R, +A] = Eff[R, Throwable, A]
final case class Builder[F[_]](name: String)
final case class Routes[F[_]](name: String)
trait LogBase
type WebTask[A] = RIO[LogBase, A]

class ToRoutes[R]:
  def toRoutes[R2]: Builder[[a] =>> RIO[R & R2, a]] => Routes[[a] =>> RIO[R & R2, a]] = b => Routes(b.name + "!")
  def named[R2]: Builder[[a] =>> RIO[R & R2, a]] => String = b => b.name

def mk(b: Builder[WebTask]): Routes[WebTask] = ToRoutes[LogBase]().toRoutes(b)

@main def main(): Unit =
  val b: Builder[WebTask] = Builder("ws")
  println(mk(b))
  println(ToRoutes[LogBase]().named(b))
