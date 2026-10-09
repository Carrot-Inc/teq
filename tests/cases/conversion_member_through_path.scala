import scala.language.implicitConversions

// Conversions compared by the member the selection expects see the member's result through the
// path they convert to: `a.foo` is an `a.Out`, which `a`'s type fixes as `String`.
trait T:
  type Out
  def foo: Out

object Defs:
  val a: T { type Out = String } = new T { type Out = String; def foo = "a" }
  val b: T { type Out = Int } = new T { type Out = Int; def foo = 1 }
  implicit def ca(x: Boolean): a.type = a
  implicit def cb(x: Boolean): b.type = b

import Defs.*

@main def main(): Unit =
  val s: String = true.foo
  val n: Int = false.foo
  println(s + " " + n)
