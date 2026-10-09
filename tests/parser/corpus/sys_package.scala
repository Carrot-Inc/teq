// `scala.sys` is a package, as in scala-library: `sys.error`, `scala.sys.error` and an import of it.
import scala.sys as s

@main def run(): Unit =
  def attempt(f: => Nothing): String = try f catch case e: RuntimeException => e.getMessage
  println(attempt(sys.error("one")))
  println(attempt(scala.sys.error("two")))
  println(attempt(s.error("three")))
