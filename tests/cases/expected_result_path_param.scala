// The expected type constrains a call's result before its arguments are typed when only a
// using parameter's type is a path (`trace: Tracer.instance.Type`, zio's `Trace`): the
// callback's error type comes from the expected `Eff[Any, Throwable, String]`, not from the
// first call to it (`ZIO.async`).
trait Tracer:
  type Type <: AnyRef
object Tracer:
  val instance: Tracer = new Tracer { type Type = String }
  given autoTrace: Tracer.instance.Type = "here".asInstanceOf[Tracer.instance.Type]
type Trace = Tracer.instance.Type

final class Unsafe
final class Eff[-R, +E, +A](val run: () => Either[E, A])
object Eff:
  def succeed[A](a: A)(implicit trace: Trace): Eff[Any, Nothing, A] = Eff(() => Right(a))
  def fail[E](e: => E)(implicit trace: Trace): Eff[Any, E, Nothing] = Eff(() => Left(e))
  def async[R, E, A](register: Unsafe ?=> (Eff[R, E, A] => Unit) => Unit, blockingOn: => Int = 0)(implicit trace: Trace): Eff[R, E, A] =
    var result: Eff[R, E, A] = null
    register(using new Unsafe)(r => result = r)
    result

object Main:
  def load(ok: Boolean): Eff[Any, Throwable, String] =
    Eff.async { resume =>
      if ok then resume(Eff.succeed("image"))
      else resume(Eff.fail(new RuntimeException("could not load")))
    }
  def main(args: Array[String]): Unit =
    println(load(true).run())
    println(load(false).run().left.map(_.getMessage))
