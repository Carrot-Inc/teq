package fix.facades

import scala.scalajs.js

// A stack over Scala.js's js.Array, as a runtime library keeps one: `new js.Array[A]()`, `push`,
// `pop`, `shift` and `length_=` from the jar's bodies.
final class JsStack[A]:
  private val items = new js.Array[A]()
  def push(a: A): Unit = items.push(a)
  def pop(): A = items.pop()
  def dropFirst(): A = items.shift()
  def size: Int = items.length
  def clear(): Unit = items.length = 0
  def drained(): String =
    val out = new js.Array[String]()
    while items.length > 0 do out.push(items.pop().toString)
    out.join(",")
