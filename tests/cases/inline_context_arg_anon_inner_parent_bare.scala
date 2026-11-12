// As inline_context_arg_anon_inner_parent, the anonymous subclass capturing nothing of its own: on JavaScript
// the instance it is handed lands in the field its parent reads its outer instance from.
sealed trait Unsafe
object Unsafe extends Unsafe
trait Eff[+A]:
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
  private class Stub extends Adaptor:
    override def name: String = "stub:" + tag
  def adaptors: Adaptors = new Adaptors:
    override def mk(n: Int): Eff[Adaptor] = Eff.succeed(new Stub:
      override def extra(ids: List[String]) = "lit")
  def test: String = { val x = adaptors.mk(7).run(); x.name + "/" + x.extra(Nil) }

@main def run(): Unit = println(Suite().test)
