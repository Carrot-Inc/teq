// jars: scalatest-core-sjs scalactic-sjs scala-xml-sjs
//> using platform js
//> using jsVersion 1.22.0
//> using dep org.scalatest::scalatest-core::3.2.20
// ScalaTest's `assert` on Scala.js, its macro expanded and its messages made by scalactic's bodies:
// comparisons of primitives and references, a JavaScript member the macro selects again, and
// `requireNonNull`, whose macro reads its vararg as a sequence literal.
import org.scalatest.Assertions.*
import org.scalactic.Requirements.*
import scala.scalajs.js

@js.native
trait Listy extends js.Object:
  def length: Int = js.native

object Twice:
  def twice(n: Int): Int = n * 2

class Named(val name: String, val count: Option[Int]):
  requireNonNull(name, count)

def failing(body: => Unit): Unit =
  try body
  catch case e: org.scalatest.exceptions.TestFailedException => println(e.getMessage)

@main def run(): Unit =
  assert(Twice.twice(3) == 6)
  val l = js.Array(1, 2, 3).asInstanceOf[Listy]
  assert(l.length == 3)
  failing(assert(Twice.twice(3) == 7))
  failing(assert(l.length > 4))
  val a: AnyRef = "x"
  val b: AnyRef = new String("y")
  failing(assert(a eq b))
  failing(assert(List(1, 2) == List(1, 3)))
  println(new Named("n", Some(1)).name)
  try new Named(null, None)
  catch case e: org.scalactic.exceptions.NullArgumentException => println(e.getMessage)
