// jars: scala-library scalajs-dom
// With scalajs-dom on the class path its facades are `org.scalajs.dom` and the std's DOM shapes
// stay out of the build: the jar's `HTMLDialogElement.open` is a val and its `Headers` has no
// `getSetCookie`, scalac's two errors. tests/interop/dom_std_shapes.scala is the same program
// without the jar, which the std's shapes accept.
// expect: value getSetCookie is not a member of Headers
// expect: reassignment to val open
import org.scalajs.dom
import org.scalajs.dom.{Headers, HTMLDialogElement}

def page(): Unit =
  val h = new Headers()
  println(h.getSetCookie())
  val d = dom.document.createElement("dialog").asInstanceOf[HTMLDialogElement]
  d.open = true

@main def main(): Unit = println(dom.document.title)
