// An inline method applies its inline context-function parameter to an object inside a closure of its body
// (the shape of a library's `succeed`): the argument makes an anonymous subclass of an inner class, inside a
// method of an anonymous class.
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
final class Imports(db: String, a: Adaptors):
  def run(n: Int): String = { val x = a.mk(n).run(); db + ":" + x.name + "/" + x.extra(Nil) }

class Suite:
  private val tag = "suite"
  private class Stub extends Adaptor:
    override def name: String = "stub:" + tag
  private def mkImports(response: String): Imports =
    Imports("db", new Adaptors:
      override def mk(n: Int): Eff[Adaptor] = Eff.succeed(new Stub:
        override def extra(ids: List[String]) = response))
  def test: String = mkImports("resp").run(7)

@main def run(): Unit = println(Suite().test)
