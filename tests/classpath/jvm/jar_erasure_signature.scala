// jars: scala-library erasure-lib
// std: scala-library
// Jar types that only the signatures of `Shapes` name, so that the typer completes none of them:
// two opaque types of tests/support/erasure_lib.scala, over `Long` and over a `List`, and a value
// class over `Int`. Each erases as its jar says, which the descriptors reflection finds show as
// scalac's class files have them (the opaque types were `Object` and the value class `Cm`).
import erasure.{Cm, Descriptors, Id, Tags}

object Shapes:
  def passId(i: Id): Id = i
  def passTags(t: Tags[Int]): Tags[Int] = t
  def passCm(c: Cm): Cm = c

object Main:
  def main(args: Array[String]): Unit =
    for m <- List("passId", "passTags", "passCm") do println(Descriptors.of("Shapes$", m))
