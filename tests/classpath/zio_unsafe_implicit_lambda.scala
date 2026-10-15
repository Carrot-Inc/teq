// jars: scala-library zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat
// zio's `Unsafe.unsafe[A](f: Unsafe => A)` given an `implicit` lambda without a parameter type: the
// parameter is the expected function's `Unsafe` (`Typer.typedFunctionValue` from `pt`) and a given in
// the body, a call that needs one included; the val `Unsafe.unsafe` beside the method is no candidate
// for an application. scalac 3.8.4 accepts it.
import zio.*

object Main:
  def needs(using Unsafe): Int = 1
  val direct: Int = Unsafe.unsafe { implicit unsafe => val u = summon[Unsafe]; needs + u.hashCode * 0 }
  def run(effect: UIO[Int]): Exit[Nothing, Int] =
    Unsafe.unsafe { implicit unsafe =>
      Runtime.default.unsafe.run(effect)
    }
