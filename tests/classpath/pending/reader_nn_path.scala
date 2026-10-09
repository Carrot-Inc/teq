// jars: scala-library fixtures fixtures-rdecl
// std: lean scala-library
// A library class's val passed to `Predef.nn` (`RdCursor.size` of tests/tasty/src/reader.scala,
// `nn[Array[Int]](RdCursor.this.stack).length`, the shape of scala-library's
// `RedBlackTree.TreeIterator.pushNext`). Under the lean std the output reads the field `stack`
// through the expansion's binding and never gets the val's getter, which a library val is,
// so the run meets `undefined.length`; under `--std=scala-library` the
// expansion of scala-library's `nn` does not type, "found (x : (stack : Array[Int] | Null)) &
// Array[Int], required (RdCursor.this.stack : Array[Int] | Null) & Array[Int]", as it does not
// for the same class in source. The expectation is scalac's.
import fix.reader.*

@main def main(): Unit =
  println(RdCursor(3).size + " " + RdCursor(0).size)
