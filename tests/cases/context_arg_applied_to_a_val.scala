// The inline method applies its context-function parameter to a val, which is no path the application is
// reduced for: the argument's closure is called.
final class Unsafe
object Unsafe:
  val unsafe: Unsafe = new Unsafe
trait Eff[+A]:
  def run(): A
trait Companion:
  inline def succeed[A](inline a: Unsafe ?=> A): Eff[A] = Eff.Sync(() => a(using Unsafe.unsafe))
object Eff extends Companion:
  final case class Sync[A](f: () => A) extends Eff[A]:
    def run(): A = f()

trait Adaptor:
  def name: String
  def extra(ids: List[String]): String = "none"
trait Adaptors:
  def mk(n: Int): Eff[Adaptor]

class Suite:
  private val tag = "suite"
  class Stub extends Adaptor:
    override def name: String = "stub:" + tag
  def mkAdaptors: Adaptors =
    new Adaptors:
      override def mk(n: Int): Eff[Adaptor] = Eff.succeed(new Stub:
        override def extra(ids: List[String]) = "lit")
  def test: String = { val x = mkAdaptors.mk(7).run(); x.name + "/" + x.extra(Nil) }

@main def run(): Unit = println(Suite().test)
