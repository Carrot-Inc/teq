// Without scalajs-dom on the class path `org.scalajs.dom` is the std's DOM layer, whose shapes
// differ from the jar's where the application needed them to: a writable
// `HTMLDialogElement.open` and `Headers.getSetCookie` (tests/classpath/dom_jar_shapes.scala has
// the jar's errors for the same program).
import org.scalajs.dom
import org.scalajs.dom.{Headers, HTMLDialogElement}

def page(): Unit =
  val h = new Headers()
  println(h.getSetCookie())
  val d = dom.document.createElement("dialog").asInstanceOf[HTMLDialogElement]
  d.open = true

@main def main(): Unit = println(dom.document.title)
