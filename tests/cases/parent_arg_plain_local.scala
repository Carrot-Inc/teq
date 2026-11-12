// The parent's argument of the anonymous class made in the applied argument is a parameter of the method
// around: a local of the place, no enclosing instance.
sealed trait Unsafe
object Unsafe extends Unsafe
trait Eff[A]:
  def run(): A
object Eff:
  inline def succeed[A](inline a: Unsafe ?=> A): Eff[A] = Sync(() => a(using Unsafe))
  final class Sync[A](eval: () => A) extends Eff[A]:
    def run(): A = eval()
trait Adaptor:
  def name: String
  def extra(ids: List[String]): String = "none"
trait Adaptors:
  def mk(n: Int): Eff[Adaptor]

abstract class Base(val t: String) extends Adaptor:
  def name: String = "base:" + t
class Suite:
  private val tag = "suite"
  def mkAdaptors(response: String): Adaptors =
    new Adaptors:
      override def mk(n: Int): Eff[Adaptor] = Eff.succeed(new Base(response):
        override def extra(ids: List[String]) = "lit")
  def test: String = { val x = mkAdaptors("resp").mk(7).run(); x.name + "/" + x.extra(Nil) }

@main def run(): Unit = println(Suite().test)
