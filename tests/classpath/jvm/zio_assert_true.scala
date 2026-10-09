// jars: scala-library zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm zio-streams-jvm zio-test-jvm
// std: scala-library
//> using dep dev.zio::zio-test:2.1.26
// zio-test's `assertTrue` over a by-name argument: its macro reads `from.getOrElse(None)` and
// rebuilds it with `Select.overloaded`, the argument the expression as scalac's trees hold it.
import zio.test.*
object Main:
  def main(args: Array[String]): Unit =
    val from: Either[String, Option[Int]] = Right(Some(1))
    val left: Either[String, Option[Int]] = Left("no")
    println(assertTrue(from.getOrElse(None).nonEmpty).isSuccess)
    println(assertTrue(from.getOrElse[Option[Int]](None).nonEmpty).isSuccess)
    println(assertTrue(left.getOrElse(None).nonEmpty).isSuccess)
