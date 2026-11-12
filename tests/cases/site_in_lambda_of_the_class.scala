// An inline method applies its inline context-function parameter to an object: the argument makes an anonymous
// subclass of an inner class in a closure of the class around, where the outer instance is `this`.
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
  def mk: Int => Eff[Adaptor] = n => Eff.succeed(new Stub:
    override def extra(ids: List[String]) = "lit")
  def test: String = { val x = mk(7).run(); x.name + "/" + x.extra(Nil) }

@main def run(): Unit = println(Suite().test)
