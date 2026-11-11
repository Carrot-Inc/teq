// The sum mirror of an enum read back from its pickle: its elements are the pickle's Child annotations, each once, as
// TreeUnpickler registers no child, so the downstream prints scalac 3.8.4's `(Dark,Light)` and 2 over teq's products,
// on the JVM, on JavaScript and in the interpreter, and over scalac's on the JVM (over-scalac.txt). Integration listed
// every case twice from e835f35451 on, `(Dark,Dark,Light,Light)` and 4, the TASTy loader registering each case again
// beside its pickled Child (a TR regression; master printed scalac's output). Found by the independent reviewer of the
// workers exports helper's push 10 (its probe mir/sb2).
package esb

import scala.deriving.Mirror
import scala.compiletime.{constValue, constValueTuple}

object U:
  inline def labels[A](using m: Mirror.SumOf[A]): Tuple = constValueTuple[m.MirroredElemLabels]
  inline def size[A](using m: Mirror.SumOf[A]): Int = constValue[Tuple.Size[m.MirroredElemTypes]]

@main def run(): Unit =
  println(U.labels[esa.Shade])
  println(U.size[esa.Shade])
