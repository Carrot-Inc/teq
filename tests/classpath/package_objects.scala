// jars: scala-library cats-kernel cats-core
// The members a package object inherits are members of the package: a wildcard import of
// cats' instance packages brings the implicits their instance traits define.
import cats.kernel.instances.int._
import cats.kernel.instances.string._
import cats.kernel.{Order, Monoid}
object Main:
  def main(args: Array[String]): Unit =
    val o: Order[Int] = catsKernelStdOrderForInt
    println(o.compare(1, 2))
    val m = implicitly[Monoid[String]]
    println(m.combine("a", "b"))
    println(cats.kernel.instances.int.catsKernelStdGroupForInt.combine(1, 2))
