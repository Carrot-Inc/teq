//> using platform js
// Scala.js's `js.Array.from` over a `js.Iterable[A]`, which infers the element type, and `jsSlice`
// with the reference's named parameters and their defaults.
import scala.scalajs.js

@main def main(): Unit =
  val a = js.Array.from(js.Array(1, 2))
  println(a(0) + 1)
  println(js.Array(1, 2, 3).jsSlice(end = 1).length)
  println(js.Array(1, 2, 3).jsSlice(1).length)
  println(js.Array(1, 2, 3).jsSlice().length)
  println(js.Array(1, 2, 3).jsSlice(start = 1, end = 2)(0))
