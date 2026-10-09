// A parameterless method whose result takes the arguments (`withService(f)` for a
// `def withService[T]: Partial[T]` with an `apply` on `Partial`): the type arguments the result
// leaves open are settled by that `apply`, from the expected type and the arguments.
class Tag[A](val name: String)
object Tag:
  given Tag[Any] = new Tag("Any")
  given Tag[Svc] = new Tag("Svc")
  given Tag[Store] = new Tag("Store")

class ZIO[-R, +E, +A](val run: R => A):
  def flatMap[R1 <: R, E1 >: E, B](f: A => ZIO[R1, E1, B]): ZIO[R1, E1, B] = new ZIO(r => f(run(r)).run(r))
  def map[B](f: A => B): ZIO[R, E, B] = new ZIO(r => f(run(r)))

object ZIO:
  def succeed[A](a: A): ZIO[Any, Nothing, A] = new ZIO(_ => a)
  def service[S](using tag: Tag[S]): ZIO[S, Nothing, S] = new ZIO(s => s)
  final class ServiceWith[Service]:
    def apply[R <: Service, E, A](f: Service => ZIO[R, E, A])(using tag: Tag[Service]): ZIO[R, E, A] =
      service[Service].flatMap(f)
    def named[A](f: Service => A)(using tag: Tag[Service]): String = tag.name + "." + f.toString.length.min(1)
  def serviceWithZIO[S]: ServiceWith[S] = new ServiceWith[S]

class Svc(val name: String)
class Store(val items: List[Int])

final class Builder[T](val prefix: String):
  def apply(value: T)(suffix: String): String = s"$prefix${value}$suffix"

object Main:
  def withService[T]: ZIO.ServiceWith[T] = ZIO.serviceWithZIO
  def builder[T]: Builder[T] = new Builder[T]("<")

  val get: ZIO[Svc, Throwable, String] = withService(svc => ZIO.succeed(svc.name + "!"))
  val total: ZIO[Store, Nothing, Int] = withService(_.items match { case xs => ZIO.succeed(xs.sum) })
  val explicit: ZIO[Svc, Nothing, Int] = ZIO.serviceWithZIO[Svc](s => ZIO.succeed(s.name.length))

  def main(args: Array[String]): Unit =
    println(get.run(new Svc("s")))
    println(total.run(new Store(List(1, 2, 3))))
    println(explicit.run(new Svc("abcd")))
    val shown: String = builder(42)(">")
    println(shown)
    val b: Builder[Boolean] = builder
    println(b(true)("."))
