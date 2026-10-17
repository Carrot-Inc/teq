// jars: zio zio-test zio-streams zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat scala-java-time macrotask-executor cats-kernel-sjs cats-core-sjs
// targets: js interp
//> using dep dev.zio::zio-test:2.1.26
//> using dep org.typelevel::cats-core:2.13.0
// zio-test's smart assertion over cats' `===`, an extension through the conversion `catsSyntaxEq`
// (`Apply(Select(Apply(catsSyntaxEq, x), ===), 2)` in scalac's trees), and a lambda's `matches`: its
// macro reads each node it tests as a `Select` through `Select.unapply` (0.1.8 refused one).
import zio.test.*
import cats.syntax.eq.*
object Main:
  def main(args: Array[String]): Unit =
    val segments = "ab/c_d-1".split("/").toList
    println(assertTrue(segments.size === 2, segments.forall(_.matches("[A-Za-z0-9_-]+"))).isSuccess)
    println(assertTrue(segments.size === 3).isSuccess)
    println(assertTrue(segments.forall(_.matches("[a-z]+"))).isSuccess)
