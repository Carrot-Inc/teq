// A client whose first use of each enum of the products is its companion (`values`, a case), which completes the
// companion before the enum asks its children: the same labels, values and ordinals as scalac 3.8.4 prints, as
// enumordinalfirst's client, which reaches each enum first, prints them.
package ecfb

import scala.deriving.Mirror
import scala.compiletime.constValueTuple

object U:
  inline def labels[A](using m: Mirror.SumOf[A]): Tuple = constValueTuple[m.MirroredElemLabels]

@main def run(): Unit =
  println(ecfa.Color.values.toList)
  println(ecfa.Mixed.C)
  println(U.labels[ecfa.Color])
  println(U.labels[ecfa.Mixed])
  println(ecfa.Color.Light.ordinal)
  println(ecfa.Mixed.C.ordinal)
