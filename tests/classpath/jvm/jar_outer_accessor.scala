// jars: scala-library outer-lib
// std: scala-library
// A trait nested in a jar's trait (doobie's `ReadPlatform.Auto`, tests/support/outer_lib.scala):
// scalac names its outer accessor `outerlib$Platform$Auto$$$outer`. An inline method of the trait
// expanded here calls it on the jar's own instance, and the jar's default method calls it on an
// object of this program mixing the trait in, which has to define it under that name.
import outerlib.*

object Mine extends Platform:
  def base = 7
  object A extends Auto

object Main:
  def main(args: Array[String]): Unit =
    println(Lib.Impl.value)
    println(Lib.Impl.viaOuter)
    println(Mine.A.value)
    println(Mine.A.viaOuter)
