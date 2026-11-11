// A program JS class's method with a repeated parameter, called with three arguments: teq writes the class's member
// as a method taking one `Seq`, so the call passes one (a call lowered by JavaScript's rule passed `1` as `xs`). The
// expectation is scalac 3.8.4's under Scala.js 1.22.0 (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`).
import scala.scalajs.js
class C extends js.Object:
  def count(xs: Int*): Int = xs.length
@main def run(): Unit =
  println(new C().count(1, 2, 3))
