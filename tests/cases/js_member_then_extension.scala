//> using platform js
//> using dep org.scala-js::scalajs-dom::2.8.1
// jars: scalajs-dom
// A member that is a value applied to arguments it does not take, beside an extension of its name that
// takes them (indigo's `e.width(magnification)` on a `dom.PointerEvent`): the extension applies, as dotty
// retries any failed application of a selection on the qualifier (`tryWithImplicitOnQualifier`).
import scala.scalajs.js
import org.scalajs.dom

object W:
  extension (e: dom.PointerEvent)
    def width(m: Int): Int = (e.width * m).toInt
  def f(e: dom.PointerEvent): Int = e.width(2)

@main def run(): Unit =
  val e = js.Dynamic.literal(width = 3.0).asInstanceOf[dom.PointerEvent]
  println(W.f(e))
  println(e.width)
