//> using platform js
// Scala.js's js.Array mutations on the JavaScript array.
import scala.scalajs.js

@main def main(): Unit =
  val a = js.Array[Int]()
  a.push(1)
  a.push(2)
  a.push(3)
  println(a.pop())
  a.push(4)
  println(a.shift())
  println(a.unshift(7, 8))
  println(a.join(","))
  val removed = a.splice(1, 2, 20, 30, 40)
  println(removed.join("-"))
  println(a.join())
  a.length = 2
  println(a.length)
  println(a.join(" "))
  println(a.jsSlice(1).join())
  println(a.jsSlice(0, 1).join())
  println(a.concat(js.Array(5, 6), js.Array(9)).join())
  println(js.Array(1, 2, 1).lastIndexOf(1))
  println(a.indexOf(20))
