// jars: scala-library fixtures fixtures-rdecl
// std: lean scala-library
// Library bodies over the standard library under both std modes: calls whose
// `ClassTag` and `<:<` evidence the lean std's members do without, a wildcard captured as
// `TypeBox[Nothing, Any]#CAP`, and an `IArray` extension whose jar body names `arr.T` with its
// class (`TYPEREFin`). `RdCursor` is `tests/classpath/pending/reader_nn_path.scala`'s.
import fix.reader.*

@main def main(): Unit =
  println(RdCalls.arr(List(1, 2, 3)).toList)
  println(RdCalls.unz(List(1 -> "a", 2 -> "b")))
  println(RdCalls.capturedArray(Array("x", "y")) + " " + RdCalls.capturedArray(Array[Int]()))
  println(RdCalls.firstOfIArray(IArray(4, 5)))
