package eob

import scala.deriving.Mirror
import scala.compiletime.constValueTuple
import eoa.Kind

@main def run(): Unit =
  println(Kind.Small.ordinal)
  println(Kind.Large.ordinal)
  println(Kind.Sized(3).ordinal)
  println(Kind.Huge.ordinal)
  val m = summon[Mirror.SumOf[Kind]]
  val labels = constValueTuple[m.MirroredElemLabels]
  println(labels.productElement(m.ordinal(Kind.Small)))
  println(labels.productElement(m.ordinal(Kind.Sized(1))))
