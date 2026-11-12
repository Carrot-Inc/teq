// The anonymous subclass made in the applied argument reads a member of the class around the anonymous class
// whose method applies it: it captures that instance itself.
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

class Suite:
  private val tag = "suite"
  class Stub extends Adaptor:
    override def name: String = "stub:" + tag
  def mkAdaptors: Adaptors =
    new Adaptors:
      override def mk(n: Int): Eff[Adaptor] = Eff.succeed(new Stub:
        override def extra(ids: List[String]) = tag)
  def test: String = { val x = mkAdaptors.mk(7).run(); x.name + "/" + x.extra(Nil) }

@main def run(): Unit = println(Suite().test)
