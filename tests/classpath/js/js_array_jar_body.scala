// jars: fixtures
// A class of a Scala.js jar keeping a stack in a `js.Array`: `new js.Array[A]()`, `push`, `pop`, `shift`
// and `length_=` from its bodies, with Scala.js's semantics.
import fix.facades.JsStack

@main def main(): Unit =
  val s = JsStack[Int]()
  s.push(1)
  s.push(2)
  s.push(3)
  println(s.pop())
  println(s.dropFirst())
  println(s.size)
  s.push(4)
  s.push(5)
  println(s.drained())
  s.push(6)
  s.clear()
  println(s.size)
