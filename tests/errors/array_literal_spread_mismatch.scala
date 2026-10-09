// expect: 6:11: error: None of the overloaded alternatives of method apply in object Array
// expect: 1 error found
// A spread's elements are not widened: `Seq(2)` is no `Seq[Long]`, so `Array(1L, Seq(2)*)` fits
// no overload, as under scalac (E134).
@main def run(): Unit =
  println(Array(1L, Seq(2)*).mkString(","))
