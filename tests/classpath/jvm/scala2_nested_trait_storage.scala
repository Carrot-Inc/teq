// jars: scala-library scala2-lib
// A Scala 2.13 trait nested in an object, whose `ScalaSignature` is its top-level class's, gives the class mixing it
// in its private var, its object and its private lazy val from the class file as a top-level trait does, and its
// `$init$` runs; a private lazy val computed to null is computed once (scala-library's `LazyVals$NullValue$` marks
// it). The nested trait was once left out (`AbstractMethodError`) and the null recomputed.
import scala2lib.*

class N extends Box.Nested
class L extends NullLazy

@main def run(): Unit =
  val n = N()
  println(n.next + n.next)
  println((n.cell eq n.cell, n.cell.owner eq n))
  val l = L()
  println(l.read)
  println(l.read)
