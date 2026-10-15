// A typed array takes Scala.js's constructors: a length, another of its kind, a JS iterable of its elements or
// a buffer with an offset and a length; a `DataView` is none of them, as scalac rejects it.
// expect: 7:11: error: None of the overloaded alternatives of constructor Int16Array in class Int16Array with types
import scala.scalajs.js.typedarray.*
@main def main(): Unit =
  println(new Int16Array(4).length)
  val a = new Int16Array(new DataView(new ArrayBuffer(4)))
  println(a.length)
