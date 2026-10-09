// jars: scala-library zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// std: scala-library
// targets: jvm
// The JVM side of tests/classpath/js/zio_runtime_sync.scala, against zio's JVM jars
// (the _3 artifacts of the same versions, izumi-reflect 3.0.9, scala-collection-compat 2.14.0; names
// tests/support/jars.sh does not know yet), linked as bytecode.
//> using dep dev.zio::zio:2.1.26
// zio 2.1.26's runtime from its jar, effects that complete without an asynchronous boundary run by
// `Runtime.default.unsafe.run` under `Unsafe.unsafe`: construction, `map`/`flatMap`, errors and
// their handlers, `Ref`, a `Promise` completed before it is awaited, a `Queue` offered then taken,
// `foreach`, and `Exit` as the result.
package ziosync

import zio.*

final case class Missing(key: String)

object Main:
  def lookup(m: Map[String, Int], k: String): IO[Missing, Int] =
    ZIO.fromOption(m.get(k)).orElseFail(Missing(k))

  def main(args: Array[String]): Unit =
    val table = Map("a" -> 1, "b" -> 2)
    val program: UIO[List[String]] =
      for
        ref <- Ref.make(0)
        _ <- ZIO.foreachDiscard(1 to 5)(i => ref.update(_ + i))
        total <- ref.get
        p <- Promise.make[Nothing, String]
        _ <- p.succeed("done")
        pv <- p.await
        q <- Queue.unbounded[Int]
        _ <- q.offerAll(List(3, 1, 2))
        taken <- q.takeAll
        found <- lookup(table, "b").either
        missing <- lookup(table, "z").either
        caught <- ZIO.attempt("x".toInt).catchAll(e => ZIO.succeed(-1))
        mapped <- ZIO.fail("boom").mapError(_.length).flip
        fromE <- ZIO.fromEither(Right(7): Either[String, Int]).orElseSucceed(0)
        squares <- ZIO.foreach(List(1, 2, 3))(n => ZIO.succeed(n * n))
        whenV <- ZIO.when(total > 10)(ZIO.succeed("big"))
        _ <- ZIO.unit
      yield List(total, pv, taken, found, missing, caught, mapped, fromE, squares, whenV).map(_.toString)
    val out = Unsafe.unsafe { implicit u => Runtime.default.unsafe.run(program).getOrThrowFiberFailure() }
    out.foreach(println)
    val exit = Unsafe.unsafe { implicit u => Runtime.default.unsafe.run(ZIO.fail("bad").unit) }
    println(exit.isFailure)
    println(exit.causeOption.map(_.failures))
