// A non-native JS class's private member is the class's own method, which JavaScript does not see by its name, even
// where another member calls it. The expectation is scalac 3.8.4's under Scala.js 1.22.0 (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`).
import scala.scalajs.js
class Box(n: Int) extends js.Object:
  private def secret: Int = n + 7
  def uses: Int = secret * 2
@main def run(): Unit =
  val d = new Box(1).asInstanceOf[js.Dynamic]
  println(d.uses)
  println(js.isUndefined(d.secret))
