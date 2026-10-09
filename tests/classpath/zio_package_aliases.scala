// jars: scala-library zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat
// `import zio.*` finds the aliases of zio's package object that share their names with top-level
// objects of the package (`Trace`, `Duration`) next to those that do not (`UIO`, `ULayer`).
import zio.*

object Main:
  def a(t: Trace): Trace = t
  def d(x: Duration): Duration = x
  def u(x: UIO[Int]): UIO[Int] = x
  def l(x: ULayer[Int]): ULayer[Int] = x
  def main(args: Array[String]): Unit = println(d(Duration.Zero))
