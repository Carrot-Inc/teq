// jars: scala-library zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm zio-streams-jvm zio-test-jvm cats-kernel cats-core
// std: scala-library
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
