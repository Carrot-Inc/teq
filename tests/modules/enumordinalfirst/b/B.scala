// A client whose first use of each enum of the products is `ordinal` on a value of it, which completes the enum
// before its companion: the enum's children are its pickle's Child annotations, read when they are asked
// (`SymDenotation.children`, which completes the companion first, SymDenotations.scala:1781-1782), so each value
// case is the companion's member, and the client prints scalac 3.8.4's labels, values and ordinals. Integration lost
// `Light` and `C` (NoSuchMethodError `Color$.Light()` on the JVM, null in the interpreter, `undefined.ordinal` on
// JavaScript) when the enum's completion read its Child annotations before the companion entered its values (found
// by the review of tr/lead-enum-mirror-children; enumcompanionfirst is the other order).
package eofb

import scala.deriving.Mirror
import scala.compiletime.constValueTuple

object U:
  def o(c: eofa.Color): Int = c.ordinal
  def p(c: eofa.Mixed): Int = c.ordinal
  inline def labels[A](using m: Mirror.SumOf[A]): Tuple = constValueTuple[m.MirroredElemLabels]

@main def run(): Unit =
  println(U.labels[eofa.Color])
  println(U.labels[eofa.Mixed])
  println(eofa.Color.values.toList)
  println(U.o(eofa.Color.Light))
  println(U.p(eofa.Mixed.C))
