// jars: cats-kernel-sjs cats-core-sjs cats-effect-kernel cats-effect-std cats-effect macrotask-executor
// targets: js
// cats-effect's bodies that tyrian's sandbox reaches, typed again from its jars: `Async.cont` of
// `Kleisli` (its lambdas written as trees, `Kleisli[G, R, *]` over a local method's type parameter),
// `Resource.start` (a local case class's defaults) and `Dispatcher` (the cast scalac makes of a
// pattern-bound wildcard's member).
//> using scala 3.8.4
//> using platform scala-js
//> using dep org.typelevel::cats-effect::3.6.3
package cebodies

import cats.data.Kleisli
import cats.effect.{Async, IO, Resource}
import cats.effect.std.Dispatcher
import cats.effect.unsafe.implicits.global

type K[X] = Kleisli[IO, Int, X]

@main def main(): Unit =
  val k = Async[K].async_[Int](cb => cb(Right(41))).map(_ + 1)
  val started = Resource.make(IO(println("acquire")))(_ => IO(println("release"))).start.use(_.join.use_)
  val dispatched = Dispatcher.sequential[IO].use(d => IO.fromFuture(IO(d.unsafeToFuture(IO(println("dispatched")).as(7)))))
  (k.run(1).flatMap(n => IO(println(n))) *> started *> dispatched.flatMap(n => IO(println(n)))).unsafeRunAndForget()
