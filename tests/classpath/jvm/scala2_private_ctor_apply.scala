// jars: scala-library scala2-lib
// std: scala-library
// A Scala 2.13 case class with a private constructor keeps its companion's public `apply`,
// which scalac 3 calls; the companion's own factories reach the constructor.
import scala2lib.*

object Main:
  def main(args: Array[String]): Unit =
    println(Version(1, 2))
    println(Version.of(3))
    println(Guarded.make(4).n)
