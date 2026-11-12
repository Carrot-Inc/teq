//> using dep dev.zio::zio:2.1.26
//> using platform jvm
// jars: zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// split-watch: jvm
// An anonymous subclass of an inner class made in the argument of zio's inline `succeed`, which applies its
// context-function parameter to `Unsafe`, inside a method of an anonymous class.
import zio.*

trait Adaptor:
  def name: String
  def extra(ids: List[String]): String = "none"
trait Adaptors:
  def mk(n: Int): UIO[Adaptor]
final class Imports(db: String, a: Adaptors):
  def run(n: Int): UIO[String] = a.mk(n).map(x => x.name + "/" + x.extra(Nil))

class Suite:
  private val tag = "suite"
  private class Stub extends Adaptor:
    override def name: String = "stub:" + tag
  private def mkImports(response: String): UIO[Imports] =
    ZIO.succeed("db").map(db =>
      Imports(db, new Adaptors:
        override def mk(n: Int): UIO[Adaptor] = ZIO.succeed(new Stub:
          override def extra(ids: List[String]) = response))
    )
  def test: UIO[String] = mkImports("resp").flatMap(_.run(7))

@main def run(): Unit =
  println(Unsafe.unsafe(u => Runtime.default.unsafe.run(Suite().test)(using Trace.empty, u).getOrThrowFiberFailure()(using u)))
