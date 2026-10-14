// jars: zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time zio-streams zio-managed zio-interop-cats zio-interop-tracer cats-kernel-sjs cats-core-sjs cats-effect-kernel cats-effect-std cats-effect cats-mtl fs2-core
//> using dep dev.zio::zio:2.1.26
//> using dep dev.zio::zio-interop-cats:23.1.0.13
// cats' `parMapN` on a tuple of ZIO effects whose environments and errors differ, `UIO[A]` beside
// `RIO[R, B]` with `R` abstract: the syntax's `M[_]` (`catsSyntaxTuple2Parallel[M[_], A0, A1]`) is bounded
// below by the first operand's alias `UIO`, not fixed to it, and the next operand's constructor widens
// it to `[X] =>> RIO[R, X]`, as dotty's `compareAppliedTypeParamRef` constrains one bound of the
// variable; zio-interop-cats' `Parallel` for ZIO then runs the effects. Either's `Parallel` is the control.
package ziopar

import cats.syntax.all.*
import zio.*
import zio.interop.catz.*

object Main extends ZIOAppDefault:
  def two[R, A, B](u: UIO[A], r: RIO[R, B]): RIO[R, (A, B)] = (u, r).parMapN((a, b) => (a, b))

  def three[R, A, B, C](u: UIO[A], r: RIO[R, List[B]], z: ZIO[R, Throwable, C]): RIO[R, (A, List[B], C)] =
    (u, r, z).parMapN((a, b, c) => (a, b, c))

  def mixed[R, A, B](u: UIO[A], r: RIO[R, B]): RIO[R, String] = (u, r, u).parMapN((a, b, c) => s"$a/$b/$c")

  def run =
    val e: Either[String, Int] = (Right(1): Either[String, Int], Right(2): Either[String, Int]).parMapN(_ + _)
    for
      a <- two[Any, Int, String](ZIO.succeed(1), ZIO.attempt("b"))
      b <- three[Any, Int, Int, Char](ZIO.succeed(2), ZIO.attempt(List(3, 4)), ZIO.attempt('c'))
      c <- mixed[Any, Int, Boolean](ZIO.succeed(5), ZIO.attempt(true))
      d <- (ZIO.succeed(6), ZIO.attempt(7)).parMapN(_ + _)
      _ <- Console.printLine(s"$e $a $b $c $d")
    yield ()
