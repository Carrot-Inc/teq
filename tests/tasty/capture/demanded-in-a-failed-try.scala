package capture.demanded

import scala.language.implicitConversions

// Members whose result is inferred, demanded first inside a conversion's try that fails on its
// using clause: the body typed for the result is the member's, and so are its records.

trait Ord[A]

object Syntax:
  implicit def toOps[A](a: A)(implicit o: Ord[A]): Ops[A] = new Ops(a)

object Far:
  implicit def toOther(a: Int): Other = new Other(a)

class Other(a: Int):
  def foo(b: Int): Int = a * 10 + b
  def bar: Int = a + 1

object Main:
  import Syntax.*
  import Far.*
  def main(args: Array[String]): Unit =
    println(3.foo(4))
    println(3.bar)
    println(3.min(4))
    println(2.5.max(1.5))

class Ops[A](a: A):
  def foo(b: A) = List(a, b).head
  def bar = Seq(a).length
  def min(b: A) = Seq(a, b).head
  def max(b: A) = s"$a$b".length
