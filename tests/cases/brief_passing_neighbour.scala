// A by-name `succeed` of the program with a using clause after it: the anonymous subclass of an inner class is
// made in the by-name argument, inside a method of an anonymous class; nothing is applied.
trait Adaptor:
  def name: String
  def extra(ids: List[String]): String = "none"
trait Adaptors:
  def mk(n: Int): () => Adaptor
final class Imports(db: String, a: Adaptors):
  def run(n: Int): String = { val x = a.mk(n)(); x.name + "/" + x.extra(Nil) }
final class Trace
given Trace = new Trace
def succeed[A](a: => A)(using Trace): () => A = () => a
def serviceWith[S](using Trace): [R] => (String => R) => R = [R] => (f: String => R) => f("db")

abstract class Base:
  def tests: List[() => String]

class Suite extends Base:
  private val tag = "suite"
  private class Stub extends Adaptor:
    override def name: String = "stub:" + tag
  private def mkImports(response: String): Imports =
    serviceWith[Int][Imports](db =>
      Imports(db, new Adaptors:
        override def mk(n: Int): () => Adaptor = succeed(new Stub:
          override def extra(ids: List[String]) = response))
    )
  def tests = List(() => mkImports("resp").run(7))

@main def run(): Unit = println(Suite().tests.map(_()).mkString)
