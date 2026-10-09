// jars: zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time
// targets: js
//> using dep dev.zio::zio:2.1.26
// zio's `catchSome` over a source-defined generic handler with a `ClassTag[E]`: `case _: E` tests
// the tag's class (the dotty audit's ranked item 1), so a non-matching failure is left alone.
import zio.*
import scala.reflect.ClassTag

object Main:
  def handle[E <: Throwable: ClassTag](io: Task[Int]): Task[Int] =
    io.catchSome { case _: E => ZIO.succeed(7) }
  def main(args: Array[String]): Unit =
    val a = handle[IllegalArgumentException](ZIO.fail(new IllegalStateException("wrong"))).either
    val b = handle[IllegalArgumentException](ZIO.fail(new IllegalArgumentException("right"))).either
    val program = a.zipWith(b)((x, y) => (x.isLeft, y == Right(7)))
    println(Unsafe.unsafe { implicit u => Runtime.default.unsafe.run(program).getOrThrowFiberFailure() })
