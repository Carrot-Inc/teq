// jars: scala-library scalatest-core scalatest-compatible scalactic
// targets: jvm
//> using dep org.scalatest::scalatest-core::3.2.20
// ScalaTest's `assert` and scalactic's `requireNonNull` on the JVM (teq interp has no body of the JDK's
// `java.text.MessageFormat`, which their messages use): the bodies
// scalatest_assert runs on Scala.js, a library val (`Prettifier.default`) a macro's expansion
// reads before its class joins the program, a val implementing a def of one empty clause
// (`fullyQualifiedName`), and the members the expansions rebuild.
import org.scalatest.Assertions.*
import org.scalactic.Requirements.*

object Twice:
  def twice(n: Int): Int = n * 2

class Named(val name: String, val count: Option[Int]):
  requireNonNull(name, count)

def failing(body: => Unit): Unit =
  try body
  catch case e: org.scalatest.exceptions.TestFailedException => println(e.getMessage)

@main def run(): Unit =
  assert(Twice.twice(3) == 6)
  failing(assert(Twice.twice(3) == 7))
  val a: AnyRef = "x"
  val b: AnyRef = "y"
  failing(assert(a eq b))
  failing(assert(List(1, 2) == List(1, 3)))
  println(new Named("n", Some(1)).name)
  try new Named(null, None)
  catch case e: org.scalactic.exceptions.NullArgumentException => println(e.getMessage)
