//> using platform js
// Scala.js's `js.Array` members Scala's `Array` lacks: `new js.Array[A]()` is an empty array,
// `reverseInPlace()` reverses the array itself and returns it (Scala's `reverse` copies), and
// `sort()` sorts in place, as strings without a comparison function.
import scala.scalajs.js

@main def run(): Unit =
  val a = new js.Array[Int]()
  a.push(3)
  a.push(1)
  a.push(20)
  println(a.length)
  val r = a.reverseInPlace()
  println(a.join(","))
  println(r eq a)
  val copy = a.reverse
  println(copy eq a)
  println(a.sort().join(","))
  println(a.sort((x, y) => x - y).join(","))
  val s = js.Array("b", "a", "c")
  s.sort()
  println(s.join(""))
